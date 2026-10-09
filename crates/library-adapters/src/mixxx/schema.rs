//! Mixxx database schema detection.
//!
//! Modern Mixxx (schema revision ≥ 3) stores `library.location` as an integer
//! foreign key to `track_locations.id`; older databases store the text path.
//! Optional metadata columns are detected so older databases still import.

use rusqlite::Connection;

use library_core::{LibraryError, Result};

/// A `LibraryError::Backend` tagged with this adapter's name.
pub(crate) fn backend(message: impl Into<String>) -> LibraryError {
    LibraryError::Backend {
        backend: "mixxx",
        message: message.into(),
    }
}

/// Metadata columns read from the Mixxx `library` table, in query order.
const METADATA_COLUMNS: &[&str] = &[
    "title",
    "artist",
    "album",
    "genre",
    "bpm",
    "duration",
    "samplerate",
    "channels",
    "bitrate",
    "replaygain",
    "key",
];

pub(crate) struct LibrarySchema {
    /// `library.location` is an integer FK to `track_locations.id` (modern);
    /// otherwise it is the text path (legacy).
    location_is_integer: bool,
    has_mixxx_deleted: bool,
    columns: Vec<String>,
}

impl LibrarySchema {
    fn has(&self, name: &str) -> bool {
        self.columns.iter().any(|column| column == name)
    }

    /// `SELECT` list for the shared track query, aliasing to fixed names.
    pub(crate) fn metadata_select(&self) -> String {
        let mut parts = vec!["l.id AS id".to_string(), "tl.location AS path".to_string()];
        for column in METADATA_COLUMNS {
            if self.has(column) {
                parts.push(format!("l.{column} AS {column}"));
            } else {
                parts.push(format!("NULL AS {column}"));
            }
        }
        parts.join(", ")
    }

    pub(crate) fn track_join(&self) -> &'static str {
        if self.location_is_integer {
            "JOIN track_locations tl ON tl.id = l.location"
        } else {
            "JOIN track_locations tl ON tl.location = l.location"
        }
    }

    pub(crate) fn deleted_filter(&self) -> &'static str {
        if self.has_mixxx_deleted {
            "WHERE COALESCE(l.mixxx_deleted, 0) = 0"
        } else {
            ""
        }
    }
}

pub(crate) fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |row| row.get(0),
        )
        .map_err(|e| backend(format!("inspect mixxx schema: {e}")))?;
    Ok(count > 0)
}

pub(crate) fn detect(conn: &Connection) -> Result<LibrarySchema> {
    if !table_exists(conn, "library")? || !table_exists(conn, "track_locations")? {
        return Err(backend(
            "not a Mixxx library database (missing library or track_locations)",
        ));
    }

    let mut stmt = conn
        .prepare("PRAGMA table_info(library)")
        .map_err(|e| backend(format!("inspect mixxx schema: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            let name: String = row.get(1)?;
            let ty: String = row.get(2)?;
            Ok((name, ty))
        })
        .map_err(|e| backend(format!("inspect mixxx schema: {e}")))?;

    let mut columns = Vec::new();
    let mut location_type = String::new();
    for row in rows {
        let (name, ty) = row.map_err(|e| backend(format!("inspect mixxx schema: {e}")))?;
        if name == "location" {
            location_type = ty;
        }
        columns.push(name);
    }

    if !columns.iter().any(|column| column == "location") {
        return Err(backend("unsupported Mixxx schema (no location column)"));
    }

    Ok(LibrarySchema {
        location_is_integer: location_type.to_ascii_uppercase().contains("INT"),
        has_mixxx_deleted: columns.iter().any(|column| column == "mixxx_deleted"),
        columns,
    })
}
