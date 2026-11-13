//! Ordered, transactional library migrations.
//!
//! The runner records applied versions in `seaql_migrations` and applies each
//! pending migration in its own `BEGIN IMMEDIATE` transaction. A failed `up`
//! rolls back both its writes and its version row, so the next start retries it.

use std::fmt;

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

pub(crate) mod beat_grid_backfill;

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
            "INSERT INTO seaql_migrations (version, applied_at) VALUES (?, ?)",
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

/// Run the migrations compiled into this build.
pub(crate) fn run_pending_migrations(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    run_migrations(conn, MIGRATIONS)
}

/// All migrations in this build.
pub(crate) const MIGRATIONS: &[Migration] = &[beat_grid_backfill::MIGRATION];

fn applied_versions(conn: &DatabaseConnection) -> Result<Vec<String>, MigrationError> {
    let rows = conn
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT version FROM seaql_migrations".to_string(),
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

        let rows = conn
            .as_connection()
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT version FROM seaql_migrations".to_string(),
            ))
            .unwrap();
        assert_eq!(rows.len(), 1);
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
        let rows = conn
            .as_connection()
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT version FROM seaql_migrations".to_string(),
            ))
            .unwrap();
        assert!(rows.is_empty());
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
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'seaql_migrations'"
                    .to_string(),
            ))
            .unwrap();
        assert_eq!(rows.len(), 1);
    }
}
