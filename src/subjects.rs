use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SubjectId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklySlot {
    pub day: String,
    pub starts_at: String,
    pub ends_at: String,
    pub room: String,
    pub room_page_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub id: SubjectId,
    pub name: String,
    pub weekly_slot: WeeklySlot,
}

/// The user's timetable, not a claim that any occurrence is currently streaming.
pub fn defaults() -> Vec<Subject> {
    [
        (
            "PV157",
            "Autentizace a řízení přístupu",
            "Monday",
            "08:00",
            "09:50",
            "FI A217",
            Some("munifia217"),
        ),
        (
            "PV281",
            "Programování v Rust",
            "Monday",
            "16:00",
            "17:50",
            "FI A217",
            Some("munifia217"),
        ),
        (
            "PV017",
            "Bezpečnost IT",
            "Tuesday",
            "08:00",
            "09:50",
            "FI A318",
            Some("munifia318"),
        ),
        (
            "PA191",
            "Advanced Computer Networking",
            "Wednesday",
            "08:00",
            "09:50",
            "FI A318",
            Some("munifia318"),
        ),
        (
            "PA015",
            "AI for Software Professionals",
            "Wednesday",
            "10:00",
            "11:50",
            "FI A217",
            Some("munifia217"),
        ),
        (
            "PA103",
            "OOAD Methods",
            "Wednesday",
            "14:00",
            "15:50",
            "FAST/D182",
            None,
        ),
        (
            "PA220",
            "DB Systems for Data Analytics",
            "Wednesday",
            "16:00",
            "17:50",
            "FAST/R229",
            None,
        ),
        (
            "PA017/CZ",
            "Information Systems Management",
            "Wednesday",
            "18:00",
            "19:50",
            "FAST/R229",
            None,
        ),
        (
            "PV005",
            "Služby počítačových sítí",
            "Thursday",
            "16:00",
            "17:50",
            "FI A217",
            Some("munifia217"),
        ),
    ]
    .into_iter()
    .map(|(code, name, day, start, end, room, page)| Subject {
        id: SubjectId(code.into()),
        name: name.into(),
        weekly_slot: WeeklySlot {
            day: day.into(),
            starts_at: start.into(),
            ends_at: end.into(),
            room: room.into(),
            room_page_url: page.map(|page| format!("https://live.cesnet.cz/{page}.html")),
        },
    })
    .collect()
}
