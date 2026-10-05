use crate::rooms::{Room, RoomId};
use crate::subjects::{Subject, SubjectId};
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
    #[serde(default)]
    pub room_name: String,
    #[serde(default)]
    pub subject_id: Option<SubjectId>,
    pub source_url: String,
    pub started_at: i64,
    #[serde(default)]
    pub scheduled_start: Option<i64>,
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
             CREATE TABLE IF NOT EXISTS subjects (id TEXT PRIMARY KEY, document TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS removed_room_sources (url TEXT PRIMARY KEY);
             PRAGMA user_version=2;",
        )?;
        let media = root.join("recordings");
        std::fs::create_dir_all(&media)?;
        let library = Self { db, media };
        let saved = library.rooms()?;
        for room in crate::rooms::fi_defaults() {
            let removed: bool = library.db.query_row(
                "SELECT EXISTS(SELECT 1 FROM removed_room_sources WHERE url IN (?1, ?2))",
                params![room.page_url, room.stream_url],
                |row| row.get(0),
            )?;
            if !removed
                && !saved.iter().any(|existing| {
                    existing.page_url == room.page_url || existing.stream_url == room.stream_url
                })
            {
                library.save_room(&room)?;
            }
        }
        for subject in crate::subjects::defaults() {
            library.db.execute(
                "INSERT OR IGNORE INTO subjects VALUES (?1, ?2)",
                params![subject.id.0, serde_json::to_string(&subject)?],
            )?;
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

    pub fn remove_room(&mut self, id: RoomId) -> Result<()> {
        let room = self
            .rooms()?
            .into_iter()
            .find(|room| room.id == id)
            .context("Room not found")?;
        if self
            .recordings()?
            .iter()
            .any(|recording| recording.room_id == id && recording.phase.active())
        {
            bail!("Stop the active recording before removing this room");
        }
        let transaction = self.db.transaction()?;
        for url in [&room.page_url, &room.stream_url] {
            transaction.execute(
                "INSERT OR IGNORE INTO removed_room_sources VALUES (?1)",
                [url],
            )?;
        }
        transaction.execute("DELETE FROM rooms WHERE id = ?1", [id.0.to_string()])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn subjects(&self) -> Result<Vec<Subject>> {
        let mut statement = self
            .db
            .prepare("SELECT document FROM subjects ORDER BY rowid")?;
        let documents = statement.query_map([], |row| row.get::<_, String>(0))?;
        documents
            .map(|doc| Ok(serde_json::from_str(&doc?)?))
            .collect()
    }

    pub fn assign_subject(&self, recording: &mut Recording, id: Option<SubjectId>) -> Result<()> {
        let subject = id
            .as_ref()
            .map(|id| {
                self.subjects()?
                    .into_iter()
                    .find(|subject| &subject.id == id)
                    .context("Subject not found")
            })
            .transpose()?;
        if recording.room_name.is_empty() {
            recording.room_name = recording.title.clone();
        }
        recording.title = subject
            .map(|subject| format!("{} · {}", subject.id.0, subject.name))
            .unwrap_or_else(|| recording.room_name.clone());
        recording.subject_id = id;
        self.save_recording(recording)
    }

    pub fn recordings(&self) -> Result<Vec<Recording>> {
        let mut statement = self
            .db
            .prepare("SELECT document FROM recordings ORDER BY rowid DESC")?;
        let documents = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut recordings: Vec<Recording> = documents
            .map(|doc| Ok(serde_json::from_str(&doc?)?))
            .collect::<Result<_>>()?;
        recordings.sort_by_key(|recording| std::cmp::Reverse(recording.started_at));
        Ok(recordings)
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
            if !duration.is_finite() || duration < 0.0 {
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
            let duration = pending.take().context("Missing EXTINF")?;
            // FFmpeg can publish an empty final fragment when capture is
            // interrupted. It contains no playback time and must not abort
            // the entire recording or prevent subsequent retries.
            if duration > 0.0 {
                segments.push((duration, line.to_owned()));
            }
        }
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_defaults_stay_removed_without_losing_subjects_or_recordings() {
        let temp = tempfile::tempdir().unwrap();
        let mut library = Library::open(temp.path()).unwrap();
        let room = library.rooms().unwrap().remove(0);
        // This is the old on-disk shape: no subject_id or room_name.
        let mut recording: Recording = serde_json::from_value(serde_json::json!({
            "id": Uuid::new_v4(), "room_id": room.id, "title": room.name,
            "source_url": room.stream_url, "started_at": 100, "ends_at": 200,
            "phase": "recording", "incomplete": false, "playable": true,
            "duration": 20.0, "message": null
        }))
        .unwrap();
        library.save_recording(&recording).unwrap();
        assert!(library.remove_room(room.id).is_err());
        recording.phase = Phase::Stopped;
        library
            .assign_subject(&mut recording, Some(SubjectId("PV017".into())))
            .unwrap();
        let directory = library.directory(recording.id);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("retained.ts"), b"recording").unwrap();
        library.remove_room(room.id).unwrap();
        assert!(
            library
                .assign_subject(&mut recording, Some(SubjectId("unknown".into())))
                .is_err()
        );
        drop(library);
        let library = Library::open(temp.path()).unwrap();
        assert_eq!(library.rooms().unwrap().len(), 4);
        assert!(
            !library
                .rooms()
                .unwrap()
                .iter()
                .any(|saved| saved.stream_url == room.stream_url)
        );
        assert_eq!(library.subjects().unwrap().len(), 9);
        let saved = &library.recordings().unwrap()[0];
        assert_eq!(saved.subject_id, Some(SubjectId("PV017".into())));
        assert_eq!(saved.room_name, room.name);
        assert!(directory.join("retained.ts").exists());
    }

    #[test]
    fn adds_monday_subjects_to_existing_library_once() {
        let temp = tempfile::tempdir().unwrap();
        let library = Library::open(temp.path()).unwrap();
        library
            .db
            .execute("DELETE FROM subjects WHERE id IN ('PV157', 'PV281')", [])
            .unwrap();
        assert_eq!(library.subjects().unwrap().len(), 7);
        drop(library);
        for _ in 0..2 {
            let library = Library::open(temp.path()).unwrap();
            let subjects = library.subjects().unwrap();
            assert_eq!(subjects.len(), 9);
            let windows = crate::schedule::open_windows(&subjects, 1791180000);
            assert_eq!(windows.len(), 1);
            assert_eq!(windows[0].subject_id.0, "PV157");
        }
    }

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
            room_name: "test".into(),
            subject_id: None,
            source_url: "test".into(),
            started_at: 0,
            scheduled_start: None,
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
                "#EXTM3U\n#EXTINF:4.2,\nsegment000000.ts\n#EXTINF:0.000000,\nsegment000001.ts\n#EXT-X-ENDLIST\n",
            )
            .unwrap();
        }
        library.publish(&mut rec, false).unwrap();
        assert_eq!(rec.duration, 8.4);
        let empty_attempt = library.directory(rec.id).join("attempt-000003");
        std::fs::create_dir_all(&empty_attempt).unwrap();
        std::fs::write(
            empty_attempt.join("index.m3u8"),
            "#EXTM3U\n#EXTINF:0.000000,\nsegment000000.ts\n#EXT-X-ENDLIST\n",
        )
        .unwrap();
        let content =
            std::fs::read_to_string(library.directory(rec.id).join("index.m3u8")).unwrap();
        assert!(content.contains("#EXT-X-TARGETDURATION:5"));
        assert_eq!(content.matches("#EXT-X-DISCONTINUITY").count(), 1);
        assert!(!content.contains("#EXT-X-ENDLIST"));
        assert!(!content.contains("segment000001.ts"));
        library.publish(&mut rec, true).unwrap();
        assert!(
            std::fs::read_to_string(library.directory(rec.id).join("index.m3u8"))
                .unwrap()
                .ends_with("#EXT-X-ENDLIST\n")
        );
    }
}
