//! Calendar events.
//!
//! An event is just a note whose front-matter contains `type: event` and a
//! `date`. This keeps events plain Markdown (compatible with Obsidian and
//! Dataview), lets them hold arbitrary notes and `[[links]]`, and means any
//! existing note can be turned into an event by adding two properties:
//!
//! ```text
//! ---
//! type: event
//! date: 2026-10-25
//! start: 14:00
//! end: 15:00
//! location: Clinic
//! ---
//! Bring [[Insurance card]].
//! ```

use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::frontmatter::Frontmatter;
use crate::note::{Note, NoteId};
use crate::vault::Vault;

const KIND: &str = "event";
const DATE_FMT: &str = "%Y-%m-%d";
const TIME_FMT: &str = "%H:%M";

/// An event read from the vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: NoteId,
    pub title: String,
    pub date: NaiveDate,
    pub start: Option<NaiveTime>,
    pub end: Option<NaiveTime>,
    pub location: Option<String>,
}

impl Event {
    /// Parses an event out of a note. Returns `None` if the note is not an event.
    pub fn from_note(note: &Note) -> Option<Self> {
        let fm = note.frontmatter();
        if fm.get("type") != Some(KIND) {
            return None;
        }
        let date = NaiveDate::parse_from_str(fm.get("date")?, DATE_FMT).ok()?;
        let time = |key| fm.get(key).and_then(|t| NaiveTime::parse_from_str(t, TIME_FMT).ok());
        Some(Self {
            id: note.id.clone(),
            title: fm
                .get("title")
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| note.title())
                .to_owned(),
            date,
            start: time("start"),
            end: time("end"),
            location: fm.get("location").filter(|l| !l.is_empty()).map(str::to_owned),
        })
    }

    pub fn is_all_day(&self) -> bool {
        self.start.is_none()
    }

    /// Human readable time range, e.g. `"14:00 – 15:00"` or `"All day"`.
    pub fn time_label(&self) -> String {
        match (self.start, self.end) {
            (None, _) => "All day".into(),
            (Some(s), None) => s.format(TIME_FMT).to_string(),
            (Some(s), Some(e)) => format!("{} – {}", s.format(TIME_FMT), e.format(TIME_FMT)),
        }
    }
}

/// User input for creating a new event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDraft {
    pub title: String,
    pub date: NaiveDate,
    pub start: Option<NaiveTime>,
    pub end: Option<NaiveTime>,
    pub location: String,
    pub description: String,
}

impl EventDraft {
    pub fn new(date: NaiveDate) -> Self {
        Self {
            title: String::new(),
            date,
            start: None,
            end: None,
            location: String::new(),
            description: String::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            return Err(Error::InvalidEvent("title is required".into()));
        }
        if let (Some(s), Some(e)) = (self.start, self.end) {
            if e < s {
                return Err(Error::InvalidEvent("end time is before start time".into()));
            }
        }
        if self.end.is_some() && self.start.is_none() {
            return Err(Error::InvalidEvent("end time requires a start time".into()));
        }
        Ok(())
    }

    fn render(&self) -> String {
        let mut fm = Frontmatter::new();
        fm.set("type", KIND)
            .set("title", self.title.trim())
            .set("date", self.date.format(DATE_FMT).to_string());
        if let Some(s) = self.start {
            fm.set("start", s.format(TIME_FMT).to_string());
        }
        if let Some(e) = self.end {
            fm.set("end", e.format(TIME_FMT).to_string());
        }
        if !self.location.trim().is_empty() {
            fm.set("location", self.location.trim());
        }
        let mut out = fm.render();
        out.push_str(&format!("# {}\n\n", self.title.trim()));
        if !self.description.trim().is_empty() {
            out.push_str(self.description.trim());
            out.push('\n');
        }
        out
    }
}

/// Event storage configuration and queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EventStore {
    /// Folder where new events are created.
    pub folder: String,
}

impl Default for EventStore {
    fn default() -> Self {
        Self {
            folder: "Events".into(),
        }
    }
}

impl EventStore {
    /// Creates a note for the event and returns its id.
    pub fn create(&self, vault: &mut Vault, draft: &EventDraft) -> Result<NoteId> {
        draft.validate()?;
        let safe_title: String = draft
            .title
            .trim()
            .chars()
            .map(|c| if "[]|#^:*?\"<>/\\".contains(c) { ' ' } else { c })
            .collect();
        let base = format!(
            "{}/{} {}",
            self.folder,
            draft.date.format(DATE_FMT),
            safe_title.split_whitespace().collect::<Vec<_>>().join(" ")
        );
        let id = vault.unique_id(&NoteId::new(&base)?);
        vault.create(id.clone(), &draft.render())?;
        Ok(id)
    }

    /// Every event in the vault (from any folder), sorted chronologically.
    pub fn all(&self, vault: &Vault) -> Vec<Event> {
        let mut events: Vec<Event> = vault.notes().filter_map(Event::from_note).collect();
        events.sort_by(|a, b| {
            (a.date, a.start, &a.title).cmp(&(b.date, b.start, &b.title))
        });
        events
    }

    /// Events on a given day.
    pub fn on(&self, vault: &Vault, date: NaiveDate) -> Vec<Event> {
        self.between(vault, date, date)
    }

    /// Events with `from <= date <= to`.
    pub fn between(&self, vault: &Vault, from: NaiveDate, to: NaiveDate) -> Vec<Event> {
        self.all(vault)
            .into_iter()
            .filter(|e| (from..=to).contains(&e.date))
            .collect()
    }
}

/// Parses `"HH:MM"` (also accepts `"H:MM"`); empty input yields `Ok(None)`.
pub fn parse_time(input: &str) -> Result<Option<NaiveTime>> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }
    NaiveTime::parse_from_str(input, TIME_FMT)
        .map(Some)
        .map_err(|_| Error::InvalidEvent(format!("invalid time {input:?}, expected HH:MM")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn create_and_query() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::open(dir.path()).unwrap();
        let store = EventStore::default();

        let mut draft = EventDraft::new(d(25));
        draft.title = "Dentist: checkup".into();
        draft.start = Some(t(14, 0));
        draft.end = Some(t(15, 0));
        draft.location = "Clinic".into();
        draft.description = "Bring [[Insurance card]]".into();
        let id = store.create(&mut vault, &draft).unwrap();
        assert_eq!(id.as_str(), "Events/2026-10-25 Dentist checkup");

        let mut all_day = EventDraft::new(d(24));
        all_day.title = "Holiday".into();
        store.create(&mut vault, &all_day).unwrap();

        let events = store.all(&vault);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].title, "Holiday");
        assert!(events[0].is_all_day());

        let on_25 = store.on(&vault, d(25));
        assert_eq!(on_25.len(), 1);
        assert_eq!(on_25[0].title, "Dentist: checkup");
        assert_eq!(on_25[0].time_label(), "14:00 – 15:00");
        assert_eq!(on_25[0].location.as_deref(), Some("Clinic"));
        assert_eq!(store.between(&vault, d(1), d(31)).len(), 2);
    }

    #[test]
    fn validation() {
        let mut draft = EventDraft::new(d(1));
        assert!(draft.validate().is_err());
        draft.title = "x".into();
        assert!(draft.validate().is_ok());
        draft.start = Some(t(10, 0));
        draft.end = Some(t(9, 0));
        assert!(draft.validate().is_err());
    }

    #[test]
    fn time_parsing() {
        assert_eq!(parse_time("").unwrap(), None);
        assert_eq!(parse_time("9:05").unwrap(), Some(t(9, 5)));
        assert!(parse_time("25:00").is_err());
    }
}
