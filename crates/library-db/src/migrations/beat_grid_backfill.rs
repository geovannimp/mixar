//! One-shot backfill of `track_analysis.beat_grid_json` into canonical grids.

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use super::MigrationError;
use crate::db::{TRACKS_TABLE, TRACK_ANALYSIS_TABLE, TRACK_BEAT_GRID_TABLE};
use library_core::{from_analysis_json, from_legacy_json, SOURCE_ANALYSIS, SOURCE_MANUAL};

pub(crate) const VERSION: &str = super::BACKFILL_VERSION;
pub(crate) struct BeatGridBackfill;

migration!(BeatGridBackfill, VERSION);

pub(super) fn usable_bpm(bpm: f64) -> bool {
    (20.0..=400.0).contains(&bpm)
}

fn migration_error(err: impl std::fmt::Display) -> MigrationError {
    super::migration_error(VERSION, err)
}

pub(crate) fn up_impl(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    let rows = conn
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                r#"
            SELECT t.id AS track_id,
                   t.bpm AS tracks_bpm,
                   t.bpm_source,
                   COALESCE(t.metadata_edited, 0) AS metadata_edited,
                   t.duration_ms AS track_duration_ms,
                   a.bpm AS analysis_bpm,
                   a.beat_grid_json,
                   a.backend,
                   a.duration_analyzed_ms
            FROM {TRACKS_TABLE} t
            JOIN {TRACK_ANALYSIS_TABLE} a ON a.id = (
                SELECT MAX(id) FROM {TRACK_ANALYSIS_TABLE} WHERE track_id = t.id
            )
            WHERE a.beat_grid_json IS NOT NULL
              AND a.beat_grid_json NOT IN ('null', '')
              AND NOT EXISTS (
                  SELECT 1 FROM {TRACK_BEAT_GRID_TABLE} g WHERE g.track_id = t.id
              )
            ORDER BY t.id
            "#
            ),
        ))
        .map_err(migration_error)?;

    for row in rows {
        let track_id: String = row.try_get("", "track_id").map_err(migration_error)?;
        let tracks_bpm: Option<f64> = row.try_get("", "tracks_bpm").map_err(migration_error)?;
        let bpm_source: Option<String> = row.try_get("", "bpm_source").map_err(migration_error)?;
        let metadata_edited: bool = row
            .try_get("", "metadata_edited")
            .map_err(migration_error)?;
        let track_duration_ms: Option<i32> = row
            .try_get("", "track_duration_ms")
            .map_err(migration_error)?;
        let analysis_bpm: Option<f64> = row.try_get("", "analysis_bpm").map_err(migration_error)?;
        let backend: String = row.try_get("", "backend").map_err(migration_error)?;
        let json: String = row.try_get("", "beat_grid_json").map_err(migration_error)?;
        let duration_analyzed_ms: i32 = row
            .try_get("", "duration_analyzed_ms")
            .map_err(migration_error)?;
        if backend != SOURCE_MANUAL
            && (tracks_bpm.is_some_and(|bpm| !usable_bpm(bpm))
                || (tracks_bpm.is_none()
                    && (metadata_edited || bpm_source.as_deref() == Some(SOURCE_MANUAL))))
        {
            tracing::warn!(
                track_id,
                "beat grid migration skipped a cleared or invalid track BPM"
            );
            continue;
        }
        let covered_to_ms = (duration_analyzed_ms > 0
            && track_duration_ms.is_none_or(|duration| duration_analyzed_ms < duration))
        .then(|| i64::from(duration_analyzed_ms));
        let (grid, grid_bpm_source) = if backend == SOURCE_MANUAL {
            // A manual grid's tempo comes from its own markers; the stored BPM
            // must not overwrite a tempo the user audited by hand. It is only a
            // last resort when the markers yield no usable tempo at all.
            let fallback = analysis_bpm
                .filter(|&bpm| usable_bpm(bpm))
                .or_else(|| tracks_bpm.filter(|&bpm| usable_bpm(bpm)));
            let grid = match from_legacy_json(&json, None, SOURCE_MANUAL, None) {
                Err(_) => from_legacy_json(&json, fallback, SOURCE_MANUAL, None),
                parsed => parsed,
            };
            (grid, SOURCE_MANUAL)
        } else {
            (
                from_analysis_json(&json, analysis_bpm, tracks_bpm, covered_to_ms),
                bpm_source.as_deref().unwrap_or(if tracks_bpm.is_some() {
                    "legacy"
                } else {
                    SOURCE_ANALYSIS
                }),
            )
        };

        match grid {
            Ok(Some(grid)) => {
                super::grid_io::write_grid(
                    conn,
                    VERSION,
                    &track_id.as_str().into(),
                    &grid,
                    grid_bpm_source,
                )
                .map_err(migration_error)?;
            }
            Ok(None) => {
                tracing::warn!(track_id, "beat grid migration skipped unusable geometry");
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
    use crate::db::open_in_memory_unmigrated;
    use crate::migrations::grid_io::load_grid;

    fn insert_legacy_track(
        db: &crate::db::Db,
        id: &str,
        tracks_bpm: f64,
        analysis_bpm: f64,
        backend: &str,
        json: &str,
    ) {
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            format!(
                "INSERT INTO {TRACKS_TABLE} (id, source_type, source_ref, bpm, duration_ms, added_at, updated_at) VALUES (?, '{}', ?, ?, 180000, 'now', 'now')",
                library_core::SOURCE_TYPE_FILE
            ),
            [id.into(), id.into(), tracks_bpm.into()],
        ))
        .unwrap();
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            format!(
                "INSERT INTO {TRACK_ANALYSIS_TABLE} (track_id, backend, backend_version, analyzed_at, bpm, sample_rate, duration_analyzed_ms, beat_grid_json) VALUES (?, ?, 'test', 'now', ?, 44100, 180000, ?)"
            ),
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
                format!("SELECT bpm FROM {TRACKS_TABLE} WHERE id = ?"),
                [id.into()],
            ))
            .unwrap()
            .and_then(|row| row.try_get::<Option<f64>>("", "bpm").unwrap())
    }

    fn run_backfill(db: &crate::db::Db) {
        migrator::Migrator::new()
            .push(BeatGridBackfill)
            .up(db.conn().unwrap().as_connection())
            .unwrap();
    }

    fn read_grid(db: &crate::db::Db, id: &str) -> Option<library_core::CanonicalBeatGrid> {
        load_grid(db.conn().unwrap().as_connection(), VERSION, &id.into()).unwrap()
    }

    #[test]
    fn backfill_copies_legacy_grids_and_skips_malformed() {
        let db = open_in_memory_unmigrated().unwrap();
        insert_legacy_track(
            &db,
            "ok",
            117.0,
            78.0,
            SOURCE_ANALYSIS,
            r#"{"beats":[0.0,0.769,1.538],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#,
        );
        insert_legacy_track(&db, "bad", 120.0, 120.0, "analysis", "{not json");

        run_backfill(&db);

        let grid = read_grid(&db, "ok").unwrap();
        assert_eq!(grid.source, SOURCE_ANALYSIS);
        assert_eq!(grid.bpm(), Some(117.0));
        assert_eq!(track_bpm(&db, "ok"), Some(117.0));
        assert!(read_grid(&db, "bad").is_none());

        // Idempotent: a second run records nothing and rewrites nothing.
        run_backfill(&db);
        assert_eq!(read_grid(&db, "ok"), Some(grid));
    }

    #[test]
    fn manual_grids_fall_back_to_a_stored_tempo_when_markers_are_unusable() {
        let db = open_in_memory_unmigrated().unwrap();
        // Four seconds between markers is 15 BPM, below the window; the stored
        // tempo is all that can describe the grid then.
        insert_legacy_track(
            &db,
            "sparse",
            120.0,
            120.0,
            SOURCE_MANUAL,
            r#"{"beats":[0.0,4.0,8.0],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#,
        );

        run_backfill(&db);

        assert_eq!(read_grid(&db, "sparse").unwrap().bpm(), Some(120.0));
    }

    #[test]
    fn manual_grids_keep_their_marker_tempo() {
        let db = open_in_memory_unmigrated().unwrap();
        // One second between markers is 60 BPM, while both stored BPMs say 120.
        insert_legacy_track(
            &db,
            "audited",
            120.0,
            120.0,
            SOURCE_MANUAL,
            r#"{"beats":[0.0,1.0,2.0],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#,
        );

        run_backfill(&db);

        assert_eq!(read_grid(&db, "audited").unwrap().bpm(), Some(60.0));
        assert_eq!(track_bpm(&db, "audited"), Some(60.0));
    }
}
