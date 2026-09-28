use crate::rooms::{Room, RoomId};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordingId(pub Uuid);

impl std::fmt::Display for RecordingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Waiting,
    Recording,
    Retrying,
    Finished,
    Stopped,
    Failed,
    NoStream,
}

impl Phase {
    pub fn active(self) -> bool {
        matches!(self, Self::Waiting | Self::Recording | Self::Retrying)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: RecordingId,
    pub room_id: RoomId,
    pub title: String,
    pub source_url: String,
    pub started_at: i64,
    pub ends_at: i64,
    pub phase: Phase,
    pub incomplete: bool,
    pub playable: bool,
    pub duration: f64,
    pub message: Option<String>,
}

pub struct Library {
    db: Connection,
    pub media: PathBuf,
}

impl Library {
    pub fn open(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        let db = Connection::open(root.join("dvr.sqlite"))?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS rooms (id TEXT PRIMARY KEY, document TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS recordings (id TEXT PRIMARY KEY, document TEXT NOT NULL);
             PRAGMA user_version=1;",
        )?;
        let media = root.join("recordings");
        std::fs::create_dir_all(&media)?;
        let library = Self { db, media };
        let saved = library.rooms()?;
        for room in crate::rooms::fi_defaults() {
            if !saved.iter().any(|existing| {
                existing.page_url == room.page_url || existing.stream_url == room.stream_url
            }) {
                library.save_room(&room)?;
            }
        }
        Ok(library)
    }

    pub fn rooms(&self) -> Result<Vec<Room>> {
        let mut statement = self
            .db
            .prepare("SELECT document FROM rooms ORDER BY rowid")?;
        let documents = statement.query_map([], |row| row.get::<_, String>(0))?;
        documents
            .map(|doc| Ok(serde_json::from_str(&doc?)?))
            .collect()
    }

    pub fn save_room(&self, room: &Room) -> Result<()> {
        self.db.execute(
            "INSERT INTO rooms VALUES (?1, ?2)",
            params![room.id.0.to_string(), serde_json::to_string(room)?],
        )?;
        Ok(())
    }

    pub fn recordings(&self) -> Result<Vec<Recording>> {
        let mut statement = self
            .db
            .prepare("SELECT document FROM recordings ORDER BY rowid DESC")?;
        let documents = statement.query_map([], |row| row.get::<_, String>(0))?;
        documents
            .map(|doc| Ok(serde_json::from_str(&doc?)?))
            .collect()
    }

    pub fn save_recording(&self, recording: &Recording) -> Result<()> {
        self.db.execute("INSERT INTO recordings VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET document=excluded.document", params![recording.id.to_string(), serde_json::to_string(recording)?])?;
        Ok(())
    }

    pub fn directory(&self, id: RecordingId) -> PathBuf {
        self.media.join(id.to_string())
    }

    // FFmpeg publishes each attempt playlist atomically. Only complete segments
    // listed in those playlists are exposed in the recording-level playlist.
    pub fn publish(&self, recording: &mut Recording, finished: bool) -> Result<()> {
        let root = self.directory(recording.id);
        std::fs::create_dir_all(&root)?;
        let mut attempts: Vec<_> = std::fs::read_dir(&root)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("attempt-"))
            .collect();
        attempts.sort_by_key(|entry| entry.file_name());
        let mut body = String::new();
        let mut duration = 0.0;
        let mut target = 1.0_f64;
        let mut has_segments = false;
        for attempt in attempts {
            let Ok(playlist) = std::fs::read_to_string(attempt.path().join("index.m3u8")) else {
                continue;
            };
            let segments = parse_segments(&playlist)?;
            if segments.is_empty() {
                continue;
            }
            if has_segments {
                body.push_str("#EXT-X-DISCONTINUITY\n");
            }
            for (length, name) in segments {
                target = target.max(length.ceil());
                duration += length;
                body.push_str(&format!(
                    "#EXTINF:{length:.6},\n{}/{name}\n",
                    attempt.file_name().to_string_lossy()
                ));
            }
            has_segments = true;
        }
        recording.duration = duration;
        recording.playable = has_segments;
        if has_segments {
            let mut playlist = format!(
                "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:{target:.0}\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:EVENT\n{body}"
            );
            if finished {
                playlist.push_str("#EXT-X-ENDLIST\n");
            }
            let temporary = root.join("index.m3u8.tmp");
            std::fs::write(&temporary, playlist)?;
            std::fs::rename(temporary, root.join("index.m3u8"))?;
        }
        Ok(())
    }
}

