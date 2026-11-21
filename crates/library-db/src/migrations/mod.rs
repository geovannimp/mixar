//! Versioned library migrations.
//!
//! The chain is applied by [`migrator`] on the connection the library keeps
//! using, before anything else touches the database. A step is a type: its
//! `VERSION` is the ledger key and its `up` runs inside the migrator's
//! transaction, so a failed step leaves neither schema changes nor a row.
//!
//! Two phases share one ledger:
//!
//! - the schema phase establishes the tables. The frozen baseline owns the
//!   shape; the steps around it repair layouts an older database already has
//!   (a renamed table, a cache table with an unusable shape, a primary key that
//!   has to be swapped), which plain `ALTER TABLE` cannot express.
//! - the data phase backfills rows once the final shape exists.
//!
//! Versions use the shipped spelling (no `m` prefix) and the ledger keeps
//! RFC3339 text, so opening a database written by another build neither
//! rewrites the ledger nor re-runs an applied step. Rows recorded by another
//! build are carried over untouched.

use library_core::{LibraryError, Result};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

pub use migrator::{APPLIED_AT_COLUMN, LEDGER_TABLE, VERSION_COLUMN};

/// Declare a step's [`migrator::Migration`] impl around the module's `up_impl`,
/// so the error-conversion contract lives in one place.
macro_rules! migration {
    ($step:ty, $version:expr) => {
        migration!($step, $version, migration_error);
    };
    ($step:ty, $version:expr, $kind:ident) => {
        impl migrator::Migration for $step {
            const VERSION: &'static str = $version;

            fn up(&self, conn: &sea_orm::DatabaseConnection) -> Result<(), migrator::Error> {
                up_impl(conn).map_err(|err| migrator::Error::message(migration!(@text $kind, err)))
            }
        }
    };
    (@text migration_error, $err:ident) => {
        $err.message
    };
    (@text db_error, $err:ident) => {
        $err.to_string()
    };
}

pub mod beat_grid_backfill;
pub mod canonical_track_facts;
pub mod grid_io;
pub mod legacy_collection_tracks;
pub mod legacy_track_stem_cache;
pub mod library_schema_baseline;
pub mod track_analysis_append_only;

/// Ledger versions, spelled once for tests and fixtures.
pub const LEGACY_COLLECTION_TRACKS_VERSION: &str = "2026111001_legacy_collection_tracks";
pub const LEGACY_TRACK_STEM_VERSION: &str = "2026111002_legacy_track_stem_cache";
pub const BASELINE_VERSION: &str = "2026111200_library_schema_baseline";
pub const BACKFILL_VERSION: &str = "2026111201_beat_grid_backfill";
pub const APPEND_ONLY_VERSION: &str = "2026111900_track_analysis_append_only";
pub const CANONICAL_FACTS_VERSION: &str = "2026111901_canonical_track_facts";

/// Applies every step the database is missing, schema first.
pub fn run_pending_migrations(conn: &DatabaseConnection) -> Result<()> {
    schema_chain()
        .up(conn)
        .and_then(|_| data_chain().up(conn))
        .map(|_| ())
        .map_err(library_backend_error)
}

/// Applies the schema phase only, leaving the data migrations pending.
///
/// Tests seed legacy rows between the two phases.
pub fn run_schema_migrations(conn: &DatabaseConnection) -> Result<()> {
    schema_chain()
        .up(conn)
        .map(|_| ())
        .map_err(library_backend_error)
}

/// Surface a failed chain the way every library entry point reports it.
fn library_backend_error(err: ::migrator::Error) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: err.to_string(),
    }
}

/// Layout repairs and table creation.
fn schema_chain() -> ::migrator::Migrator {
    ::migrator::Migrator::new()
        .push(legacy_collection_tracks::LegacyCollectionTracks)
        .push(legacy_track_stem_cache::LegacyTrackStemCache)
        .push(library_schema_baseline::Baseline)
        .push(track_analysis_append_only::TrackAnalysisAppendOnly)
}

/// Backfills that need the final schema.
fn data_chain() -> ::migrator::Migrator {
    ::migrator::Migrator::new()
        .push(beat_grid_backfill::BeatGridBackfill)
        .push(canonical_track_facts::CanonicalTrackFacts)
}

/// Failure inside one step. The runner adds the version when the error leaves
/// the step, so the message carries the same context in both places.
#[derive(Debug)]
pub struct MigrationError {
    pub version: String,
    pub message: String,
}

impl MigrationError {
    pub fn new(version: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "migration {}: {}", self.version, self.message)
    }
}

impl std::error::Error for MigrationError {}

/// Map any error into a migration failure tagged with `version`.
pub fn migration_error(version: &str, err: impl std::fmt::Display) -> MigrationError {
    MigrationError::new(version, err.to_string())
}

/// Run one migration statement, naming `version` when it fails.
pub fn execute(
    conn: &DatabaseConnection,
    version: &str,
    sql: impl Into<String>,
) -> std::result::Result<(), MigrationError> {
    conn.execute_raw(Statement::from_string(DatabaseBackend::Sqlite, sql.into()))
        .map(|_| ())
        .map_err(|err| migration_error(version, err))
}

/// Whether this database has a table named `table`.
pub fn table_exists(
    conn: &DatabaseConnection,
    version: &str,
    table: &str,
) -> std::result::Result<bool, MigrationError> {
    let row = conn
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = ?",
            [table.into()],
        ))
        .map_err(|err| migration_error(version, err))?;
    Ok(row
        .and_then(|result| result.try_get::<i64>("", "count").ok())
        .unwrap_or(0)
        != 0)
}

