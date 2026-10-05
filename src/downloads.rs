use crate::library::Recording;
use anyhow::{Context, Result, bail};
use axum::{
    body::{Body, Bytes},
    http::header,
    response::{IntoResponse, Response},
};
use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, sync::Semaphore};

#[derive(Clone)]
pub(crate) struct Downloads {
    media: PathBuf,
    ffmpeg: String,
    slots: Arc<Semaphore>,
}

impl Downloads {
    pub fn new(media: PathBuf, ffmpeg: String) -> Self {
        Self {
            media,
            ffmpeg,
            slots: Arc::new(Semaphore::new(2)),
        }
    }

    pub async fn open(&self, recording: &Recording) -> Result<Response> {
        if !recording.playable {
            bail!("No complete footage is available to download yet");
        }
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .context("Two downloads are already running. Try again after one finishes")?;
        let root = tokio::fs::canonicalize(self.media.join(recording.id.to_string())).await?;
        // Publication replaces this file atomically. Read it once so new footage
        // cannot extend the export or make FFmpeg wait for the live stream.
        let playlist = tokio::fs::read_to_string(root.join("index.m3u8")).await?;
        let snapshot = snapshot_playlist(&playlist, &root)?;
        let mut input = tempfile::Builder::new().suffix(".m3u8").tempfile()?;
        input.write_all(snapshot.as_bytes())?;
        input.flush()?;
        let mut log = tempfile::tempfile()?;
        let mut child = Command::new(&self.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-protocol_whitelist",
                "file",
                "-i",
            ])
            .arg(input.path())
            .args([
                "-map",
                "0:v:0",
                "-map",
                "0:a:0?",
                "-c",
                "copy",
                "-bsf:a",
                "aac_adtstoasc",
                "-movflags",
                "frag_keyframe+empty_moov+default_base_moof",
                "-f",
                "mp4",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log.try_clone()?))
            .kill_on_drop(true)
            .spawn()
            .context("Could not start the video download")?;
        let mut stdout = child.stdout.take().context("Missing download output")?;
        let mut buffer = vec![0; 64 * 1024];
        let first = tokio::time::timeout(Duration::from_secs(30), stdout.read(&mut buffer))
            .await
            .context("Video download did not start within 30 seconds")??;
        if first == 0 {
            let status = tokio::time::timeout(Duration::from_secs(30), child.wait()).await??;
            log_failure(&mut log, status);
            bail!("Could not export this recording as MP4");
        }
        let first = Bytes::copy_from_slice(&buffer[..first]);
        let filename = download_name(recording);
        let stream = async_stream::stream! {
            // These guards live until the response completes or the browser
            // disconnects. Dropping Child terminates an abandoned export.
            let _permit = permit;
            let _input = input;
            let mut log = log;
            yield Ok::<_, io::Error>(first);
            loop {
                let read = tokio::time::timeout(Duration::from_secs(60), stdout.read(&mut buffer))
                    .await.unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::TimedOut, "Video download stalled")));
                match read {
                    Ok(0) => break,
                    Ok(count) => yield Ok(Bytes::copy_from_slice(&buffer[..count])),
                    Err(error) => { yield Err(error); return; }
                }
            }
            let status = tokio::time::timeout(Duration::from_secs(30), child.wait())
                .await.unwrap_or_else(|_| Err(io::Error::new(io::ErrorKind::TimedOut, "Video export did not finish")));
            match status {
                Ok(status) if status.success() => {},
                Ok(status) => {
                    log_failure(&mut log, status);
                    yield Err(io::Error::other("Video export failed"));
                }
                Err(error) => yield Err(error),
            }
        };
        // A fragmented MP4 can be sent directly to the browser without keeping
        // another full recording on disk or buffering the video in RAM.
        let body = Body::from_stream(stream);
        Ok((
            [
                (header::CONTENT_TYPE, "video/mp4".to_owned()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{filename}\""),
                ),
            ],
            body,
        )
            .into_response())
    }
}

fn log_failure(log: &mut std::fs::File, status: std::process::ExitStatus) {
    let mut detail = String::new();
    let _ = log.seek(SeekFrom::Start(0));
    let _ = log.take(4096).read_to_string(&mut detail);
    tracing::warn!(%status, %detail, "Video export failed");
}

fn snapshot_playlist(playlist: &str, root: &Path) -> Result<String> {
    let root = root
        .to_str()
        .context("Recording path must be valid UTF-8")?;
    if root.contains(['\n', '\r']) {
        bail!("Invalid recording path");
    }
    let mut snapshot = String::new();
    let mut segments = 0;
    for line in playlist.lines() {
        if line == "#EXT-X-ENDLIST" {
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            snapshot.push_str(line);
        } else {
            // Only local, completed capture segments may enter an export.
            let (attempt, segment) = line.split_once('/').context("Invalid segment path")?;
            let attempt = attempt
                .strip_prefix("attempt-")
                .context("Invalid capture attempt")?;
            let segment = segment
                .strip_prefix("segment")
                .and_then(|name| name.strip_suffix(".ts"))
                .context("Invalid capture segment")?;
            if attempt.is_empty()
                || segment.is_empty()
                || !attempt.bytes().all(|byte| byte.is_ascii_digit())
                || !segment.bytes().all(|byte| byte.is_ascii_digit())
            {
                bail!("Invalid segment path");
            }
            snapshot.push_str(&format!("{root}/{line}"));
            segments += 1;
        }
        snapshot.push('\n');
    }
    if segments == 0 {
        bail!("No complete footage is available to download yet");
    }
    snapshot.push_str("#EXT-X-ENDLIST\n");
    Ok(snapshot)
}

fn download_name(recording: &Recording) -> String {
    let title: String = recording
        .title
        .chars()
        .take(80)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    let title = title.trim_matches('-');
    let title = if title.is_empty() { "recording" } else { title };
    let partial = if recording.phase.active() {
        "-so-far"
    } else {
        ""
    };
    format!("{title}-{}{partial}.mp4", recording.started_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freezes_live_playlist_and_preserves_attempt_boundaries() {
        let playlist = "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXTINF:4,\nattempt-000001/segment00000001.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:4,\nattempt-000002/segment00000001.ts\n";
        let snapshot = snapshot_playlist(playlist, Path::new("/recordings/test")).unwrap();
        assert!(snapshot.contains("/recordings/test/attempt-000001/segment00000001.ts"));
        assert!(snapshot.contains("#EXT-X-DISCONTINUITY\n"));
        assert!(snapshot.ends_with("#EXT-X-ENDLIST\n"));
        let finished = snapshot_playlist(
            &format!("{playlist}#EXT-X-ENDLIST\n"),
            Path::new("/recordings/test"),
        )
        .unwrap();
        assert_eq!(snapshot, finished);
    }

    #[test]
    fn rejects_empty_playlists_and_non_capture_paths() {
        for path in [
            "../private.ts",
            "attempt-1/../../private.ts",
            "https://example.com/video.ts",
            "attempt-1/segment1.ts?x",
            "attempt-/segment1.ts",
        ] {
            assert!(
                snapshot_playlist(
                    &format!("#EXTM3U\n#EXTINF:4,\n{path}\n"),
                    Path::new("/recordings/test")
                )
                .is_err()
            );
        }
        assert!(snapshot_playlist("#EXTM3U\n", Path::new("/recordings/test")).is_err());
    }
}
