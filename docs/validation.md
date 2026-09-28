# First playback build validation

Validated on 2026-09-28 using a local generated H.264/AAC live HLS source and the actual Rust server, FFmpeg recorder, and Svelte/Tailwind frontend.

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

The test is reproducible with `node web/tests/playback.mjs` after building the backend and frontend. Temporary fixtures and screenshots from the passing run are in `/tmp/cesnet-dvr-test-80K4pU` on the development machine.

## Still needed

- Playback against a live CESNET lecture. The example room page resolved to the expected source, but that playlist returned HTTP 404 during this session.
- Listening to audio at different speeds to assess its quality. Decode counters confirm audio processing, not subjective listening quality.
- Long recordings, changing upstream keyframe intervals, and extended source outages.
- Abrupt process termination, host failure, and disk-full recovery.
- Actual Brave on Arch Linux and Android. A narrow desktop viewport is not an Android playback test.
- Full specification checks for scheduling, retention, saved progress, and tailnet deployment after those features are implemented.
