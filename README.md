# Room Replay

A personal CESNET DVR built with Rust, Svelte, TypeScript, and Tailwind CSS.

Most of this project was generated with AI coding tools and then adapted for personal use. It has been exercised with generated test streams, but still needs more testing with live lectures and long recordings.

The app saves rooms, records one room at a time, starts configured FI classes from the weekly timetable, and lets you watch a recording while capture continues. FFmpeg copies the source audio/video into HLS segments; the browser uses hls.js.

## Run locally

Install Rust, Node.js 22.18 or newer, npm, and FFmpeg with HLS support. From the repository root:

```sh
npm --prefix web ci
npm --prefix web run build
cargo run
```

Open http://127.0.0.1:3000. FI rooms A217, A218, A318, A319, and A320 are preloaded. You can remove rooms, including defaults; removed rooms stay removed after restart, and existing recordings are kept. A room cannot be removed while it is recording. You can also add a CESNET room page or a direct CESNET HLS playlist URL.

Your seven timetable subjects are preloaded separately from rooms. Choose a subject when recording to suggest its configured FI room, or leave the recording unassigned. You can assign or change a recording's subject afterward. Filter the library by subject or Unassigned; recordings always appear newest first. FAST subjects are included, but their stream links are not configured.

Select a room and an end time, then choose Record now. Watch becomes available after FFmpeg publishes the first complete segment.

The [weekly timetable](docs/timetable.md) automatically starts the four FI subjects in Europe/Bratislava time and stops at their listed end times. Starting the app during a class captures the remaining time. The app retries an unavailable stream until the end, and records a No stream history entry if no footage arrives. FAST subjects remain visible but cannot run until their stream links are configured. The timetable is preloaded and cannot yet be edited in the UI.

The player restores its saved position and offers seeking, 0.5x through 2x playback with −/+ buttons, and Go live. With a player button focused, left/right arrows seek five seconds and up/down arrows change speed. Refreshing reopens the last recording, paused at its saved position. Progress is stored in this browser and does not sync between devices. Controls sit inside the video and remain available in fullscreen. Move the pointer, tap, or focus a control to reveal them during playback. Accelerated playback returns to 1x within eight seconds of the captured live edge. This margin is an initial implementation value to validate with real lectures.

The subject filter is in the sidebar. Rooms and Subjects are collapsed by default; expand Rooms to add or remove a room.

Stop ends an active recording; Extend changes its deadline. Closing the browser does not stop capture. A stopped recording stays stopped after an application restart. An interrupted recording resumes within its original recording window and is marked incomplete. If no footage arrives, the attempt appears in history instead of the playable library.

For frontend development, run `cargo run` and `npm --prefix web run dev` in separate terminals. Vite forwards `/api` and `/media` to the Rust server.

## Current limits

- Editable timetable entries, storage-cap enforcement and automatic cleanup, Keep protection, recording deletion controls, and cross-device playback progress are not implemented yet.
- Only one application instance should use a data directory. Multi-process coordination and abrupt-process-death handling need hardening before unattended deployment.
- The manual end-time field uses the browser's timezone and currently limits a recording to the next 24 hours. Weekly schedules use Europe/Bratislava.
- No application login is provided. The TrueNAS deployment binds only to its Tailscale IP. The local development server binds to loopback.
- Local generated-stream tests do not replace testing a live CESNET lecture, long recordings, or Brave on an Android device.
- See [TrueNAS deployment](docs/truenas-deployment.md) for an example setup.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `DVR_BIND` | `127.0.0.1:3000` | HTTP listen address |
| `DVR_DATA_DIR` | `data` | SQLite metadata and recording files |
| `DVR_WEB_DIR` | `web/dist` | Built frontend assets |
| `DVR_FFMPEG` | `ffmpeg` | FFmpeg executable |
| `RUST_LOG` | `info` | Backend logging |
| `DVR_ALLOW_TEST_SOURCES` | unset | Set to `1` only for local fixtures; permits loopback HTTP HLS sources |

Keep the data directory to retain rooms and recordings. Each recording contains separate capture attempts and an aggregate playlist with discontinuities between attempts. Attempt-specific FFmpeg errors are in `data/recordings/<id>/attempt-*/ffmpeg.log`.

## Check the build

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
npm --prefix web run check
npm --prefix web test
npm --prefix web run build
```

The browser integration test requires Chromium with H.264/AAC support and FFmpeg with the libx264 encoder:

```sh
cargo build
npm --prefix web run build
node web/tests/playback.mjs
```

It creates a local generated live stream, records it through the actual backend, and exercises playback at 2x, seeking, catch-up, restart recovery, finalization, and a narrow viewport. It checks audio/video decoding; it cannot confirm audio quality by listening. It uses temporary data and retains logs/screenshots under the printed `/tmp/cesnet-dvr-test-*` path. Set `DVR_TEST_BROWSER` to use another Chromium-compatible executable, such as `/usr/bin/brave`.

To explicitly test a currently live A318 stream against the running local app, use `DVR_LIVE_TEST=1 node web/tests/live.mjs`. This records a short real sample, tests Chromium and Brave, stops capture, and keeps the sample in the library. It refuses to start if another recording is active.

See [the specification](docs/specification.md) for the full agreed scope and [the architecture](docs/architecture.md) for module responsibilities.
