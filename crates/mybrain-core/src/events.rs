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
use crate::note::{Note, NoteId};
use crate::vault::Vault;

const KIND: &str = "event";
const DATE_FMT: &str = "%Y-%m-%d";
const TIME_FMT: &str = "%H:%M";

/// An event object (embedded inside any note via ```event blocks, or via note frontmatter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Note id containing this event object.
    pub id: NoteId,
    /// Line number where the event block starts (0-indexed).
    pub line: usize,
    pub title: String,
    pub date: NaiveDate,
    pub start: Option<NaiveTime>,
    pub end: Option<NaiveTime>,
    pub location: Option<String>,
    pub description: Option<String>,
}

impl Event {
    /// Parses the first event out of a note (legacy frontmatter or first ```event block).
    pub fn from_note(note: &Note) -> Option<Self> {
        Self::all_from_note(note).into_iter().next()
    }

    /// Parses all events from a note (both legacy frontmatter and all ```event blocks).
    pub fn all_from_note(note: &Note) -> Vec<Self> {
        let mut events = Vec::new();

        // 1. Check legacy frontmatter
        let fm = note.frontmatter();
        if fm.get("type") == Some(KIND) {
            if let Some(date_val) = fm.get("date") {
                if let Ok(date) = NaiveDate::parse_from_str(date_val, DATE_FMT) {
                    let time = |key| fm.get(key).and_then(|t| NaiveTime::parse_from_str(t, TIME_FMT).ok());
                    let title = fm
                        .get("title")
                        .filter(|t| !t.is_empty())
                        .unwrap_or_else(|| note.title())
                        .to_owned();
                    let location = fm.get("location").filter(|l| !l.is_empty()).map(str::to_owned);
                    let description = if !note.body().trim().is_empty() {
                        Some(note.body().trim().to_string())
                    } else {
                        None
                    };
                    events.push(Self {
                        id: note.id.clone(),
                        line: 0,
                        title,
                        date,
                        start: time("start"),
                        end: time("end"),
                        location,
                        description,
                    });
                }
            }
        }

        // 2. Scan note content for ```event blocks
        let mut in_block = false;
        let mut block_lines: Vec<&str> = Vec::new();
        let mut start_line = 0;

        for (i, line) in note.content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("```event") {
                in_block = true;
                start_line = i;
                block_lines.clear();
            } else if in_block && trimmed.starts_with("```") {
                in_block = false;
                if let Some(ev) = Self::parse_block(note, start_line, &block_lines) {
                    events.push(ev);
                }
                block_lines.clear();
            } else if in_block {
                block_lines.push(line);
            }
        }

