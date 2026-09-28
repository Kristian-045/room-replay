# Room Replay

A personal CESNET DVR built with Rust, Svelte, TypeScript, and Tailwind CSS.

This is the first working recording/playback slice. It saves rooms, records one room at a time, and lets you watch a recording while capture continues. FFmpeg copies the source audio/video into HLS segments; the browser uses hls.js.

## Run locally

Install Rust, Node.js 22.18 or newer, npm, and FFmpeg with HLS support. From the repository root:

```sh
npm --prefix web ci
npm --prefix web run build
cargo run
```

Open http://127.0.0.1:3000. FI rooms A217, A218, A318, A319, and A320 are preloaded. Startup adds missing defaults without replacing existing rooms or duplicating matching sources. You can also add a CESNET room page or a direct CESNET HLS playlist URL. Select the room and an end time, then choose Record now. Watch becomes available after FFmpeg publishes the first complete segment.

The user's [weekly timetable](docs/timetable.md) is recorded for the scheduling implementation. Its entries do not trigger automatic capture yet; FAST sources remain deferred.

The player starts at the beginning and offers seeking, 0.5x through 2x playback, Start over, and Go live. Accelerated playback returns to 1x within eight seconds of the captured live edge. This margin is an initial implementation value to validate with real lectures.

Stop ends an active recording; Extend changes its deadline. Closing the browser does not stop capture. A stopped recording stays stopped after an application restart. An interrupted recording resumes within its original recording window and is marked incomplete. If no footage arrives, the attempt appears in history instead of the playable library.

For frontend development, run `cargo run` and `npm --prefix web run dev` in separate terminals. Vite forwards `/api` and `/media` to the Rust server.

## Current limits

- Weekly scheduling, storage-cap enforcement and automatic cleanup, Keep protection, deletion controls, and saved playback progress are not implemented yet.
- Only one application instance should use a data directory. Multi-process coordination and abrupt-process-death handling need hardening before unattended deployment.
- The manual end-time field uses the browser's timezone and currently limits a recording to the next 24 hours. Weekly schedules will use Europe/Bratislava as specified.
- No application login is provided. The development server binds to loopback. Tailnet-only deployment has not yet been configured or verified.
- Local generated-stream tests do not replace testing a live CESNET lecture, long recordings, or Brave on an Android device.
- No Docker deployment is supplied at this stage. Container packaging follows the playback validation and scheduling/storage work.

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

See [the specification](docs/specification.md) for the full agreed scope and [the architecture](docs/architecture.md) for module responsibilities.
