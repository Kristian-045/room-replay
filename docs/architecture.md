# Codebase design proposal

Status: Rust backend and Svelte with TypeScript and Tailwind CSS frontend selected. First implementation in progress. This document does not change the agreed product specification.

## Technology direction

Use Rust for the backend, selected by the user for its strong types, and Svelte with TypeScript and Tailwind CSS for the frontend. The first slice uses Tokio and Axum, SQLite through rusqlite for metadata, FFmpeg as a child process for media capture, and hls.js for browser playback.

Keep the Player module in plain TypeScript. Svelte displays its state and calls its interface; playback behavior does not depend on Svelte. Build the frontend as static browser assets served by Rust, with no separate frontend server required in production.

Use distinct identifier types, validated recording windows, and enums for recording phases to express invariants. Rust types complement runtime validation and recovery tests; they do not guarantee consistency between a child process, files, and database records. Media work belongs to FFmpeg.

Propose one application process that serves browser assets, control requests, and recording media, packaged with FFmpeg in one container. Store SQLite metadata and recording files in persistent storage. A separate database server, reverse proxy, and queue are not required by the current scope. Existing tailnet deployment may supply a proxy if needed.

## Modules and ownership

The first slice implements rooms, recorder, library, HTTP, and player modules. Timetable calculation, retention, and viewing sessions below remain planned. SQLite currently stores typed room and recording snapshots as JSON documents; schema migrations and indexed timetable queries will be added with scheduling. Database access is serialized inside the recorder task for this single-user slice.

| Module | Interface responsibilities | Hidden implementation |
| --- | --- | --- |
| Timetable | Save entries; calculate due occurrences for a supplied time | Local time conversion, recurrence, overlap rejection |
| Recorder | Start an occurrence; stop or extend it; observe its state | One active recording, durable intent, retries, FFmpeg supervision, recovery, finalization |
| Library | List recordings; keep or delete; open or renew viewing sessions; save progress | SQLite metadata, recording files, usage accounting, retention, expiring playback protection |
| Player | Open a recording; seek; select speed; go live | hls.js, buffering, catch-up behavior, progress updates, viewing-session renewal |
| HTTP | Translate browser requests into module calls and serve media | Request validation, response mapping, file delivery |

The recorder owns the recording lifecycle. The timetable never starts FFmpeg directly. Scheduled and manual recording use the same recorder interface. Admission must reject conflicts with existing reservations, including manual starts and extensions.

The library owns deletion and playback protection together. Opening a viewing session and selecting cleanup candidates must be coordinated so cleanup cannot race with a new viewer. Protection expires when renewal stops; a closed browser must not protect a recording forever.

Keep persistence details private to the backend modules, with explicit ownership of tables and mutations. Do not expose a generic repository interface just to wrap SQL.

## Recovery design to validate

Persist a recording's identity, selected source, and end time before capture starts. Snapshot these values so later timetable changes cannot change an active recording. Identify scheduled occurrences durably to prevent duplicate capture after restart.

Model source loss as a retrying phase within the same recording. Completion and completeness are separate: a finished recording may contain gaps. A stopped recording must remain stopped after restart.

Prototype how multiple FFmpeg attempts produce one playable recording. Prefer separate attempt directories with a recording-level playlist that handles discontinuities and finalization, subject to testing. Do not assume blindly appending to a playlist after a crash preserves timestamps or avoids filename collisions.

SQLite and filesystem operations do not form one atomic transaction. Recovery must reconcile interrupted file publication and deletion with metadata. The library owns that reconciliation, and the recorder coordinates it before resuming capture.

## Proposed Rust layout

```text
Cargo.toml
src/
  main.rs
  lib.rs
  timetable/
  recorder/
  library/
  http/
web/src/
  player/
  rooms/
  timetable/
  recordings/
migrations/
tests/integration/
docs/
```

Keep FFmpeg supervision inside recorder initially. Extract another module only when it hides a distinct, reusable set of behavior. Start with a single Rust crate containing a library and binary, avoiding a multi-crate workspace initially.

## Tests and seams

- Exercise timetable calculations with explicit times, including daylight-saving transitions and midnight crossings.
- Test the recorder through its lifecycle interface using a controllable capture adapter for failures and a real FFmpeg adapter for media integration tests. This is a real seam because both implementations are needed.
- Test the library with temporary SQLite databases and directories, including deletion recovery and viewer protection races. Avoid mocking SQL itself.
- Test playback in the target browsers against growing and interrupted recordings, including long seeks and returning from 2x to 1x.
- Validate actual child-process termination, restart recovery, and playlist continuity with real FFmpeg before relying on simulated failure tests.

## References

- Rust web framework: https://docs.rs/axum/latest/axum/
- SQLite storage: https://www.sqlite.org/appfileformat.html
- FFmpeg HLS output: https://ffmpeg.org/ffmpeg-formats.html#hls-2
- Browser HLS playback: https://github.com/video-dev/hls.js
- Svelte: https://svelte.dev/docs/svelte/overview

## Design work next

- Specify recording states and commands, including which module serializes admission and timetable changes to prevent recording conflicts.
- Define ownership of persisted room settings, timetable entries, recording lifecycle metadata, media files, and viewing progress.
- Define the HTTP interface between Rust and TypeScript, including how shared request and response types stay consistent.
- Validate the proposed capture and playlist recovery approach with a playback proof of concept.

Exact package versions, storage paths, and container settings belong to implementation after the design and playback proof of concept.
