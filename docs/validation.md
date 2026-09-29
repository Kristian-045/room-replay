# First playback build validation

Validated on 2026-09-28 using a local generated H.264/AAC live HLS source and the actual Rust server, FFmpeg recorder, and Svelte/Tailwind frontend. Live CESNET validation was added on 2026-09-29.

## Passed

- Rust formatting, Clippy with warnings treated as errors, and backend tests.
- Svelte/TypeScript checks and the production frontend build.
- Player catch-up policy tests.
- Headless Chromium integration: saving a room, manual capture, rejection of a second recording, and a growing playlist.
- Playback before capture ends, seeking to the beginning, and 2x playback with increasing audio and video decode counters.
- Return to 1x near the live edge.
- Graceful server restart resumes the same recording, adds a playlist discontinuity, and marks the recording incomplete.
- Stop publishes ENDLIST and remains stopped after another restart.
- Finished playback crosses the attempt discontinuity and reaches the end at 2x.
- Desktop and 390px-wide layouts rendered without browser exceptions or horizontal overflow. Screenshots were inspected.

The test is reproducible with `node web/tests/playback.mjs` after building the backend and frontend. The updated run also passed room removal, persistence of removal across restart, subject-based room selection, filtering, and reassignment of existing recordings. Its artifacts are in `/tmp/cesnet-dvr-test-mZkxMx` on the development machine. Backend tests also verify migration of older recordings and retention of recording files after removing a room.

## Live CESNET check, 2026-09-29

The A318 HLS source was live. A short sample was captured through the running app and exercised in headless Chromium and Brave on this Linux machine. Both passed playback during capture, seeking to the beginning, 2x playback with increasing audio/video decode counters, and return to 1x near the live edge. Stop finalized the playlist with ENDLIST.

- Source: https://live.cesnet.cz/munifia318.html
- Recording ID: `88ef5514-4c65-4b05-8d7b-df0b01070b30`, retained in the local library as an unassigned sample.
- Test: `DVR_LIVE_TEST=1 node web/tests/live.mjs`, against an already running local app with no active recording.
- Results and screenshots: `/tmp/cesnet-live-test-ff0Mhc`.

This was a short live sample, not the full lecture or an hour-long seek test.

## Player controls and sidebar, 2026-09-29

`node web/tests/player-controls.mjs` passed in Chromium and Brave against the running app. It makes no recording-management requests. Checks covered collapsed Rooms, the sidebar subject filter, play/pause, seeking, mute, speed selection, fullscreen with controls retained, auto-hide and pointer reveal, and control placement within a 390px-wide player. Desktop and narrow-layout screenshots were inspected at `/tmp/cesnet-controls-vlGjFQ`.

The active PV017 recording continued during these checks; the server was not restarted. Svelte/TypeScript validation, the frontend production build, and player policy tests passed.

## Remaining validation

- Listening to audio at different speeds to assess its quality. Decode counters confirm audio processing, not subjective listening quality.
- Long recordings, changing upstream keyframe intervals, and extended source outages.
- Abrupt process termination, host failure, and disk-full recovery.
- Brave on a physical Android device and normal interactive playback. A narrow desktop viewport is not an Android playback test.
- Full specification checks for scheduling, retention, saved progress, and tailnet deployment after those features are implemented.

Player update: the read-only browser checks passed in Chromium and Brave for speed buttons, arrow-key seeking and speed changes, and saved position after refresh and reopening. Start over is removed. Artifacts: `/tmp/cesnet-controls-zangO5`. Svelte check and the production build passed. Progress currently uses browser-local storage.

## TrueNAS custom app, 2026-09-29

Built `room-replay:0.1.0` from the multistage Dockerfile and started it locally with a writable data mount. Transferred the 198 MiB compressed image to TrueNAS; local and remote SHA-256 matched. Created the `tank/room-replay` dataset through the TrueNAS API with a 100 GiB quota and UID/GID 568, then installed the app through `app.create`. TrueNAS reported one running container bound to `192.0.2.10:3000`. The tailnet API returned five rooms, seven subjects, and zero recordings. The app did not answer on `192.0.2.20:3000`. Brave and Chromium passed a narrow-screen UI smoke check over the tailnet.

## Weekly scheduler, 2026-09-29

Added Europe/Bratislava weekly windows for the FI subjects with configured room streams. Rust tests cover start/end boundaries, the October daylight-saving change, missing FAST streams, and persistence that prevents a duplicate occurrence after restart. `cargo test` passed eight tests and `cargo clippy --all-targets -- -D warnings` passed. Built `room-replay:0.2.0`, transferred and checksum-verified its archive, and updated the TrueNAS custom app through `app.update`. TrueNAS reported the new image running. Today's PV017 recording retained its original ID and grew after the update, confirming restart recovery. The NAS recording was started manually as a bridge before the scheduler update; the next scheduled occurrence will use the scheduler.
