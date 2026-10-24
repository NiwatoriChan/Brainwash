//! Daily notes: one note per calendar day, e.g. `Daily/2026-10-24.md`.

use std::collections::BTreeSet;

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::note::NoteId;
use crate::vault::Vault;

/// Daily notes configuration. Serializable so the app can persist it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DailyNotes {
    /// Folder (relative to the vault root) where daily notes live.
    pub folder: String,
    /// `chrono` format string used for the file name.
    pub date_format: String,
    /// Template for new daily notes. Supports `{{date}}`, `{{date:FORMAT}}`
    /// and `{{title}}` placeholders.
    pub template: String,
}

impl Default for DailyNotes {
    fn default() -> Self {
        Self {
            folder: "Daily".into(),
            date_format: "%Y-%m-%d".into(),
            template: "# {{date:%A, %B %-d, %Y}}\n\n## Notes\n\n".into(),
        }
    }
}

impl DailyNotes {
    /// Note id for `date`.
    pub fn id_for(&self, date: NaiveDate) -> NoteId {
        let name = date.format(&self.date_format).to_string();
        let raw = if self.folder.trim().is_empty() {
            name
        } else {
            format!("{}/{}", self.folder, name)
        };
        // The defaults always produce a valid id; fall back to ISO if a custom
        // format yields something unusable.
        NoteId::new(&raw).unwrap_or_else(|_| {
            NoteId::new(&date.format("%Y-%m-%d").to_string()).expect("ISO date is a valid id")
        })
    }

    /// Returns the date if `id` is a daily note.
    pub fn date_of(&self, id: &NoteId) -> Option<NaiveDate> {
        let folder = self.folder.trim().trim_matches('/');
        let in_folder = match id.folder() {
            Some(f) => f == folder,
            None => folder.is_empty(),
        };
        if !in_folder {
            return None;
        }
        NaiveDate::parse_from_str(id.name(), &self.date_format).ok()
    }

    /// Expands the template for `date`.
    pub fn render_template(&self, date: NaiveDate) -> String {
        expand_placeholders(&self.template, date, &date.format(&self.date_format).to_string())
    }

    /// Returns the id of the daily note for `date`, creating it from the
    /// template if it does not exist yet.
    pub fn open_or_create(&self, vault: &mut Vault, date: NaiveDate) -> Result<NoteId> {
        let id = self.id_for(date);
        if !vault.contains(&id) {
            vault.create(id.clone(), &self.render_template(date))?;
        }
        Ok(id)
    }

    /// Convenience for [`Self::open_or_create`] with today's date.
    pub fn open_today(&self, vault: &mut Vault) -> Result<NoteId> {
        self.open_or_create(vault, today())
    }

    /// All dates that have a daily note.
    pub fn existing_dates(&self, vault: &Vault) -> BTreeSet<NaiveDate> {
        vault.notes().filter_map(|n| self.date_of(&n.id)).collect()
    }
}

/// Today's date in the local time zone.
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn expand_placeholders(template: &str, date: NaiveDate, title: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let tag = after[..end].trim();
        match tag {
            "date" => out.push_str(&date.format("%Y-%m-%d").to_string()),
            "title" => out.push_str(title),
            _ => match tag.strip_prefix("date:") {
                Some(fmt) => out.push_str(&date.format(fmt.trim()).to_string()),
                None => out.push_str(&rest[start..start + 2 + end + 2]),
            },
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
    }

    #[test]
    fn id_and_date_roundtrip() {
        let daily = DailyNotes::default();
        let id = daily.id_for(date());
        assert_eq!(id.as_str(), "Daily/2026-10-24");
        assert_eq!(daily.date_of(&id), Some(date()));
        assert_eq!(daily.date_of(&NoteId::new("Other/2026-10-24").unwrap()), None);
    }

    #[test]
    fn template_expansion() {
        let daily = DailyNotes {
            template: "{{date}} | {{date:%d/%m}} | {{title}} | {{unknown}}".into(),
            ..Default::default()
        };
        assert_eq!(
            daily.render_template(date()),
            "2026-10-24 | 24/10 | 2026-10-24 | {{unknown}}"
        );
    }

    #[test]
    fn open_or_create_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::open(dir.path()).unwrap();
        let daily = DailyNotes::default();
        let id = daily.open_or_create(&mut vault, date()).unwrap();
        vault.save(&id, "edited").unwrap();
        daily.open_or_create(&mut vault, date()).unwrap();
        assert_eq!(vault.get(&id).unwrap().content, "edited");
        assert_eq!(daily.existing_dates(&vault).len(), 1);
    }
}
