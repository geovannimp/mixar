//! SeaORM sync connection setup and entity-first schema sync.

use std::path::Path;
use std::sync::Mutex;

use library_core::{LibraryError, Result};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbErr,
    Statement,
};

pub const SEAQL_MIGRATIONS_TABLE: &str = "seaql_migrations";
pub const TRACKS_TABLE: &str = "tracks";
pub const TRACK_ANALYSIS_TABLE: &str = "track_analysis";
pub const TRACK_BEAT_GRID_TABLE: &str = "track_beat_grid";
#[allow(dead_code)]
pub const TRACK_BEAT_SEGMENT_TABLE: &str = "track_beat_segment";

/// Wrapper so `Db` can satisfy `Library: Send + Sync`.
///
/// SeaORM sync's `DatabaseConnection` is `!Send` because of an optional metric
/// callback type; we never register one, and all access is serialized through
/// the mutex around the rusqlite-backed connection.
pub struct SyncConnection(DatabaseConnection);

unsafe impl Send for SyncConnection {}
unsafe impl Sync for SyncConnection {}

pub struct Db {
    conn: Mutex<SyncConnection>,
    /// Thread currently inside [`Db::transaction`], so its own re-entry reports
    /// the mistake while another thread simply waits its turn.
    tx_owner: Mutex<Option<std::thread::ThreadId>>,
    tx_free: std::sync::Condvar,
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
        Self::connect_with(url, crate::migrations::run_pending_migrations)
    }

    /// Bootstrap `url`, hand the connection to `phase`, then wrap it.
    fn connect_with(
        url: &str,
        phase: fn(&sea_orm::DatabaseConnection) -> Result<()>,
    ) -> Result<Self> {
        let conn = Self::open_connection(url)?;
        phase(conn.as_connection())?;
        Ok(Self::from_connection(conn))
    }

    /// Shared connection bootstrap: options, schema sync, and the Send+Sync
    /// wrapper. Migrations are layered on top by [`Self::connect`].
    fn open_connection(url: &str) -> Result<SyncConnection> {
        let mut opts = ConnectOptions::new(url.to_string());
        // SeaORM logs queries (with bound parameters) via the `sea_orm` target when
        // RUST_LOG includes `sea_orm=debug`; keep sqlx's own logging off.
        opts.max_connections(1).sqlx_logging(false);

        let conn = Database::connect(opts).map_err(db_err)?;
        // Each pragma answers with a row, so it is queried, not executed.
        for pragma in [
            "PRAGMA foreign_keys = ON;",
            // Another process opening the same file may hold the migration
            // write lock; wait for it instead of failing with SQLITE_BUSY.
            "PRAGMA busy_timeout = 5000;",
            // Reading `journal_mode` returns the mode it settled on.
            "PRAGMA journal_mode = WAL;",
        ] {
            Self::run_pragma(&conn, pragma)?;
        }
        Ok(SyncConnection(conn))
    }

    /// Open an in-memory database with the schema pragmas and only the schema
    /// phase of the chain.
    ///
    /// Tests use it to seed a legacy layout before the data phase runs.
    #[allow(dead_code)]
    pub fn open_in_memory_unmigrated() -> Result<Self> {
        Self::connect_with("sqlite::memory:", crate::migrations::run_schema_migrations)
    }

    fn run_pragma(conn: &DatabaseConnection, sql: &str) -> Result<()> {
        conn.query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            sql.to_string(),
        ))
        .map(|_| ())
        .map_err(db_err)
    }

    /// Wrap a bootstrapped connection in the shared handle every constructor uses.
    fn from_connection(conn: SyncConnection) -> Self {
        Self {
            conn: Mutex::new(conn),
            tx_owner: Mutex::new(None),
            tx_free: std::sync::Condvar::new(),
        }
    }

    pub fn conn(&self) -> Result<std::sync::MutexGuard<'_, SyncConnection>> {
        Ok(lock(&self.conn))
    }

    /// Run `f` inside one `BEGIN IMMEDIATE` transaction on the single connection.
    ///
    /// The connection is acquired once, so `f` must use only the connection it
    /// is given; a nested call returns an error instead of deadlocking. A
    /// closure or commit error rolls the whole transaction back and the
    /// original error is returned, so a partially written import can never be
    /// observed.
    pub fn transaction<T>(&self, f: impl FnOnce(&DatabaseConnection) -> Result<T>) -> Result<T> {
        let me = std::thread::current().id();
        {
            let mut owner = lock(&self.tx_owner);
            while let Some(current) = *owner {
                if current == me {
                    return Err(LibraryError::Backend {
                        backend: "library",
                        message: "nested transactions are not supported".into(),
                    });
                }
                owner = self
                    .tx_free
                    .wait(owner)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            *owner = Some(me);
        }
        let mut slot = TransactionSlot {
            conn: &self.conn,
            owner: &self.tx_owner,
            free: &self.tx_free,
            finished: false,
        };
        let outcome = self.run_transaction(f);
        slot.finished = true;
        outcome
    }

    fn run_transaction<T>(&self, f: impl FnOnce(&DatabaseConnection) -> Result<T>) -> Result<T> {
        let guard = self.conn()?;
        let conn = guard.as_connection();
        begin(conn)?;
        match f(conn) {
            Ok(value) => match commit(conn) {
                Ok(()) => Ok(value),
                Err(err) => {
                    rollback(conn);
                    Err(err)
                }
            },
            Err(err) => {
                rollback(conn);
                Err(err)
            }
        }
    }
}

