//! Error type shared by the whole core crate.

use std::path::PathBuf;

/// Convenient alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Everything that can go wrong while manipulating a vault.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("note already exists: {0}")]
    AlreadyExists(String),

    #[error("note not found: {0}")]
    NotFound(String),

    #[error("invalid note name: {0:?}")]
    InvalidName(String),

    #[error("invalid event: {0}")]
    InvalidEvent(String),
}

impl Error {
    /// Helper to wrap an [`std::io::Error`] together with the path that caused it.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
