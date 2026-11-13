//! One-shot backfill of `track_analysis.beat_grid_json` into canonical grids.

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use super::{Migration, MigrationError};
use crate::beat_grid;

pub(crate) const VERSION: &str = "2026111201_beat_grid_backfill";

pub(crate) const MIGRATION: Migration = Migration {
    version: VERSION,
    up,
};

pub(crate) fn up(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    let rows = conn
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            r#"
            SELECT t.id AS track_id,
                   t.bpm AS tracks_bpm,
                   t.duration_ms AS track_duration_ms,
                   a.bpm AS analysis_bpm,
                   a.beat_grid_json,
                   a.backend,
                   a.duration_analyzed_ms
            FROM tracks t
            LEFT JOIN track_analysis a ON a.track_id = t.id
            WHERE a.beat_grid_json IS NOT NULL
              AND a.beat_grid_json NOT IN ('null', '')
              AND NOT EXISTS (
                  SELECT 1 FROM track_beat_grid g WHERE g.track_id = t.id
              )
            ORDER BY t.id
            "#,
        ))
        .map_err(|err| MigrationError::new(VERSION, err.to_string()))?;

    for row in rows {
        let track_id: String = row
            .try_get("", "track_id")
            .map_err(|err| MigrationError::new(VERSION, err.to_string()))?;
        let tracks_bpm: Option<f64> = row.try_get("", "tracks_bpm").ok().flatten();
        let track_duration_ms: Option<i32> = row.try_get("", "track_duration_ms").ok().flatten();
        let analysis_bpm: Option<f64> = row.try_get("", "analysis_bpm").ok().flatten();
        let backend: String = row
            .try_get::<Option<String>>("", "backend")
            .ok()
            .flatten()
            .unwrap_or_else(|| "analysis".to_string());
        let json: String = row
            .try_get("", "beat_grid_json")
            .map_err(|err| MigrationError::new(VERSION, err.to_string()))?;
        let duration_analyzed_ms: Option<i32> =
            row.try_get("", "duration_analyzed_ms").ok().flatten();
        let covered_to_ms = if backend == "manual" {
            None
        } else {
            duration_analyzed_ms
                .filter(|duration| *duration > 0)
                .filter(|duration| {
                    track_duration_ms.is_none_or(|track_duration| *duration < track_duration)
                })
                .map(i64::from)
        };

        match beat_grid::from_legacy_json(
            &json,
            tracks_bpm.or(analysis_bpm),
            &backend,
            covered_to_ms,
        ) {
            Ok(Some(grid)) => {
                beat_grid::write_track_beat_grid_tx(conn, &track_id.as_str().into(), &grid)
                    .map_err(|err| MigrationError::new(VERSION, err.to_string()))?;
            }
            Ok(None) => {
                tracing::warn!(track_id, "beat grid migration skipped an empty grid");
            }
            Err(err) => {
                tracing::warn!(track_id, error = %err, "beat grid migration skipped");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beat_grid::{get_track_beat_grid, SOURCE_ANALYSIS};
    use crate::db::open_in_memory_unmigrated;
    use crate::migrations::run_migrations;

    fn insert_legacy_track(
        db: &crate::db::Db,
        id: &str,
        tracks_bpm: f64,
        analysis_bpm: f64,
        backend: &str,
        json: &str,
    ) {
        let conn = db.conn().unwrap();
        let conn = conn.as_connection();
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO tracks (id, source_type, source_ref, bpm, duration_ms, added_at, updated_at) VALUES (?, 'file', ?, ?, 180000, 'now', 'now')",
            [id.into(), id.into(), tracks_bpm.into()],
        ))
        .unwrap();
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO track_analysis (track_id, backend, backend_version, analyzed_at, bpm, sample_rate, duration_analyzed_ms, beat_grid_json) VALUES (?, ?, 'test', 'now', ?, 44100, 180000, ?)",
            [id.into(), backend.into(), analysis_bpm.into(), json.into()],
        ))
        .unwrap();
    }

    fn track_bpm(db: &crate::db::Db, id: &str) -> Option<f64> {
        db.conn()
            .unwrap()
            .as_connection()
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT bpm FROM tracks WHERE id = ?",
                [id.into()],
            ))
            .unwrap()
            .and_then(|row| row.try_get::<Option<f64>>("", "bpm").unwrap())
    }

    #[test]
    fn backfill_copies_legacy_grids_and_skips_malformed() {
        let db = open_in_memory_unmigrated().unwrap();
        insert_legacy_track(
            &db,
            "ok",
            117.0,
            78.0,
            "analysis",
            r#"{"beats":[0.0,0.769,1.538],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#,
        );
        insert_legacy_track(&db, "bad", 120.0, 120.0, "analysis", "{not json");

        run_migrations(db.conn().unwrap().as_connection(), &[MIGRATION]).unwrap();

        let grid = get_track_beat_grid(&db, &"ok".into()).unwrap().unwrap();
        assert_eq!(grid.source, SOURCE_ANALYSIS);
        assert_eq!(grid.bpm(), Some(117.0));
        assert_eq!(track_bpm(&db, "ok"), Some(117.0));
        assert!(get_track_beat_grid(&db, &"bad".into()).unwrap().is_none());

        // Idempotent: a second run records nothing and rewrites nothing.
        run_migrations(db.conn().unwrap().as_connection(), &[MIGRATION]).unwrap();
        assert_eq!(get_track_beat_grid(&db, &"ok".into()).unwrap(), Some(grid));
    }
}