/// Every column of `table` with its primary-key position (0 when not a key).
pub fn table_columns(
    conn: &DatabaseConnection,
    version: &str,
    table: &str,
) -> std::result::Result<Vec<(String, i64)>, MigrationError> {
    conn.query_all_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT name, pk FROM pragma_table_info(?)",
        [table.into()],
    ))
    .map_err(|err| migration_error(version, err))?
    .into_iter()
    .map(|row| {
        let name: String = column_value(&row, "name", version)?;
        let pk: i64 = column_value(&row, "pk", version)?;
        Ok((name, pk))
    })
    .collect()
}

/// One column of a pragma row, tagged with the step's version on failure.
fn column_value<T: sea_orm::TryGetable>(
    row: &sea_orm::QueryResult,
    column: &str,
    version: &str,
) -> std::result::Result<T, MigrationError> {
    row.try_get("", column)
        .map_err(|err| migration_error(version, err))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use sea_orm::EntityName;

    fn ledger_rows(path: &Path) -> Vec<(String, rusqlite::types::Value)> {
        let conn = rusqlite::Connection::open(path).unwrap();
        let mut statement = conn
            .prepare(&format!(
                "SELECT {v}, {a} FROM {t} ORDER BY {v}",
                v = migrator::VERSION_COLUMN,
                a = migrator::APPLIED_AT_COLUMN,
                t = migrator::LEDGER_TABLE
            ))
            .unwrap();
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        rows.map(|row| row.unwrap()).collect()
    }

    fn table_count(path: &Path, table: &str) -> i64 {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn versions(path: &Path) -> Vec<String> {
        ledger_rows(path).into_iter().map(|(v, _)| v).collect()
    }

    const ALL_VERSIONS: [&str; 6] = [
        super::LEGACY_COLLECTION_TRACKS_VERSION,
        super::LEGACY_TRACK_STEM_VERSION,
        super::BASELINE_VERSION,
        super::BACKFILL_VERSION,
        super::APPEND_ONLY_VERSION,
        super::CANONICAL_FACTS_VERSION,
    ];

    /// A database that already ran the chain keeps its ledger exactly as it
    /// was: every row, including one another build added, survives a reopen.
    #[test]
    fn existing_ledger_is_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");

        crate::db::open(&path).unwrap();
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(&format!(
                "INSERT INTO {t} ({v}, {a}) \
                 VALUES ('2099010100_from_newer_build', '2030-01-01T00:00:00+00:00');",
                t = migrator::LEDGER_TABLE,
                v = migrator::VERSION_COLUMN,
                a = migrator::APPLIED_AT_COLUMN
            ))
            .unwrap();
        }
        let before = ledger_rows(&path);

        crate::db::open(&path).unwrap();

        assert_eq!(ledger_rows(&path), before, "the ledger is not rewritten");
        assert_eq!(
            table_count(&path, crate::entity::tracks::Entity.table_name()),
            1
        );
    }

    /// A ledger written by a newer build (`m` prefix, integer time) is preserved
    /// row for row, and pending steps join it in the shape it already has.
    #[test]
    fn newer_build_ledger_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(&format!(
                "CREATE TABLE {t} (
                     {v} varchar NOT NULL PRIMARY KEY,
                     {a} integer NOT NULL
                 );
                 INSERT INTO {t} VALUES
                     ('m{version}', 1);",
                t = migrator::LEDGER_TABLE,
                v = migrator::VERSION_COLUMN,
                a = migrator::APPLIED_AT_COLUMN,
                version = super::CANONICAL_FACTS_VERSION
            ))
            .unwrap();
        }

        crate::db::open(&path).unwrap();

        let rows = ledger_rows(&path);
        assert!(
            rows.iter().any(
                |(v, t)| v == &format!("m{}", super::CANONICAL_FACTS_VERSION)
                    && matches!(t, rusqlite::types::Value::Integer(1))
            ),
            "unknown rows are untouched: {rows:?}"
        );
        assert!(versions(&path).contains(&super::BASELINE_VERSION.to_owned()));
        let baseline = rows
            .iter()
            .find(|(version, _)| version == super::BASELINE_VERSION)
            .expect("the baseline was recorded");
        assert!(
            matches!(baseline.1, rusqlite::types::Value::Integer(_)),
            "new rows join the integer shape: {rows:?}"
        );

        let recorded = ledger_rows(&path);
        crate::db::open(&path).unwrap();
        assert_eq!(
            ledger_rows(&path),
            recorded,
            "a second open sees the rows as applied"
        );
        assert_eq!(
            table_count(&path, crate::entity::tracks::Entity.table_name()),
            1
        );
    }

    /// A blank database records every version once, in the shipped shape.
    #[test]
    fn blank_database_records_every_version_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.db");

        crate::db::open(&path).unwrap();
        let first = ledger_rows(&path);
        let recorded = versions(&path);
        for version in ALL_VERSIONS {
            assert!(recorded.contains(&version.to_owned()), "missing {version}");
        }
        assert!(
            first
                .iter()
                .all(|(_, t)| matches!(t, rusqlite::types::Value::Text(_))),
            "a fresh ledger uses the shipped RFC3339 shape: {first:?}"
        );

        crate::db::open(&path).unwrap();
        assert_eq!(ledger_rows(&path), first, "a second open records nothing");
        assert_eq!(
            table_count(&path, crate::entity::tracks::Entity.table_name()),
            1
        );
    }
}
