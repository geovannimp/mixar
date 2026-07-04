//! Database schema and connection helpers.

use std::path::Path;

use library_core::{LibraryError, Result};
use rusqlite::Connection;

pub(crate) fn open_connection(db_path: &Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let conn = Connection::open(db_path).map_err(db_err)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

pub(crate) fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory().map_err(db_err)?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        ",
    )
    .map_err(db_err)?;
    Ok(())
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tracks (
            id TEXT PRIMARY KEY NOT NULL,
            source_type TEXT NOT NULL,
            source_ref TEXT NOT NULL,
            provider TEXT,
            title TEXT,
            artist TEXT,
            album TEXT,
            genre TEXT,
            bpm REAL,
            key TEXT,
            duration_secs REAL,
            sample_rate INTEGER,
            channels INTEGER,
            bitrate_kbps INTEGER,
            added_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE (source_type, source_ref)
        );

        CREATE INDEX IF NOT EXISTS idx_tracks_source_ref ON tracks(source_ref);
        CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist);
        CREATE INDEX IF NOT EXISTS idx_tracks_title ON tracks(title);

        CREATE TABLE IF NOT EXISTS collections (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            collection_type TEXT NOT NULL,
            sortable INTEGER NOT NULL DEFAULT 1,
            fs_path TEXT,
            UNIQUE (fs_path)
        );

        CREATE TABLE IF NOT EXISTS collection_tracks (
            collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
            track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
            position INTEGER,
            PRIMARY KEY (collection_id, track_id)
        );

        CREATE INDEX IF NOT EXISTS idx_collection_tracks_track
            ON collection_tracks(track_id);
        CREATE INDEX IF NOT EXISTS idx_collection_tracks_collection_pos
            ON collection_tracks(collection_id, position);
        ",
    )
    .map_err(db_err)?;
    Ok(())
}

pub(crate) fn db_err(err: rusqlite::Error) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_creates_all_tables() {
        let conn = open_in_memory().unwrap();
        for name in ["tracks", "collections", "collection_tracks"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [name],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "missing table {name}");
        }
    }

    #[test]
    fn open_connection_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("nested").join("library.db");
        let conn = open_connection(&db_path).unwrap();
        drop(conn);
        assert!(db_path.exists());
    }
}
