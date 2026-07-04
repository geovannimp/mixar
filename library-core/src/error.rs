//! Error types for library backends.

use std::path::PathBuf;

/// Errors returned by library backends.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// Track or collection was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Path does not exist or is not accessible.
    #[error("path not found: {0}")]
    PathNotFound(PathBuf),

    /// Path is not a supported audio file.
    #[error("unsupported audio file: {0}")]
    UnsupportedFile(PathBuf),

    /// Path is not a directory (expected for folder collections).
    #[error("not a directory: {0}")]
    NotADirectory(PathBuf),

    /// Operation requires a different collection type.
    #[error("expected {expected} collection, got {got}")]
    WrongCollectionType {
        /// Expected type name (`"folder"` or `"playlist"`).
        expected: &'static str,
        /// Actual type name.
        got: &'static str,
    },

    /// Operation is not supported by this backend.
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),

    /// I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Backend-specific failure.
    #[error("{backend}: {message}")]
    Backend {
        /// Backend name (e.g. `"library"`, `"rekordbox"`).
        backend: &'static str,
        /// Human-readable message.
        message: String,
    },
}

/// Result alias for library operations.
pub type Result<T> = std::result::Result<T, LibraryError>;