/// Take a lock, recovering one poisoned by a panic: the data behind it is still
/// valid, and an unwound transaction is rolled back before it is reused.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Releases the transaction slot on drop, rolling back a transaction that
/// unwound before anyone else may take it.
struct TransactionSlot<'a> {
    conn: &'a Mutex<SyncConnection>,
    owner: &'a Mutex<Option<std::thread::ThreadId>>,
    free: &'a std::sync::Condvar,
    finished: bool,
}

impl Drop for TransactionSlot<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let guard = lock(self.conn);
            rollback(guard.as_connection());
        }
        let mut owner = lock(self.owner);
        *owner = None;
        self.free.notify_one();
    }
}

pub fn begin(conn: &DatabaseConnection) -> Result<()> {
    conn.execute_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "BEGIN IMMEDIATE".to_string(),
    ))
    .map(|_| ())
    .map_err(db_err)
}

pub fn commit(conn: &DatabaseConnection) -> Result<()> {
    conn.execute_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "COMMIT".to_string(),
    ))
    .map(|_| ())
    .map_err(db_err)
}

pub fn rollback(conn: &DatabaseConnection) {
    let _ = conn.execute_raw(Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        "ROLLBACK".to_string(),
    ));
}

impl SyncConnection {
    pub fn as_connection(&self) -> &DatabaseConnection {
        &self.0
    }
}

pub fn open(db_path: &Path) -> Result<Db> {
    Db::open(db_path)
}

pub fn open_in_memory() -> Result<Db> {
    Db::open_in_memory()
}

