use crate::{
    library::{Library, Phase, Recording, RecordingId},
    rooms::{Room, RoomId},
    subjects::{Subject, SubjectId},
};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    path::PathBuf,
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, Command},
    sync::{mpsc, oneshot},
};
use uuid::Uuid;

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Serialize)]
pub struct Snapshot {
    pub rooms: Vec<Room>,
    pub subjects: Vec<Subject>,
    pub recordings: Vec<Recording>,
}

pub enum Action {
    Inspect,
    SaveRoom(Room),
    RemoveRoom(RoomId),
    AssignSubject {
        id: RecordingId,
        subject_id: Option<SubjectId>,
    },
    Start {
        room_id: RoomId,
        ends_at: i64,
        subject_id: Option<SubjectId>,
    },
    Stop(RecordingId),
    Extend {
        id: RecordingId,
        ends_at: i64,
    },
    Shutdown,
}

struct Request {
    action: Action,
    reply: oneshot::Sender<Result<Snapshot>>,
}

#[derive(Clone)]
pub struct Recorder {
    commands: mpsc::Sender<Request>,
}

impl Recorder {
    pub fn spawn(library: Library, ffmpeg: String) -> (Self, tokio::task::JoinHandle<Result<()>>) {
        let (commands, receiver) = mpsc::channel(32);
        let handle = tokio::spawn(
            Worker {
                library,
                ffmpeg,
                active: None,
            }
            .run(receiver),
        );
        (Self { commands }, handle)
    }

    pub async fn execute(&self, action: Action) -> Result<Snapshot> {
        let (reply, response) = oneshot::channel();
        self.commands
            .send(Request { action, reply })
            .await
            .context("Recorder has shut down")?;
        response.await.context("Recorder stopped before replying")?
    }
}

struct Active {
    recording: Recording,
    child: Option<Child>,
    attempt: usize,
    retry_at: i64,
    last_progress_at: i64,
}

struct Worker {
    library: Library,
    ffmpeg: String,
    active: Option<Active>,
}

