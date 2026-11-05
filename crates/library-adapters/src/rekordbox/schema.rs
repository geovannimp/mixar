//! Rekordbox 6/7 `master.db` schema detection.
//!
//! The adapter reads a fixed slice of the library: `djmdContent` (tracks),
//! `djmdPlaylist` + `djmdSongPlaylist` (lists), `djmdCue` (cues and loops) and
//! the lookup tables behind the foreign keys. Everything else — history, My
//! Tag, active censors — is outside this adapter's scope.
//!
//! Tables a given Rekordbox version omits are detected rather than assumed, so
//! a library without colour labels or cue rows still imports.

use std::collections::HashSet;

use library_core::{LibraryError, Result};
use rusqlite::Connection;

/// A `LibraryError::Backend` tagged with this adapter's name.
pub(crate) fn backend(message: impl Into<String>) -> LibraryError {
    LibraryError::Backend {
        backend: "rekordbox",
        message: message.into(),
    }
}

/// Rekordbox 5's database file name; the format differs and is not supported.
pub(crate) const REKORDBOX5_FILE: &str = "datafile.edb";

/// Tables present in this library.
#[derive(Debug)]
pub(crate) struct Schema {
    /// `djmdArtist` — track artists.
    pub artists: bool,
    /// `djmdAlbum` — albums.
    pub albums: bool,
    /// `djmdGenre` — genres.
    pub genres: bool,
    /// `djmdColor` — track colour labels.
    pub colors: bool,
    /// `djmdKey` — musical keys (always present in practice; detected so a
    /// trimmed library does not fail the whole import).
    pub keys: bool,
    /// `djmdCue` — hot cues, memory cues and loops.
    pub cues: bool,
}

impl Schema {
    /// Every table the adapter reads, as a name set.
    pub(crate) fn table_names(conn: &Connection) -> Result<HashSet<String>> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .map_err(|e| backend(format!("prepare table query: {e}")))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| backend(format!("query tables: {e}")))?;
        let mut names = HashSet::new();
        for row in rows {
            names.insert(row.map_err(|e| backend(format!("read table name: {e}")))?);
        }
        Ok(names)
    }
}

/// How to reject a file that is not a Rekordbox 6/7 library.
pub(crate) fn unsupported(message: &'static str) -> LibraryError {
    LibraryError::Unsupported(message)
}

/// Detect the library schema, rejecting anything that is not a Rekordbox 6/7
/// `master.db`.
pub(crate) fn detect(conn: &Connection) -> Result<Schema> {
    let tables = Schema::table_names(conn)?;
    for required in ["djmdContent", "djmdPlaylist", "djmdSongPlaylist"] {
        if !tables.contains(required) {
            return Err(unsupported(
                "not a Rekordbox 6/7 master.db (no djmdContent/djmdPlaylist tables)",
            ));
        }
    }

    Ok(Schema {
        artists: tables.contains("djmdArtist"),
        albums: tables.contains("djmdAlbum"),
        genres: tables.contains("djmdGenre"),
        colors: tables.contains("djmdColor"),
        keys: tables.contains("djmdKey"),
        cues: tables.contains("djmdCue"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn_with(tables: &[&str]) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        for table in tables {
            conn.execute_batch(&format!("CREATE TABLE {table} (ID TEXT);"))
                .unwrap();
        }
        conn
    }

    #[test]
    fn detects_a_full_library() {
        let conn = conn_with(&[
            "djmdContent",
            "djmdPlaylist",
            "djmdSongPlaylist",
            "djmdCue",
            "djmdColor",
            "djmdKey",
        ]);
        let schema = detect(&conn).unwrap();
        assert!(schema.cues && schema.colors && schema.keys);
        assert!(!schema.artists && !schema.albums && !schema.genres);

        let trimmed = conn_with(&["djmdContent", "djmdPlaylist", "djmdSongPlaylist"]);
        let schema = detect(&trimmed).unwrap();
        assert!(!schema.cues && !schema.colors && !schema.keys);
    }

    #[test]
    fn rejects_a_database_without_rekordbox_tables() {
        let conn = conn_with(&["library", "track_locations"]);
        match detect(&conn) {
            Err(LibraryError::Unsupported(message)) => {
                assert!(message.contains("master.db"), "{message}");
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }
}
