# Personal stream DVR

Status: first-version specification with interview decisions recorded. The initial recording/playback slice is being implemented; this document describes the full intended version, not a claim that all features are complete. See README.md for current coverage.

## Confirmed scope

- Support recordings from different CESNET rooms. https://live.cesnet.cz/munifia318.html is the initial example, not the only source.
- The user expects room URLs to differ in their final component and streams to share a format. Verify source resolution and compatibility before relying on that assumption.
- Use weekly timetable entries with explicit recording start and end times. Individual dated entries are out of scope.
- Record even when nobody is watching.
- Prioritize Brave and Chrome on the user's Arch Linux laptop. Brave on Android is a secondary target.
- Personal use, accessed through the user's tailnet, with access restricted to the user.
- Both playback devices already access the server through the tailnet. The exact application network configuration remains a deployment detail to verify.
- No application password. Deployment must restrict access to the user through the tailnet.
- Provide a recording storage cap of approximately 100 GB.
- Target the user's TrueNAS Community 25.10.4 Goldeye server, hostname `nas`.

## Requirements carried forward from the handoff

- Play a recording from its beginning while recording continues.
- Support 2x playback with audio during recording.
- Return to 1x when playback catches up with the live edge.
- Keep finished recordings available for seeking, speed selection, and deletion.
- Free, self-hosted software with no recurring application fee.

## Confirmed behavior

### Rooms

- Preload FI A217, A218, A318, A319, and A320 using verified CESNET sources. S108 is excluded at the user's request.
- The user's weekly classes are recorded in [timetable.md](timetable.md); FI A318 and A217 cover the four currently identified FI classes. FAST classes await source links.
- Save rooms by name and CESNET URL.
- Select a saved room for each weekly timetable entry and for Record now.
- Change a timetable entry's room manually when a lecture moves. Automatic room discovery is outside scope.
- Capture at most one recording at a time across all rooms.

### Recording controls

- Interpret weekly times in Europe/Bratislava, preserving local wall-clock times across daylight-saving changes.
- Reject overlapping timetable entries across all rooms. Automatic merging is outside scope.
- Timetable edits affect future occurrences only. Use Stop and Extend to change an active recording.
- Manual skipping of cancelled sessions is not required. During a scheduled window, detect source availability and retry automatically when it is offline.
- Capture whatever footage the source supplies, including an empty room or holding slide. Do not attempt to detect lecture content.
- Keep trying until the scheduled end if the source starts late. Stop at the scheduled end even if the broadcast continues, unless the user extends the recording.
- If no footage was captured, show No stream available in recording history without creating an empty library recording.
- Offer Record now with a required end time.
- Allow stopping a recording early and extending its end time.

### Storage cleanup

- Automatically delete the oldest finished, unprotected recordings when space is needed.
- Offer a Keep option to protect selected recordings from automatic deletion.
- Protect recordings from automatic deletion while they are actively being watched.
- If sufficient space cannot be freed, stop capturing and show the reason.

### Recovery

- After an upstream outage or server restart, resume capture if the recording's end time has not passed.
- Keep captured portions together as one logical recording and mark it incomplete when footage is missing.
- Do not imply that footage missed during an outage can be recovered.

### Playback

- Start at the beginning on first viewing, including while capture continues.
- Save viewing progress and resume on return. The user favors this if implementation remains modest.
- Share viewing progress across the user's devices if implementation remains modest. Concurrent playback conflict handling remains a proposed implementation default.
- Provide Start over and Go live controls, with Go live applicable to ongoing recordings.

## Acceptance checks

1. With the app unopened, an 08:00-10:00 timetable entry starts capture automatically and stops at 10:00.
2. At 09:00, the user can open that ongoing recording at its beginning, seek within captured footage, and watch at 2x with audible audio while capture continues.
3. When accelerated playback catches up, it returns to 1x without repeatedly stalling. The exact buffer margin will be established by the playback prototype.
4. A finished recording remains playable with seeking and speed selection after the app restarts.
5. A source that starts late or temporarily disappears is retried within the scheduled window. Captured portions remain available together, with missing footage indicated by incomplete status.
6. Restarting the app during a scheduled window resumes capture for the remaining time. An entirely offline source leaves a history entry and no empty recording.
7. Under a small test storage cap, cleanup deletes the oldest eligible finished recording, preserves Keep recordings and recordings being watched, and stops capture with a visible reason if no space can be freed.
8. Timetable edits leave active recordings unchanged, overlapping entries across all rooms are rejected, and Record now requires an end time. Stop and Extend work on active recordings without allowing simultaneous capture.
9. Verify the core playback checks on Arch Linux Brave and Chrome. Repeat on Android Brave as the secondary target and document any limitations.
10. If the modest progress-saving feature is implemented, returning to a recording resumes its saved position, including from the other device.
11. Before deployment is accepted, verify that application access is restricted to the user through the tailnet without an application password.
12. Save two CESNET rooms and schedule non-overlapping entries for them. Each entry captures its selected room; changing the room of a future entry changes the source used for that occurrence.

## Implementation validation still needed

- Prove growing-recording playback, audio at 2x, seeking, and the transition to 1x before committing to a container design.
- Verify the source URL from the handoff: https://cdn.streaming.cesnet.cz/muni/munifia318.stream/playlist.m3u8.
- Inspect the server's existing tailnet routing and select persistent storage paths during deployment work.
- Choose retry intervals, storage headroom, and the live-edge buffer margin through implementation tests. These values are not user-facing requirements yet.

## Deployment evidence

TrueNAS 25.10 documentation identifies this release as Goldeye and supports custom applications. TrueNAS documents Docker Compose deployment through Install via YAML.

- https://www.truenas.com/docs/scale/25.10/scaleuireference/apps/
- https://apps.truenas.com/managing-apps/installing-custom-apps/

The handoff's FFmpeg, HLS, and browser player design remains a candidate pending a playback proof of concept. Hardware and stream bitrate estimates in the handoff have not been reverified.