        events
    }

    /// Parses an event from lines inside a ```event code block.
    pub fn parse_block(note: &Note, line: usize, lines: &[&str]) -> Option<Self> {
        let mut title: Option<String> = None;
        let mut date_str: Option<String> = None;
        let mut start_str: Option<String> = None;
        let mut end_str: Option<String> = None;
        let mut location: Option<String> = None;
        let mut description: Option<String> = None;

        for l in lines {
            let trimmed = l.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("title:").or_else(|| trimmed.strip_prefix("event:")) {
                title = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("date:") {
                date_str = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("time:") {
                let val = rest.trim();
                if let Some((s, e)) = val.split_once('-').or_else(|| val.split_once('–')) {
                    start_str = Some(s.trim().to_string());
                    end_str = Some(e.trim().to_string());
                } else {
                    start_str = Some(val.to_string());
                }
            } else if let Some(rest) = trimmed.strip_prefix("start:").or_else(|| trimmed.strip_prefix("from:")) {
                start_str = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("end:").or_else(|| trimmed.strip_prefix("to:")) {
                end_str = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("location:").or_else(|| trimmed.strip_prefix("loc:")) {
                location = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("description:").or_else(|| trimmed.strip_prefix("desc:")) {
                description = Some(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix('#') {
                if title.is_none() {
                    title = Some(rest.trim().to_string());
                }
            } else if title.is_none() {
                title = Some(trimmed.to_string());
            } else if description.is_none() {
                description = Some(trimmed.to_string());
            }
        }

        // Determine date: explicit date > date extracted from note ID/title > today
        let date = date_str
            .and_then(|s| NaiveDate::parse_from_str(&s, DATE_FMT).ok())
            .or_else(|| extract_date_from_str(note.id.name()))
            .or_else(|| extract_date_from_str(note.id.as_str()))
            .unwrap_or_else(|| chrono::Local::now().date_naive());

        let title = title.unwrap_or_else(|| format!("Event on {}", date.format("%b %-d")));

        let start = start_str.and_then(|s| parse_time(&s).ok().flatten());
        let end = end_str.and_then(|s| parse_time(&s).ok().flatten());

        Some(Self {
            id: note.id.clone(),
            line,
            title,
            date,
            start,
            end,
            location,
            description,
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

    /// Renders as a daily note task bullet line.
    pub fn to_markdown_bullet(&self) -> String {
        format!("- [ ] **{}** {}\n", self.time_label(), self.title)
    }
}

/// User input for creating or embedding a new event.
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

    /// Renders this draft as an ```event ... ``` markdown object block.
    pub fn render_block(&self) -> String {
        let mut out = String::from("```event\n");
        out.push_str(&format!("title: {}\n", self.title.trim()));
        out.push_str(&format!("date: {}\n", self.date.format(DATE_FMT)));
        if let Some(s) = self.start {
            if let Some(e) = self.end {
                out.push_str(&format!("time: {} - {}\n", s.format(TIME_FMT), e.format(TIME_FMT)));
            } else {
                out.push_str(&format!("start: {}\n", s.format(TIME_FMT)));
            }
        }
        if !self.location.trim().is_empty() {
            out.push_str(&format!("location: {}\n", self.location.trim()));
        }
        if !self.description.trim().is_empty() {
            out.push_str(&format!("description: {}\n", self.description.trim()));
        }
        out.push_str("```\n");
        out
    }

    /// Renders as legacy standalone event note.
    pub fn render(&self) -> String {
        format!("# {}\n\n{}", self.title.trim(), self.render_block())
    }
}

/// Event storage configuration and queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EventStore {
    /// Folder where new standalone event notes are created if requested.
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
    /// Appends an event object block to an existing note.
    pub fn append_to_note(&self, vault: &mut Vault, note_id: &NoteId, draft: &EventDraft) -> Result<()> {
        draft.validate()?;
        let block = draft.render_block();
        let formatted = format!("\n{block}\n");
        vault.append(note_id, &formatted)?;
        Ok(())
    }

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

    /// Every event assembled from all notes across the vault, sorted chronologically.
    pub fn all(&self, vault: &Vault) -> Vec<Event> {
        let mut events: Vec<Event> = vault.notes().flat_map(Event::all_from_note).collect();
        events.sort_by(|a, b| {
            (a.date, a.start, &a.title).cmp(&(b.date, b.start, &b.title))
        });
        events
    }

    /// Events on a given day across all notes.
    pub fn on(&self, vault: &Vault, date: NaiveDate) -> Vec<Event> {
        self.between(vault, date, date)
    }

    /// Events with `from <= date <= to` across all notes.
    pub fn between(&self, vault: &Vault, from: NaiveDate, to: NaiveDate) -> Vec<Event> {
        self.all(vault)
            .into_iter()
            .filter(|e| (from..=to).contains(&e.date))
            .collect()
    }
}

fn extract_date_from_str(s: &str) -> Option<NaiveDate> {
    if s.len() < 10 {
        return None;
    }
    for i in 0..=s.len() - 10 {
        if let Ok(d) = NaiveDate::parse_from_str(&s[i..i + 10], DATE_FMT) {
            return Some(d);
        }
    }
    None
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

    #[test]
    fn test_embedded_event_objects() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::open(dir.path()).unwrap();
        let store = EventStore::default();

        let note_content = r#"# Project Planning

Here are the upcoming milestones for the release:

```event
title: Architecture Review
date: 2026-10-27
time: 10:00 - 11:30
location: Conference Room B
description: Review technical spec
```

Some notes in between.

```event
title: Team Sync
date: 2026-10-27
start: 15:00
end: 15:30
location: Zoom
```
"#;
        let note_id = NoteId::new("Projects/Planning").unwrap();
        vault.create(note_id.clone(), note_content).unwrap();

        let events = store.all(&vault);
        assert_eq!(events.len(), 2);

        assert_eq!(events[0].title, "Architecture Review");
        assert_eq!(events[0].date, d(27));
        assert_eq!(events[0].time_label(), "10:00 – 11:30");
        assert_eq!(events[0].location.as_deref(), Some("Conference Room B"));
        assert_eq!(events[0].description.as_deref(), Some("Review technical spec"));
        assert_eq!(events[0].id, note_id);

        assert_eq!(events[1].title, "Team Sync");
        assert_eq!(events[1].date, d(27));
        assert_eq!(events[1].time_label(), "15:00 – 15:30");
        assert_eq!(events[1].location.as_deref(), Some("Zoom"));
        assert_eq!(events[1].id, note_id);

        let on_27 = store.on(&vault, d(27));
        assert_eq!(on_27.len(), 2);
    }
}

