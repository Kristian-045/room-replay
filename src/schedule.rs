use crate::subjects::{Subject, SubjectId};
use chrono::{Datelike, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Bratislava;

pub struct Window {
    pub subject_id: SubjectId,
    pub room_page_url: String,
    pub starts_at: i64,
    pub ends_at: i64,
}

fn weekday(day: &str) -> Option<Weekday> {
    match day {
        "Monday" => Some(Weekday::Mon),
        "Tuesday" => Some(Weekday::Tue),
        "Wednesday" => Some(Weekday::Wed),
        "Thursday" => Some(Weekday::Thu),
        "Friday" => Some(Weekday::Fri),
        "Saturday" => Some(Weekday::Sat),
        "Sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Windows currently open in Europe/Bratislava. Entries without a configured
/// source cannot start a capture.
pub fn open_windows(subjects: &[Subject], timestamp: i64) -> Vec<Window> {
    let Some(utc) = Utc.timestamp_opt(timestamp, 0).single() else {
        return Vec::new();
    };
    let local = utc.with_timezone(&Bratislava);
    let date = local.date_naive();
    subjects
        .iter()
        .filter_map(|subject| {
            let slot = &subject.weekly_slot;
            if weekday(&slot.day)? != local.weekday() {
                return None;
            }
            let room_page_url = slot.room_page_url.clone()?;
            let start_time = NaiveTime::parse_from_str(&slot.starts_at, "%H:%M").ok()?;
            let end_time = NaiveTime::parse_from_str(&slot.ends_at, "%H:%M").ok()?;
            let start = Bratislava
                .from_local_datetime(&date.and_time(start_time))
                .single()?;
            let end = Bratislava
                .from_local_datetime(&date.and_time(end_time))
                .single()?;
            if timestamp < start.timestamp() || timestamp >= end.timestamp() {
                return None;
            }
            Some(Window {
                subject_id: subject.id.clone(),
                room_page_url,
                starts_at: start.timestamp(),
                ends_at: end.timestamp(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    fn at(value: &str) -> i64 {
        DateTime::parse_from_rfc3339(value).unwrap().timestamp()
    }

    #[test]
    fn starts_during_window_and_ends_on_time() {
        let subjects = crate::subjects::defaults();
        assert!(open_windows(&subjects, at("2026-09-29T07:59:59+02:00")).is_empty());
        let windows = open_windows(&subjects, at("2026-09-29T09:20:00+02:00"));
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].subject_id.0, "PV017");
        assert_eq!(windows[0].starts_at, at("2026-09-29T08:00:00+02:00"));
        assert_eq!(windows[0].ends_at, at("2026-09-29T09:50:00+02:00"));
        assert!(open_windows(&subjects, at("2026-09-29T09:50:00+02:00")).is_empty());
    }

    #[test]
    fn repeats_next_week_across_daylight_saving_change() {
        let subjects = crate::subjects::defaults();
        let windows = open_windows(&subjects, at("2026-10-27T08:30:00+01:00"));
        assert_eq!(windows[0].subject_id.0, "PV017");
        assert_eq!(windows[0].starts_at, at("2026-10-27T08:00:00+01:00"));
    }

    #[test]
    fn ignores_subjects_without_stream_links() {
        let subjects = crate::subjects::defaults();
        assert!(open_windows(&subjects, at("2026-09-30T14:30:00+02:00")).is_empty());
    }
}
