//! # mybrain-core
//!
//! UI-agnostic domain logic for MyBrain, an Obsidian-like personal knowledge base.
//!
//! The crate is organised around a [`Vault`]: a folder of Markdown files on disk.
//! Everything else is a *view* over the vault:
//!
//! | Module          | Responsibility                                              |
//! |-----------------|-------------------------------------------------------------|
//! | [`note`]        | The [`Note`] type and its stable identifier [`NoteId`].     |
//! | [`frontmatter`] | Minimal `key: value` front-matter parsing / writing.        |
//! | [`links`]       | Parsing `[[wiki links]]`.                                   |
//! | [`markdown`]    | Markdown subset parser used by the preview.                 |
//! | [`index`]       | Forward links, backlinks and unresolved links.              |
//! | [`vault`]       | Loading, creating, saving, renaming and deleting notes.     |
//! | [`daily`]       | Daily notes (one note per day).                             |
//! | [`quick`]       | Quick capture notes.                                        |
//! | [`events`]      | Calendar events stored as notes with front-matter.          |
//!
//! Nothing in this crate depends on a GUI, which keeps it fast to test and
//! allows alternative front-ends (CLI, TUI, ...) to reuse it.

pub mod daily;
pub mod error;
pub mod events;
pub mod frontmatter;
pub mod index;
pub mod links;
pub mod markdown;
pub mod note;
pub mod quick;
pub mod vault;

pub use daily::DailyNotes;
pub use error::{Error, Result};
pub use events::{Event, EventDraft, EventStore};
pub use index::LinkIndex;
pub use links::WikiLink;
pub use note::{Note, NoteId};
pub use quick::QuickNotes;
pub use vault::Vault;
