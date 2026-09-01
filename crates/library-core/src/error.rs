//! Error types for library backends.

use std::path::Path;

/// Basename for user-facing path errors (avoids leaking full filesystem paths).
pub fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "(unknown)".to_string())
}

/// Errors returned by library backends.
#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    /// Track or collection was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Path does not exist or is not accessible.
    #[error("path not found: {0}")]
    PathNotFound(String),

    /// Path is not a supported audio file.
    #[error("unsupported audio file: {0}")]
    UnsupportedFile(String),

    /// Path is not a directory (expected for folder collections).
    #[error("not a directory: {0}")]
    NotADirectory(String),

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn path_label_uses_basename_only() {
        let path = Path::new("/home/user/Music/secret/track.flac");
        assert_eq!(path_label(path), "track.flac");
    }

    #[test]
    fn path_label_unknown_when_no_file_name() {
        assert_eq!(path_label(Path::new("/")), "(unknown)");
    }
}
