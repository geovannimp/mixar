//! Ordered, transactional library migrations.
//!
//! The runner records applied versions in `seaql_migrations` and applies each
//! pending migration in its own `BEGIN IMMEDIATE` transaction. A failed `up`
//! rolls back both its writes and its version row, so the next start retries it.
//!
//! Two phases share that ledger:
//!
//! - [`SCHEMA_COMPATIBILITY_MIGRATIONS`] run before SeaORM's entity sync, the
//!   only point at which a table can be renamed, dropped, or given a new
//!   primary key.
//! - [`MIGRATIONS`] run after it, so data backfills see the current schema.

use std::fmt;

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use crate::db::SEAQL_MIGRATIONS_TABLE;

pub(crate) mod beat_grid_backfill;
pub(crate) mod canonical_track_facts;
pub(crate) mod legacy_collection_tracks;
pub(crate) mod legacy_track_stem_cache;
pub(crate) mod track_analysis_append_only;

/// One migration step. Versions sort lexicographically and must be unique.
#[derive(Clone, Copy)]
pub(crate) struct Migration {
    pub version: &'static str,
    pub up: fn(&DatabaseConnection) -> Result<(), MigrationError>,
}

/// Failure while applying a migration. `version` identifies the step.
#[derive(Debug)]
pub(crate) struct MigrationError {
    pub version: String,
    pub message: String,
}

impl MigrationError {
    pub(crate) fn new(version: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "migration {}: {}", self.version, self.message)
    }
}

impl std::error::Error for MigrationError {}

/// Apply every migration not yet recorded in `seaql_migrations`.
///
/// Migrations are sorted by version. Duplicate versions are rejected; a
/// migration that fails rolls back its transaction and is not recorded.
pub(crate) fn run_migrations(
    conn: &DatabaseConnection,
    migrations: &[Migration],
) -> Result<(), MigrationError> {
    let mut ordered = migrations.to_vec();
    ordered.sort_by_key(|migration| migration.version);
    if ordered
        .windows(2)
        .any(|pair| pair[0].version == pair[1].version)
    {
        return Err(MigrationError::new(
            "",
            "duplicate migration version in the migration list",
        ));
    }

    let applied = applied_versions(conn)?;
    for migration in ordered {
        if applied.iter().any(|version| version == migration.version) {
            continue;
        }

        begin(conn, migration.version)?;
        if let Err(err) = (migration.up)(conn) {
            rollback(conn);
            return Err(err);
        }

        let insert = Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            format!("INSERT INTO {SEAQL_MIGRATIONS_TABLE} (version, applied_at) VALUES (?, ?)"),
            [
                migration.version.into(),
                chrono::Utc::now().to_rfc3339().into(),
            ],
        );
        if let Err(err) = conn.execute_raw(insert) {
            rollback(conn);
            return Err(MigrationError::new(migration.version, err.to_string()));
        }

        if let Err(err) = conn.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "COMMIT".to_string(),
        )) {
            rollback(conn);
            return Err(MigrationError::new(migration.version, err.to_string()));
        }
        tracing::info!(version = migration.version, "migration applied");
    }

    Ok(())
}

/// Run the data migrations compiled into this build.
pub(crate) fn run_pending_migrations(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    run_migrations(conn, MIGRATIONS)
}

/// Migrations that repair a layout a pre-existing database already has.
///
/// These run before SeaORM's entity sync, which can create tables and add
/// columns but never swap a primary key, rename, or drop a table. A fresh
/// database cannot have any of the legacy layouts, so every step must be a
/// no-op there; each is recorded once in the same ledger as [`MIGRATIONS`].
pub(crate) const SCHEMA_COMPATIBILITY_MIGRATIONS: &[Migration] = &[
    legacy_collection_tracks::MIGRATION,
    legacy_track_stem_cache::MIGRATION,
    track_analysis_append_only::MIGRATION,
];

/// Apply the schema-compatibility phase.
///
/// The ledger is created first: a database old enough to need these repairs
/// predates `seaql_migrations`, which entity sync would otherwise only create
/// afterwards. The statement matches the entity's shape, so sync finds it
/// already current.
pub(crate) fn run_schema_compatibility_migrations(
    conn: &DatabaseConnection,
) -> Result<(), MigrationError> {
    execute(
        conn,
        "schema ledger",
        format!(
            "CREATE TABLE IF NOT EXISTS {SEAQL_MIGRATIONS_TABLE} \
             (version TEXT NOT NULL PRIMARY KEY, applied_at TEXT NOT NULL)"
        ),
    )?;
    run_migrations(conn, SCHEMA_COMPATIBILITY_MIGRATIONS)
}

/// Run one migration statement, naming `version` when it fails.
pub(super) fn execute(
    conn: &DatabaseConnection,
    version: &str,
    sql: impl Into<String>,
) -> Result<(), MigrationError> {
    conn.execute_raw(Statement::from_string(DatabaseBackend::Sqlite, sql.into()))
        .map(|_| ())
        .map_err(|err| MigrationError::new(version, err.to_string()))
}

/// Whether this database has a table named `table`.
pub(super) fn table_exists(
    conn: &DatabaseConnection,
    version: &str,
    table: &str,
) -> Result<bool, MigrationError> {
    let row = conn
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = ?",
            [table.into()],
        ))
        .map_err(|err| MigrationError::new(version, err.to_string()))?;
    Ok(row
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or(0)
        != 0)
}

