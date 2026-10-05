# Weekly timetable

Provided by the user for the week beginning 2026-09-28, with Monday classes added on 2026-10-05. Treat these as weekly recurring classes in Europe/Bratislava time, following the agreed weekly scheduling requirement.

The app automatically records configured FI classes during these windows. It retries unavailable streams until the scheduled end time. FAST entries remain deferred until their source links are configured.

## FI classes with verified sources

| Day | Time | Course | Name | Room |
| --- | --- | --- | --- | --- |
| Monday | 08:00–09:50 | PV157 | Autentizace a řízení přístupu | FI A217 |
| Monday | 16:00–17:50 | PV281 | Programování v Rust | FI A217 |
| Tuesday | 08:00–09:50 | PV017 | Bezpečnost IT | FI A318 |
| Wednesday | 08:00–09:50 | PA191 | Advanced Computer Networking | FI A318 |
| Wednesday | 10:00–11:50 | PA015 | AI for Software Professionals | FI A217 |
| Thursday | 16:00–17:50 | PV005 | Služby počítačových sítí | FI A217 |

FI A217, A218, A318, A319, and A320 are preloaded in the app. S108 is excluded at the user's request.

## FAST classes deferred until source links are available

| Day | Time | Course | Name | Room |
| --- | --- | --- | --- | --- |
| Wednesday | 14:00–15:50 | PA103 | OOAD Methods | FAST/D182 |
| Wednesday | 16:00–17:50 | PA220 | DB Systems for Data Analytics | FAST/R229 |
| Wednesday | 18:00–19:50 | PA017/CZ | Information Systems Management | FAST/R229 |

Do not create stream URLs for FAST by guessing from FI's URL pattern. Their availability and links remain unknown.

FI's [dated streaming schedule](https://video.fi.muni.cz/seznam_streamovanych_predmetu), inspected on 2026-09-28, lists PA103 and PA220 with streaming enabled and PA017/CZ with streaming disabled for the next two Wednesday occurrences. FAST source URLs are still unknown. The original four FI courses were listed with streaming enabled. The Monday additions use the already configured A217 source; their per-occurrence availability has not been verified against the dated table. See [the schedule research](fi-stream-schedule.md) for exact dates, windows, and proposed integration behavior.

Room links were verified against [FI's official streaming page](https://www.fi.muni.cz/tech/video.html.cs) and its linked CESNET player pages on 2026-09-28. The FI page states that streaming normally depends on the teaching timetable; saved rooms can be offline outside lectures.
