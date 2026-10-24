//! Quick notes: capture a thought in one keystroke, file it later.
//!
//! Each capture becomes its own note in the quick-notes folder (so it can be
//! linked, moved or renamed like any other note) and is optionally logged as
//! a link in today's daily note.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::daily::DailyNotes;
use crate::error::{Error, Result};
use crate::frontmatter::Frontmatter;
use crate::note::{Note, NoteId};
use crate::vault::Vault;

const KIND: &str = "quick";
const MAX_TITLE_CHARS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QuickNotes {
    /// Folder where quick notes are stored.
    pub folder: String,
    /// Also add a `- HH:MM [[link]]` line to today's daily note.
    pub log_to_daily: bool,
}

impl Default for QuickNotes {
    fn default() -> Self {
        Self {
            folder: "Quick Notes".into(),
            log_to_daily: true,
        }
    }
}

impl QuickNotes {
    /// Stores `text` as a new quick note and returns its id.
    pub fn capture(
        &self,
        vault: &mut Vault,
        daily: &DailyNotes,
        text: &str,
        now: NaiveDateTime,
    ) -> Result<NoteId> {
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::InvalidName("empty quick note".into()));
        }

        let title = title_from(text);
        let base = format!("{}/{} {}", self.folder, now.format("%Y-%m-%d %H%M"), title);
        let id = vault.unique_id(&NoteId::new(&base)?);

        let mut fm = Frontmatter::new();
        fm.set("type", KIND)
            .set("created", now.format("%Y-%m-%dT%H:%M").to_string());
        vault.create(id.clone(), &format!("{}{text}\n", fm.render()))?;

        if self.log_to_daily {
            let daily_id = daily.open_or_create(vault, now.date())?;
            let line = format!("- {} [[{}|{}]]", now.format("%H:%M"), id, title);
            vault.append(&daily_id, &line)?;
        }
        Ok(id)
    }

    /// Most recent quick notes first.
    pub fn recent<'a>(&self, vault: &'a Vault, limit: usize) -> Vec<&'a Note> {
        let mut notes: Vec<&Note> = vault.notes().filter(|n| is_quick_note(n)).collect();
        notes.sort_by(|a, b| {
            let created = |n: &Note| n.frontmatter().get("created").map(str::to_owned);
            created(b).cmp(&created(a))
        });
        notes.truncate(limit);
        notes
    }
}

/// `true` if the note was created via quick capture.
pub fn is_quick_note(note: &Note) -> bool {
    note.frontmatter().get("type") == Some(KIND)
}

/// Builds a file-name-safe title from the first line of `text`.
fn title_from(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or_default();
    let cleaned: String = first_line
        .chars()
        .map(|c| if c.is_alphanumeric() || " -_.,'!()".contains(c) { c } else { ' ' })
        .collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let mut title = String::new();
    for word in words {
        if title.chars().count() + word.chars().count() + 1 > MAX_TITLE_CHARS {
            break;
        }
        if !title.is_empty() {
            title.push(' ');
        }
        title.push_str(word);
    }
    if title.is_empty() {
        "Note".into()
    } else {
        title
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 24)
            .unwrap()
            .and_hms_opt(17, 6, 0)
            .unwrap()
    }

    #[test]
    fn titles_are_sanitised() {
        assert_eq!(title_from("Buy [[milk]] & eggs\nsecond line"), "Buy milk eggs");
        assert_eq!(title_from("###"), "Note");
        assert!(title_from(&"word ".repeat(50)).chars().count() <= MAX_TITLE_CHARS);
    }

    #[test]
    fn capture_creates_note_and_logs_to_daily() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::open(dir.path()).unwrap();
        let daily = DailyNotes::default();
        let quick = QuickNotes::default();

        let id = quick.capture(&mut vault, &daily, "Call Bob about [[Project X]]", now()).unwrap();
        assert_eq!(id.as_str(), "Quick Notes/2026-10-24 1706 Call Bob about Project X");

        let note = vault.get(&id).unwrap();
        assert!(is_quick_note(note));
        assert!(note.body().contains("[[Project X]]"));

        let daily_note = vault.get(&daily.id_for(now().date())).unwrap();
        assert!(daily_note.content.contains("- 17:06 [[Quick Notes/2026-10-24 1706 Call Bob"));

        // Same text, same minute -> unique id.
        let id2 = quick.capture(&mut vault, &daily, "Call Bob about [[Project X]]", now()).unwrap();
        assert_ne!(id, id2);
        assert_eq!(quick.recent(&vault, 10).len(), 2);
    }

    #[test]
    fn rejects_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::open(dir.path()).unwrap();
        let r = QuickNotes::default().capture(&mut vault, &DailyNotes::default(), "  ", now());
        assert!(r.is_err());
    }
}