fn parse_segments(playlist: &str) -> Result<Vec<(f64, String)>> {
    let mut pending = None;
    let mut segments = Vec::new();
    for line in playlist.lines() {
        if let Some(value) = line.strip_prefix("#EXTINF:") {
            let duration: f64 = value
                .split(',')
                .next()
                .context("Missing segment duration")?
                .parse()?;
            if !duration.is_finite() || duration <= 0.0 {
                bail!("Invalid segment duration");
            }
            pending = Some(duration);
        } else if !line.starts_with('#') && !line.is_empty() {
            if line.contains('/')
                || line.contains('\\')
                || !line.ends_with(".ts")
                || line.contains("..")
            {
                bail!("Unexpected segment name");
            }
            segments.push((pending.take().context("Missing EXTINF")?, line.to_owned()));
        }
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_existing_rooms_and_do_not_duplicate_on_restart() {
        let temp = tempfile::tempdir().unwrap();
        let library = Library::open(temp.path()).unwrap();
        let rooms = library.rooms().unwrap();
        assert_eq!(rooms.len(), 5);
        assert!(rooms.iter().all(|room| room.name != "FI S108"));
        let mut existing = rooms[2].clone();
        existing.name = "My lecture room".into();
        // Simulate a user-saved direct playlist from before defaults existed.
        existing.page_url = existing.stream_url.clone();
        library.db.execute("DELETE FROM rooms", []).unwrap();
        library.save_room(&existing).unwrap();
        let custom = Room {
            id: RoomId(Uuid::new_v4()),
            name: "Another room".into(),
            page_url: "https://live.cesnet.cz/another.html".into(),
            stream_url: "https://cdn.streaming.cesnet.cz/other/playlist.m3u8".into(),
        };
        library.save_room(&custom).unwrap();
        drop(library);
        let library = Library::open(temp.path()).unwrap();
        let reopened = library.rooms().unwrap();
        assert_eq!(reopened.len(), 6);
        assert!(
            reopened
                .iter()
                .any(|room| room.id == existing.id && room.name == existing.name)
        );
        assert!(reopened.iter().any(|room| room.id == custom.id));
        let ids: Vec<_> = reopened.iter().map(|room| room.id).collect();
        drop(library);
        let library = Library::open(temp.path()).unwrap();
        assert_eq!(
            library
                .rooms()
                .unwrap()
                .iter()
                .map(|room| room.id)
                .collect::<Vec<_>>(),
            ids
        );
    }

    #[test]
    fn publishes_attempts_in_order_with_discontinuities_and_finalization() {
        let temp = tempfile::tempdir().unwrap();
        let library = Library::open(temp.path()).unwrap();
        let mut rec = Recording {
            id: RecordingId(Uuid::new_v4()),
            room_id: RoomId(Uuid::new_v4()),
            title: "test".into(),
            source_url: "test".into(),
            started_at: 0,
            ends_at: 10,
            phase: Phase::Recording,
            incomplete: true,
            playable: false,
            duration: 0.0,
            message: None,
        };
        for attempt in ["attempt-000001", "attempt-000002"] {
            let path = library.directory(rec.id).join(attempt);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join("index.m3u8"),
                "#EXTM3U\n#EXTINF:4.2,\nsegment000000.ts\n",
            )
            .unwrap();
        }
        library.publish(&mut rec, false).unwrap();
        assert_eq!(rec.duration, 8.4);
        let content =
            std::fs::read_to_string(library.directory(rec.id).join("index.m3u8")).unwrap();
        assert!(content.contains("#EXT-X-TARGETDURATION:5"));
        assert_eq!(content.matches("#EXT-X-DISCONTINUITY").count(), 1);
        assert!(!content.contains("#EXT-X-ENDLIST"));
        library.publish(&mut rec, true).unwrap();
        assert!(
            std::fs::read_to_string(library.directory(rec.id).join("index.m3u8"))
                .unwrap()
                .ends_with("#EXT-X-ENDLIST\n")
        );
    }
}
