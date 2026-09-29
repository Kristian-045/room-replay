# Personal stream DVR

A personal DVR for scheduled capture and delayed viewing of CESNET room streams.

## Language

**Source stream**:
The live broadcast for a CESNET room from which recordings are captured.

**Room**:
A CESNET broadcast location with its own source stream.

**Subject**:
A course identified by its exact course code, such as PV017 or PA017/CZ. Recordings can belong to a subject independently of the room where they were captured.

**Timetable**:
The user's weekly recording start and end times.

**Timetable entry**:
A recurring weekly recording window for a subject and selected room, with a start and end time.

**Recording**:
Captured content from the source stream that can be watched while capture continues or after it finishes.

**Live edge**:
The latest captured position available for playback in an ongoing recording.

**Storage cap**:
The configured limit on space used for recordings.

**Protected recording**:
A recording marked Keep that automatic storage cleanup must not delete.

**Incomplete recording**:
A recording with missing footage because capture was interrupted or unavailable during its planned time.

**Viewing progress**:
The saved playback position used to resume a recording on a later visit.