/// Every column of `table` with its primary-key position (0 when not a key).
pub(super) fn table_columns(
    conn: &DatabaseConnection,
    version: &str,
    table: &str,
) -> Result<Vec<(String, i64)>, MigrationError> {
    conn.query_all_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!("PRAGMA table_info({table})"),
    ))
    .map_err(|err| MigrationError::new(version, err.to_string()))?
    .into_iter()
    .map(|row| {
        let name: String = row
            .try_get("", "name")
            .map_err(|err| MigrationError::new(version, err.to_string()))?;
        let pk: i64 = row
            .try_get("", "pk")
            .map_err(|err| MigrationError::new(version, err.to_string()))?;
        Ok((name, pk))
    })
    .collect()
}

/// Data migrations, applied after entity sync has established the schema.
pub(crate) const MIGRATIONS: &[Migration] = &[
    beat_grid_backfill::MIGRATION,
    canonical_track_facts::MIGRATION,
];

fn applied_versions(conn: &DatabaseConnection) -> Result<Vec<String>, MigrationError> {
    let rows = conn
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT version FROM {SEAQL_MIGRATIONS_TABLE}"),
        ))
        .map_err(|err| MigrationError::new("", err.to_string()))?;
    let mut versions = Vec::with_capacity(rows.len());
    for row in rows {
        let version = row
            .try_get::<String>("", "version")
            .map_err(|err| MigrationError::new("", err.to_string()))?;
        versions.push(version);
    }
    Ok(versions)
}

fn begin(conn: &DatabaseConnection, version: &str) -> Result<(), MigrationError> {
    conn.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        "BEGIN IMMEDIATE".to_string(),
    ))
    .map(|_| ())
    .map_err(|err| MigrationError::new(version, err.to_string()))
}

fn rollback(conn: &DatabaseConnection) {
    let _ = conn.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        "ROLLBACK".to_string(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    static CREATE_PROBE: fn(&DatabaseConnection) -> Result<(), MigrationError> = |conn| {
        conn.execute_unprepared("CREATE TABLE runner_probe (id INTEGER)")
            .map(|_| ())
            .map_err(|err| MigrationError::new("t1", err.to_string()))
    };

    #[test]
    fn runner_records_once_and_skips_applied() {
        let db = crate::db::open_in_memory_unmigrated().unwrap();
        let conn = db.conn().unwrap();
        let migrations = [Migration {
            version: "t1",
            up: CREATE_PROBE,
        }];

        run_migrations(conn.as_connection(), &migrations).unwrap();
        run_migrations(conn.as_connection(), &migrations).unwrap();

        // The ledger also holds the schema-compatibility versions recorded at
        // open, so count only the probe.
        let applied = applied_versions(conn.as_connection()).unwrap();
        assert_eq!(applied.iter().filter(|version| *version == "t1").count(), 1);
        assert!(applied
            .iter()
            .any(|version| version == track_analysis_append_only::VERSION));
    }

    static FAIL: fn(&DatabaseConnection) -> Result<(), MigrationError> =
        |_| Err(MigrationError::new("t2", "boom"));

    #[test]
    fn runner_rolls_back_and_retries_failed_migration() {
        let db = crate::db::open_in_memory_unmigrated().unwrap();
        let conn = db.conn().unwrap();
        let migrations = [Migration {
            version: "t2",
            up: FAIL,
        }];

        assert!(run_migrations(conn.as_connection(), &migrations).is_err());
        assert!(!applied_versions(conn.as_connection())
            .unwrap()
            .iter()
            .any(|version| version == "t2"));
        assert!(run_migrations(conn.as_connection(), &migrations).is_err());
    }

    #[test]
    fn fresh_db_has_migration_table() {
        let db = open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let rows = conn
            .as_connection()
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name = '{SEAQL_MIGRATIONS_TABLE}'"
                ),
            ))
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn compatibility_phase_recreates_a_missing_ledger_and_leaves_current_layouts_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("predates-migrations.sqlite");
        {
            let db = crate::db::Db::open(&path).unwrap();
            let handle = db.conn().unwrap();
            handle
                .as_connection()
                .execute_unprepared(&format!("DROP TABLE {SEAQL_MIGRATIONS_TABLE}"))
                .unwrap();
        }

        // A database written before migrations existed has no ledger and no
        // legacy layout; the phase must rebuild the ledger and change nothing.
        let db = crate::db::Db::open(&path).unwrap();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        let recorded = applied_versions(conn).unwrap();
        assert!(recorded
            .iter()
            .any(|version| version == track_analysis_append_only::VERSION));
        let columns = table_columns(conn, "", "track_analysis").unwrap();
        let primary_key = |table_column: &str| {
            columns
                .iter()
                .find(|(name, _)| name == table_column)
                .map(|(_, pk)| *pk)
        };
        assert_eq!(primary_key("id"), Some(1));
        assert_eq!(primary_key("track_id"), Some(0));

        // Reopening records nothing new: every compatibility step is a no-op.
        drop(handle);
        drop(db);
        let db = crate::db::Db::open(&path).unwrap();
        let handle = db.conn().unwrap();
        assert_eq!(applied_versions(handle.as_connection()).unwrap(), recorded);
    }
}