impl Worker {
    async fn run(mut self, mut receiver: mpsc::Receiver<Request>) -> Result<()> {
        self.recover()?;
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(error) = self.tick().await {
                        tracing::error!(%error, "Recorder tick failed");
                        if let Some(mut active) = self.active.take() {
                            halt(&mut active.child).await;
                            active.recording.phase = Phase::Failed;
                            active.recording.incomplete = true;
                            active.recording.message = Some(format!("Recording failed: {error}"));
                            let _ = self.library.publish(&mut active.recording, true);
                            self.library.save_recording(&active.recording)?;
                        }
                    }
                }
                request = receiver.recv() => {
                    let Some(request) = request else { self.shutdown().await?; break; };
                    let shutdown = matches!(request.action, Action::Shutdown);
                    let result = self.apply(request.action).await;
                    let _ = request.reply.send(result.and_then(|_| self.snapshot()));
                    if shutdown { break; }
                }
            }
        }
        Ok(())
    }

    fn snapshot(&self) -> Result<Snapshot> {
        Ok(Snapshot {
            rooms: self.library.rooms()?,
            subjects: self.library.subjects()?,
            recordings: self.library.recordings()?,
        })
    }

    fn recover(&mut self) -> Result<()> {
        for mut recording in self.library.recordings()? {
            if !recording.phase.active() {
                continue;
            }
            recording.incomplete = true;
            if recording.ends_at > now() && self.active.is_none() {
                recording.phase = Phase::Retrying;
                recording.message = Some("Resuming after restart".into());
                self.library.publish(&mut recording, false)?;
                self.library.save_recording(&recording)?;
                self.active = Some(Active {
                    recording,
                    child: None,
                    attempt: 0,
                    retry_at: now(),
                    last_progress_at: now(),
                });
            } else {
                self.library.publish(&mut recording, true)?;
                recording.phase = if recording.playable {
                    Phase::Finished
                } else {
                    Phase::NoStream
                };
                recording.message = Some("Recording window ended while the app was offline".into());
                self.library.save_recording(&recording)?;
            }
        }
        Ok(())
    }

    async fn apply(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Inspect => {}
            Action::SaveRoom(room) => self.library.save_room(&room)?,
            Action::RemoveRoom(id) => self.library.remove_room(id)?,
            Action::AssignSubject { id, subject_id } => {
                if let Some(active) = self
                    .active
                    .as_mut()
                    .filter(|active| active.recording.id == id)
                {
                    self.library
                        .assign_subject(&mut active.recording, subject_id)?;
                } else {
                    let mut recording = self
                        .library
                        .recordings()?
                        .into_iter()
                        .find(|recording| recording.id == id)
                        .context("Recording not found")?;
                    self.library.assign_subject(&mut recording, subject_id)?;
                }
            }
            Action::Start {
                room_id,
                ends_at,
                subject_id,
            } => {
                validate_end(ends_at)?;
                if self.active.is_some() {
                    bail!("Another recording is already active");
                }
                let room = self
                    .library
                    .rooms()?
                    .into_iter()
                    .find(|room| room.id == room_id)
                    .context("Room not found")?;
                let mut recording = Recording {
                    id: RecordingId(Uuid::new_v4()),
                    room_id,
                    title: room.name.clone(),
                    room_name: room.name,
                    subject_id: None,
                    source_url: room.stream_url,
                    started_at: now(),
                    ends_at,
                    phase: Phase::Waiting,
                    incomplete: false,
                    playable: false,
                    duration: 0.0,
                    message: Some("Waiting for the first complete segment".into()),
                };
                self.library.assign_subject(&mut recording, subject_id)?;
                self.active = Some(Active {
                    recording,
                    child: None,
                    attempt: 0,
                    retry_at: now(),
                    last_progress_at: now(),
                });
            }
            Action::Stop(id) => {
                if self.active.as_ref().map(|active| active.recording.id) != Some(id) {
                    bail!("Recording is not active");
                }
                // Persist the explicit stop before touching the child process so
                // a crash cannot turn a user stop into a resumed recording.
                let active = self.active.as_mut().unwrap();
                active.recording.phase = Phase::Stopped;
                self.library.save_recording(&active.recording)?;
                self.finish(Phase::Stopped).await?;
            }
            Action::Extend { id, ends_at } => {
                validate_end(ends_at)?;
                let active = self
                    .active
                    .as_mut()
                    .filter(|active| active.recording.id == id)
                    .context("Recording is not active")?;
                if ends_at <= active.recording.ends_at {
                    bail!("New end time must be later than the current end time");
                }
                active.recording.ends_at = ends_at;
                self.library.save_recording(&active.recording)?;
            }
            Action::Shutdown => self.shutdown().await?,
        }
        Ok(())
    }

    async fn finish(&mut self, phase: Phase) -> Result<()> {
        if let Some(mut active) = self.active.take() {
            halt(&mut active.child).await;
            self.library.publish(&mut active.recording, true)?;
            active.recording.phase = if active.recording.playable {
                phase
            } else {
                Phase::NoStream
            };
            active.recording.message = if active.recording.playable {
                None
            } else {
                Some("No stream available".into())
            };
            self.library.save_recording(&active.recording)?;
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        if let Some(active) = self.active.as_mut() {
            halt(&mut active.child).await;
            active.recording.phase = Phase::Retrying;
            active.recording.incomplete = true;
            active.recording.message =
                Some("App stopped; recording will resume on restart within its window".into());
            self.library.publish(&mut active.recording, false)?;
            self.library.save_recording(&active.recording)?;
        }
        Ok(())
    }

    async fn tick(&mut self) -> Result<()> {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.recording.ends_at <= now())
        {
            return self.finish(Phase::Finished).await;
        }
        let Some(active) = self.active.as_mut() else {
            return Ok(());
        };
        if let Some(child) = active.child.as_mut() {
            let exited = child.try_wait()?.is_some();
            let previous_duration = active.recording.duration;
            self.library.publish(&mut active.recording, false)?;
            if active.recording.duration > previous_duration {
                active.last_progress_at = now();
            }
            if exited || now() - active.last_progress_at > 45 {
                halt(&mut active.child).await;
                active.recording.phase = Phase::Retrying;
                active.recording.incomplete = true;
                active.recording.message =
                    Some("Source interrupted; retrying in 10 seconds".into());
                active.retry_at = now() + 10;
            } else if active.recording.playable {
                active.recording.phase = Phase::Recording;
                active.recording.message = None;
            }
            self.library.save_recording(&active.recording)?;
        }
        if active.child.is_none() && now() >= active.retry_at {
            let directory = next_attempt(
                &self.library.directory(active.recording.id),
                &mut active.attempt,
            )?;
            let log = std::fs::File::create(directory.join("ffmpeg.log"))?;
            let result = Command::new(&self.ffmpeg)
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-nostats",
                    "-rw_timeout",
                    "15000000",
                    "-i",
                ])
                .arg(&active.recording.source_url)
                .args([
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a:0?",
                    "-c",
                    "copy",
                    "-f",
                    "hls",
                    "-hls_time",
                    "4",
                    "-hls_playlist_type",
                    "event",
                    "-hls_flags",
                    "temp_file",
                    "-hls_segment_filename",
                ])
                .arg(directory.join("segment%08d.ts"))
                .arg(directory.join("index.m3u8"))
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::from(log))
                .kill_on_drop(true)
                .spawn();
            match result {
                Ok(child) => {
                    active.child = Some(child);
                    active.last_progress_at = now();
                }
                Err(error) => {
                    active.recording.phase = Phase::Failed;
                    active.recording.message = Some(format!("Cannot start FFmpeg: {error}"));
                    self.library.publish(&mut active.recording, true)?;
                    self.library.save_recording(&active.recording)?;
                    self.active = None;
                }
            }
        }
        Ok(())
    }
}

fn validate_end(ends_at: i64) -> Result<()> {
    if ends_at <= now() || ends_at - now() > 24 * 60 * 60 {
        bail!("End time must be in the future and within 24 hours");
    }
    Ok(())
}

fn next_attempt(root: &std::path::Path, number: &mut usize) -> Result<PathBuf> {
    std::fs::create_dir_all(root)?;
    loop {
        *number += 1;
        let path = root.join(format!("attempt-{number:06}"));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
}

async fn halt(child: &mut Option<Child>) {
    if let Some(mut child) = child.take() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"q\n").await;
        }
        if tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .is_err()
        {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}
