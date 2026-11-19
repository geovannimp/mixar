//! Make the analysis audit append-only.
//!
//! `track_analysis` used to key one row per track (`track_id` primary key).
//! Analysis now appends one row per run, so the table needs a generated `id`
//! primary key and a plain indexed `track_id`. Entity sync can add columns but
//! never change a primary key, which is why this rebuild runs before it
//! ([`super::SCHEMA_COMPATIBILITY_MIGRATIONS`]).
//!
//! The existing rows are the recorded analysis history and must survive
//! verbatim, so every old column is copied; only the generated `id` is new.
//! The runner already holds the write lock, so a single pass is enough and a
//! failure rolls the whole rebuild back.

use sea_orm::DatabaseConnection;

use super::{execute, Migration, MigrationError};

pub(crate) const VERSION: &str = "2026111900_track_analysis_append_only";

pub(crate) const MIGRATION: Migration = Migration {
    version: VERSION,
    up,
};

fn up(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    if !has_legacy_primary_key(conn)? {
        return Ok(());
    }
    execute(
        conn,
        VERSION,
        "ALTER TABLE track_analysis RENAME TO track_analysis_legacy",
    )?;
    execute(
        conn,
        VERSION,
        r#"CREATE TABLE track_analysis (
            id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
            track_id TEXT NOT NULL,
            backend TEXT NOT NULL,
            backend_version TEXT NOT NULL,
            analyzed_at TEXT NOT NULL,
            bpm REAL,
            bpm_confidence REAL,
            "key" TEXT,
            key_confidence REAL,
            key_clarity REAL,
            grid_stability REAL,
            sample_rate INTEGER NOT NULL,
            duration_analyzed_ms INTEGER NOT NULL,
            loudness_lufs REAL,
            beat_grid_json TEXT,
            FOREIGN KEY (track_id) REFERENCES tracks(id)
                ON DELETE CASCADE ON UPDATE CASCADE
        )"#,
    )?;
    let columns = r#"track_id, backend, backend_version, analyzed_at, bpm, bpm_confidence,
        "key", key_confidence, key_clarity, grid_stability, sample_rate, duration_analyzed_ms,
        loudness_lufs, beat_grid_json"#;
    execute(
        conn,
        VERSION,
        format!(
            "INSERT INTO track_analysis ({columns}) SELECT {columns} FROM track_analysis_legacy"
        ),
    )?;
    execute(conn, VERSION, "DROP TABLE track_analysis_legacy")
}

/// True when `track_analysis` still keys rows by `track_id`.
///
/// A missing table, or one already keyed by `id`, is left untouched: sync owns
/// both the fresh table and its `track_id` index.
fn has_legacy_primary_key(conn: &DatabaseConnection) -> Result<bool, MigrationError> {
    let mut legacy = false;
    for (name, pk) in super::table_columns(conn, VERSION, "track_analysis")? {
        if name == "id" && pk > 0 {
            return Ok(false);
        }
        legacy |= name == "track_id" && pk > 0;
    }
    Ok(legacy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::ConnectionTrait;

    fn open_current() -> crate::db::Db {
        crate::db::open_in_memory_unmigrated().unwrap()
    }

    #[test]
    fn current_layout_is_left_alone() {
        let db = open_current();
        let conn = db.conn().unwrap();
        let conn = conn.as_connection();
        assert!(!has_legacy_primary_key(conn).unwrap());
        up(conn).unwrap();
        assert!(!has_legacy_primary_key(conn).unwrap());
    }

    #[test]
    fn legacy_rows_are_copied_into_the_append_only_shape() {
        let db = open_current();
        let conn = db.conn().unwrap();
        let conn = conn.as_connection();
        execute(conn, VERSION, "DROP TABLE track_analysis").unwrap();
        execute(
            conn,
            VERSION,
            r#"CREATE TABLE track_analysis (
                track_id TEXT NOT NULL PRIMARY KEY,
                backend TEXT NOT NULL,
                backend_version TEXT NOT NULL,
                analyzed_at TEXT NOT NULL,
                bpm REAL,
                bpm_confidence REAL,
                "key" TEXT,
                key_confidence REAL,
                key_clarity REAL,
                grid_stability REAL,
                sample_rate INTEGER NOT NULL,
                duration_analyzed_ms INTEGER NOT NULL,
                loudness_lufs REAL,
                beat_grid_json TEXT,
                FOREIGN KEY (track_id) REFERENCES tracks(id)
                    ON DELETE CASCADE ON UPDATE CASCADE
            )"#,
        )
        .unwrap();
        execute(
            conn,
            VERSION,
            "INSERT INTO tracks (id, source_type, source_ref, added_at, updated_at)
             VALUES ('t1', 'file', 't1', 'old', 'old')",
        )
        .unwrap();
        execute(
            conn,
            VERSION,
            "INSERT INTO track_analysis
             (track_id, backend, backend_version, analyzed_at, bpm, sample_rate,
              duration_analyzed_ms, loudness_lufs)
             VALUES ('t1', 'old-backend', '1', '2025-01-01T00:00:00Z', 117.0, 48000, 9000, -14.25)",
        )
        .unwrap();

        up(conn).unwrap();

        assert!(!has_legacy_primary_key(conn).unwrap());
        let row = conn
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT id, track_id, backend, bpm, sample_rate, loudness_lufs FROM track_analysis",
            ))
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "id").unwrap(), 1);
        assert_eq!(row.try_get::<String>("", "track_id").unwrap(), "t1");
        assert_eq!(row.try_get::<String>("", "backend").unwrap(), "old-backend");
        assert_eq!(row.try_get::<f64>("", "bpm").unwrap(), 117.0);
        assert_eq!(row.try_get::<i64>("", "sample_rate").unwrap(), 48_000);
        assert_eq!(row.try_get::<f64>("", "loudness_lufs").unwrap(), -14.25);

        // A second run keeps appending rather than replacing the row.
        execute(
            conn,
            VERSION,
            "INSERT INTO track_analysis
             (track_id, backend, backend_version, analyzed_at, sample_rate, duration_analyzed_ms)
             VALUES ('t1', 'new-backend', '2', '2025-02-01T00:00:00Z', 48000, 9000)",
        )
        .unwrap();
        let count = conn
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT COUNT(*) AS count FROM track_analysis WHERE track_id = 't1'",
            ))
            .unwrap()
            .unwrap();
        assert_eq!(count.try_get::<i64>("", "count").unwrap(), 2);
    }
}
