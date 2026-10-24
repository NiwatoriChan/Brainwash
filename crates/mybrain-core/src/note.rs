//! The [`Note`] type and its identifier.

use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use crate::error::{Error, Result};
use crate::frontmatter::Frontmatter;

/// File extension used for notes.
pub const NOTE_EXTENSION: &str = "md";

/// Stable identifier of a note: its path relative to the vault root, using `/`
/// as separator and **without** the `.md` extension.
///
/// Examples: `"Inbox"`, `"Daily/2026-10-24"`, `"Projects/Rust/egui"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(String);

impl NoteId {
    /// Builds an id from a user supplied string such as `"Projects/My idea"`.
    ///
    /// Leading/trailing slashes and whitespace are trimmed, a trailing `.md`
    /// is stripped, and path traversal (`..`) is rejected.
    pub fn new(raw: &str) -> Result<Self> {
        let trimmed = raw.trim().trim_matches('/');
        let trimmed = trimmed
            .strip_suffix(&format!(".{NOTE_EXTENSION}"))
            .unwrap_or(trimmed);

        if trimmed.is_empty() {
            return Err(Error::InvalidName(raw.to_owned()));
        }

        let mut parts = Vec::new();
        for part in trimmed.split(['/', '\\']) {
            let part = part.trim();
            if part.is_empty() || part == "." {
                continue;
            }
            if part == ".." || part.contains(INVALID_CHARS) {
                return Err(Error::InvalidName(raw.to_owned()));
            }
            parts.push(part);
        }

        if parts.is_empty() {
            return Err(Error::InvalidName(raw.to_owned()));
        }
        Ok(Self(parts.join("/")))
    }

    /// Builds an id from a file path relative to the vault root.
    pub fn from_relative_path(path: &Path) -> Result<Self> {
        let mut parts = Vec::new();
        for component in path.with_extension("").components() {
            match component {
                Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
                _ => return Err(Error::InvalidName(path.display().to_string())),
            }
        }
        Self::new(&parts.join("/"))
    }

    /// The id as a `/`-separated string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The last path segment, i.e. what Obsidian displays as the note title.
    pub fn name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }

    /// The containing folder (`None` for notes at the vault root).
    pub fn folder(&self) -> Option<&str> {
        self.0.rsplit_once('/').map(|(folder, _)| folder)
    }

    /// Relative path of the backing file, e.g. `Daily/2026-10-24.md`.
    pub fn relative_path(&self) -> PathBuf {
        let mut path: PathBuf = self.0.split('/').collect();
        path.set_extension(NOTE_EXTENSION);
        path
    }

    /// Case-insensitive key used for link resolution.
    pub(crate) fn link_key(&self) -> String {
        self.0.to_lowercase()
    }
}

/// Characters that are not allowed inside a note name (they break links or
/// file systems).
const INVALID_CHARS: &[char] = &['[', ']', '|', '#', '^', ':', '*', '?', '"', '<', '>'];

impl fmt::Display for NoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A Markdown note loaded in memory.
#[derive(Debug, Clone)]
pub struct Note {
    pub id: NoteId,
    /// Full file content, including front-matter.
    pub content: String,
    /// Last modification time on disk (if known).
    pub modified: Option<SystemTime>,
}

impl Note {
    pub fn new(id: NoteId, content: impl Into<String>) -> Self {
        Self {
            id,
            content: content.into(),
            modified: None,
        }
    }

    /// Display title (the file name without extension).
    pub fn title(&self) -> &str {
        self.id.name()
    }

    /// Parsed front-matter, if any.
    pub fn frontmatter(&self) -> Frontmatter {
        Frontmatter::parse(&self.content).0
    }

    /// Content with the front-matter block removed.
    pub fn body(&self) -> &str {
        Frontmatter::parse(&self.content).1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_ids() {
        assert_eq!(NoteId::new(" /Projects//Idea.md ").unwrap().as_str(), "Projects/Idea");
        assert_eq!(NoteId::new("a\\b").unwrap().as_str(), "a/b");
    }

    #[test]
    fn rejects_bad_ids() {
        assert!(NoteId::new("").is_err());
        assert!(NoteId::new("../etc/passwd").is_err());
        assert!(NoteId::new("foo[[bar").is_err());
        assert!(NoteId::new("a|b").is_err());
    }

    #[test]
    fn name_and_folder() {
        let id = NoteId::new("Daily/2026-10-24").unwrap();
        assert_eq!(id.name(), "2026-10-24");
        assert_eq!(id.folder(), Some("Daily"));
        assert_eq!(id.relative_path(), PathBuf::from("Daily/2026-10-24.md"));
    }

    #[test]
    fn from_path_roundtrip() {
        let id = NoteId::from_relative_path(Path::new("a/b/c.md")).unwrap();
        assert_eq!(id.as_str(), "a/b/c");
    }
}
