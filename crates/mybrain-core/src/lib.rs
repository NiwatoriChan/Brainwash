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

pub mod canvas;
pub mod daily;
pub mod error;
pub mod events;
pub mod events_bus;
pub mod frontmatter;
pub mod index;
pub mod links;
pub mod markdown;
pub mod note;
pub mod drawing;
pub mod drawing_codec;
pub use drawing as noteful;
pub use drawing_codec as noteful_codec;
pub mod quick;
pub mod settings;
pub mod vault;

pub use canvas::{CanvasEdge, CanvasFile, CanvasNode};
pub use daily::DailyNotes;
pub use error::{Error, Result};
pub use events::{Event, EventDraft, EventStore};
pub use events_bus::{AppEvent, EventBus};
pub use frontmatter::Frontmatter;
pub use index::LinkIndex;
pub use links::WikiLink;
pub use note::{Note, NoteId};
pub use drawing::{
    recognize_stroke_shape, BackgroundTemplate, NotefulDocument, NotefulLayer, NotefulOutlineItem,
    NotefulPage, NotefulShape, NotefulTape, NotefulTextBox, PaperColor, PaperOrientation,
    PdfBackground, PenType, RgbaColor, ShapeKind, StrokePoint, StrokeToolKind, VectorStroke,
};
pub use drawing_codec::{DrawingCodec, DrawingCodec as NotefulCodec};
pub type DrawingDocument = NotefulDocument;
pub type DrawingPage = NotefulPage;
pub type DrawingLayer = NotefulLayer;
pub type DrawingTape = NotefulTape;
pub type DrawingShape = NotefulShape;
pub type DrawingTextBox = NotefulTextBox;
pub type DrawingPdfBackground = PdfBackground;
pub use quick::QuickNotes;
pub use settings::AppSettings;
pub use vault::Vault;
