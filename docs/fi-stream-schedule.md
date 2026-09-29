# FI streaming schedule research

Inspected on 2026-09-28: https://video.fi.muni.cz/seznam_streamovanych_predmetu.

## What the page provides

The public page contains a server-rendered HTML table with Event, Date, Time, Room, and Stream columns. Its notice says it covers the next 14 days. Streaming is normally enabled for lectures and disabled for seminars with slash-containing codes and reservations, subject to requested exceptions.

The response inspected contained 300 rows, including duplicate rows for some unrelated courses. No structured feed was linked from this page or the public homepage. That does not establish that no feed exists.

The page reports planned streaming, not whether the HLS source is currently healthy. It does not explicitly label holidays or explain why a particular date has no event. An examination-period notice dated May 2026 appears only in an HTML comment and must not be treated as an active notice.

## User's courses in the inspected snapshot

| Course | Dates | Stream window | Room | Stream |
| --- | --- | --- | --- | --- |
| PV017 | 2026-09-29, 2026-10-06 | 07:55–09:55 | A318 | Yes |
| PA191 | 2026-09-30, 2026-10-07 | 07:55–09:55 | A318 | Yes |
| PA015 | 2026-09-30, 2026-10-07 | 09:55–11:55 | A217 | Yes |
| PV005 | 2026-10-01, 2026-10-08 | 15:55–17:55 | A217 | Yes |
| PA103 | 2026-09-30, 2026-10-07 | 13:55–15:55 | FAST-D182 | Yes |
| PA220 | 2026-09-30, 2026-10-07 | 15:55–17:55 | FAST-R229 | Yes |
| PA017/CZ | 2026-09-30, 2026-10-07 | 17:55–19:55 | FAST-R229 | No |

The listed windows cover five minutes before and after the user's class times. They describe source availability; they do not automatically change the user's requested recording windows. The table does not supply FAST HLS URLs.

## Proposed scheduler use

This is a design proposal, not implemented scheduling behavior.

- Keep the user's weekly timetable as the list of classes to record. Consult this page for dated streaming expectations.
- Match the exact course code, date, known room, and overlapping time window. Preserve suffixes such as /CZ. Do not require the stream window to equal the class window.
- A matching Yes row confirms expected streaming; continue retrying temporary source failures within the recording window.
- A matching No row is evidence that this occurrence is not expected to stream. Propose skipping it with a visible reason, after agreeing this policy with the user.
- A missing row, failed fetch, changed HTML structure, stale snapshot, conflicting duplicates, or date outside the published horizon is unknown. Do not silently convert unknown to a cancellation or holiday. Fall back to the user's timetable and source probing.
- A missing occurrence within a fresh, successfully parsed horizon can be displayed as not listed. Stronger automatic skipping based on absence needs evidence that the table is complete for that date.
- Deduplicate identical rows; retain fetch time, source URL, and the matched evidence for any skip decision.
- Refresh periodically and shortly before upcoming classes. Choose freshness and request-frequency limits during scheduling implementation.
- Room mismatches should be surfaced, preserving the existing decision that room changes are manual.

This source can reflect timetable exceptions without requiring a separate holiday calendar, but absence alone does not prove a holiday, cancellation, or disabled stream.
