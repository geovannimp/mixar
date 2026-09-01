//! SeaORM sync connection setup and entity-first schema sync.

use std::path::Path;
use std::sync::Mutex;

use library_core::{LibraryError, Result};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Statement};

/// Wrapper so `Db` can satisfy `Library: Send + Sync`.
///
/// SeaORM sync's `DatabaseConnection` is `!Send` because of an optional metric
/// callback type; we never register one, and all access is serialized through
/// the mutex around the rusqlite-backed connection.
pub(crate) struct SyncConnection(DatabaseConnection);

unsafe impl Send for SyncConnection {}
unsafe impl Sync for SyncConnection {}

pub struct Db {
    conn: Mutex<SyncConnection>,
}

impl Db {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        Self::connect(&url)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::connect("sqlite::memory:")
    }

    fn connect(url: &str) -> Result<Self> {
        let mut opts = ConnectOptions::new(url.to_string());
        // SeaORM logs queries (with bound parameters) via the `sea_orm` target when
        // RUST_LOG includes `sea_orm=debug`; keep sqlx's own logging off.
        opts.max_connections(1).sqlx_logging(false);

        let conn = Database::connect(opts).map_err(db_err)?;
        sync_schema(&conn).map_err(db_err)?;
        Ok(Self {
            conn: Mutex::new(SyncConnection(conn)),
        })
    }

    pub(crate) fn conn(&self) -> Result<std::sync::MutexGuard<'_, SyncConnection>> {
        self.conn.lock().map_err(|_| LibraryError::Backend {
            backend: "library",
            message: "library database lock poisoned".into(),
        })
    }
}

impl SyncConnection {
    pub(crate) fn as_connection(&self) -> &DatabaseConnection {
        &self.0
    }
}

pub fn open(db_path: &Path) -> Result<Db> {
    Db::open(db_path)
}

pub fn open_in_memory() -> Result<Db> {
    Db::open_in_memory()
}

fn sync_schema(conn: &DatabaseConnection) -> std::result::Result<(), DbErr> {
    conn.execute_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "PRAGMA foreign_keys = ON;".to_string(),
    ))?;
    // journal_mode returns a row; execute_raw would fail with "did you mean to call query?"
    conn.query_one_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "PRAGMA journal_mode = WAL;".to_string(),
    ))?;

    rename_legacy_collection_tracks_table(conn)?;

    conn.get_schema_registry("library::entity::*").sync(conn)
}

fn rename_legacy_collection_tracks_table(
    conn: &DatabaseConnection,
) -> std::result::Result<(), DbErr> {
    let has_old = conn
        .query_one_raw(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type='table' AND name='collection_tracks'",
            [],
        ))?
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or(0);
    if has_old == 0 {
        return Ok(());
    }
    conn.execute_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "ALTER TABLE collection_tracks RENAME TO collection_entries".to_string(),
    ))?;
    Ok(())
}

pub fn db_err(err: DbErr) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_creates_all_tables() {
        let db = open_in_memory().unwrap();
        for name in [
            "tracks",
            "collections",
            "collection_entries",
            "track_analysis",
            "track_waveform",
            "track_hot_cue",
            "track_loop",
            "sampler_bank",
            "sampler_slot",
        ] {
            let count: i64 = db
                .conn()
                .unwrap()
                .as_connection()
                .query_one_raw(Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Sqlite,
                    "SELECT COUNT(*) AS count FROM sqlite_master WHERE type='table' AND name=?",
                    [name.into()],
                ))
                .map_err(db_err)
                .unwrap()
                .unwrap()
                .try_get("", "count")
                .unwrap();
            assert_eq!(count, 1, "missing table {name}");
        }
    }

    #[test]
    fn open_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("nested").join("library.db");
        let db = open(db_path.as_path()).unwrap();
        drop(db);
        assert!(db_path.exists());
    }
}