#[allow(dead_code)]
pub fn open_in_memory_unmigrated() -> Result<Db> {
    Db::open_in_memory_unmigrated()
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
            "track_cue",
            "track_waveform",
            "track_stem",
            "track_hot_cue",
            "track_loop",
            "sampler_bank",
            "sampler_slot",
        ] {
            let count = count_rows(
                &db,
                format!(
                    "SELECT COUNT(*) AS count FROM sqlite_master \
                     WHERE type='table' AND name='{name}'"
                ),
            );
            assert_eq!(count, 1, "missing table {name}");
        }
    }

    /// Row count from a `SELECT COUNT(*) AS count …` statement.
    ///
    /// Every schema assertion asks the same shape of question; keeping the
    /// connection/lock/`try_get` boilerplate here means a change to it lands in
    /// one place instead of once per test.
    fn count_rows(db: &Db, sql: impl Into<String>) -> i64 {
        db.conn()
            .unwrap()
            .as_connection()
            .query_one_raw(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                sql.into(),
            ))
            .map_err(db_err)
            .unwrap()
            .unwrap()
            .try_get("", "count")
            .unwrap()
    }

    fn table_columns(db: &Db, table: &str) -> Vec<String> {
        db.conn()
            .unwrap()
            .as_connection()
            .query_all_raw(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                format!("PRAGMA table_info({table})"),
            ))
            .map_err(db_err)
            .unwrap()
            .into_iter()
            .map(|row| row.try_get::<String>("", "name").unwrap())
            .collect()
    }

    fn track_columns(db: &Db) -> Vec<String> {
        table_columns(db, "tracks")
    }

    /// The main cue table is created on an existing database by `sync`, with the
    /// two columns the cue store reads and writes.
    #[test]
    fn sync_creates_track_cue() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        // An older database: `sync` has to add `track_cue` on open.
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE tracks (
                    id TEXT PRIMARY KEY NOT NULL,
                    source_type TEXT NOT NULL,
                    source_ref TEXT NOT NULL,
                    added_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );",
            )
            .unwrap();
        }

        let db = open(&db_path).unwrap();
        let columns = table_columns(&db, "track_cue");
        for column in ["track_id", "cue_ms", "source", "updated_at"] {
            assert!(columns.iter().any(|name| name == column), "{column}");
        }

        // `track_id` is the primary key, so SQLite builds the index every cue
        // lookup (`find_by_id`) uses; the table needs no separate one.
        let index_count = count_rows(
            &db,
            "SELECT COUNT(*) AS count FROM sqlite_master \
             WHERE type='index' AND tbl_name='track_cue'",
        );
        assert!(index_count >= 1, "the primary key must back cue lookups");
    }

    #[test]
    fn fresh_tracks_table_has_the_musicbrainz_style_columns() {
        let db = open_in_memory().unwrap();
        let columns = track_columns(&db);
        for column in [
            "rating",
            "comment",
            "year",
            "track_number",
            "disc_number",
            "color_label",
            "metadata_edited",
        ] {
            assert!(columns.iter().any(|name| name == column), "{column}");
        }
    }

    #[test]
    fn sync_adds_new_track_columns_to_an_existing_database() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        {
            // The `tracks` schema before rating/year/comment/numbers/colour.
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE tracks (
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
                    duration_ms INTEGER,
                    sample_rate INTEGER,
                    channels INTEGER,
                    bitrate_kbps INTEGER,
                    replaygain_track_gain_db REAL,
                    isrc TEXT,
                    last_sampler_bank_id TEXT,
                    added_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );",
            )
            .unwrap();
        }

        let db = open(&db_path).unwrap();
        let columns = track_columns(&db);
        for column in [
            "rating",
            "comment",
            "year",
            "track_number",
            "disc_number",
            "color_label",
            "metadata_edited",
        ] {
            assert!(
                columns.iter().any(|name| name == column),
                "sync did not add {column}"
            );
        }
    }

    #[test]
    fn sync_adds_auto_saved_to_existing_track_loop() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE track_loop (
                    track_id TEXT NOT NULL,
                    slot_index INTEGER NOT NULL,
                    in_ms INTEGER NOT NULL,
                    out_ms INTEGER NOT NULL,
                    label TEXT,
                    color TEXT,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (track_id, slot_index)
                );",
            )
            .unwrap();
        }

        let db = open(&db_path).unwrap();
        let columns = table_columns(&db, "track_loop");
        assert!(
            columns.iter().any(|name| name == "auto_saved"),
            "sync did not add auto_saved: {columns:?}"
        );
    }

    #[test]
    fn open_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("nested").join("library.db");
        let db = open(db_path.as_path()).unwrap();
        drop(db);
        assert!(db_path.exists());
    }

    #[test]
    fn drops_legacy_four_path_track_stem_before_sync() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE track_stem (
                    track_id TEXT PRIMARY KEY NOT NULL,
                    backend TEXT NOT NULL,
                    source_fingerprint TEXT NOT NULL DEFAULT '',
                    format TEXT NOT NULL DEFAULT 'opus',
                    sample_rate INTEGER NOT NULL,
                    vocals_path TEXT NOT NULL,
                    drums_path TEXT NOT NULL,
                    bass_path TEXT NOT NULL,
                    other_path TEXT NOT NULL,
                    generated_at TEXT NOT NULL
                );
                INSERT INTO track_stem VALUES (
                    't1','backend','fp','opus',48000,'a','b','c','d','now'
                );",
            )
            .unwrap();
        }
        let db = open(db_path.as_path()).unwrap();
        let has_path = count_rows(
            &db,
            "SELECT COUNT(*) AS count FROM pragma_table_info('track_stem') WHERE name='path'",
        );
        assert_eq!(has_path, 1);
        let rows = count_rows(&db, "SELECT COUNT(*) AS count FROM track_stem");
        assert_eq!(rows, 0, "legacy stem rows should be dropped with the table");
    }

    #[test]
    fn concurrent_transactions_take_their_turn() {
        use std::sync::{Arc, Barrier};

        let db = Arc::new(Db::open_in_memory().unwrap());
        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let db = Arc::clone(&db);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                db.transaction(|_conn| Ok(()))
            }));
        }

        for handle in handles {
            handle
                .join()
                .unwrap()
                .expect("a second thread waits instead of reporting a nesting error");
        }
    }

    #[test]
    fn a_panicking_transaction_releases_the_connection() {
        let db = Db::open_in_memory().unwrap();

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = db.transaction(|_conn| -> Result<()> { panic!("boom") });
        }));
        assert!(panicked.is_err());

        db.transaction(|_conn| Ok(()))
            .expect("the next transaction runs");
    }

    #[test]
    fn nested_transaction_is_rejected_instead_of_deadlocking() {
        let db = Db::open_in_memory().unwrap();

        let error = db
            .transaction(|_conn| db.transaction(|_| Ok(())))
            .unwrap_err();

        assert!(
            error.to_string().contains("nested transactions"),
            "unexpected error: {error}"
        );
    }
}
