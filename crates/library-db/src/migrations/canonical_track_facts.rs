//! Backfill track-owned facts from legacy audit rows and reconcile compact grids.

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};

use super::beat_grid_backfill::usable_bpm;
use super::MigrationError;
use crate::db::{
    TRACKS_TABLE, TRACK_ANALYSIS_TABLE, TRACK_BEAT_GRID_TABLE, TRACK_BEAT_SEGMENT_TABLE,
};
use library_core::TrackId;
use library_core::{
    align_segmented_bpm, from_analysis_json, is_family_mismatch, BeatSegment, CanonicalBeatGrid,
    GridError, SOURCE_ANALYSIS, SOURCE_IMPORT, SOURCE_MANUAL,
};

const VERSION: &str = super::CANONICAL_FACTS_VERSION;
pub(crate) struct CanonicalTrackFacts;

migration!(CanonicalTrackFacts, VERSION);

fn migration_error(err: impl std::fmt::Display) -> MigrationError {
    super::migration_error(VERSION, err)
}

#[derive(Clone, Debug, PartialEq)]
struct Facts {
    bpm: Option<f64>,
    bpm_source: Option<String>,
    key_source: Option<String>,
    loudness_lufs: Option<f64>,
    sample_rate: Option<i64>,
    analysis_completed_at: Option<String>,
}

fn up_impl(conn: &DatabaseConnection) -> Result<(), MigrationError> {
    use sea_orm::ColumnTrait;

    let beat_grid_json = crate::entity::track_analysis::Column::BeatGridJson
        .as_column_ref()
        .1
        .to_string();
    let rows = conn
        .query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                r#"
            SELECT t.id AS track_id, t.bpm, t.bpm_source, t.key_source, t.loudness_lufs,
                   t.sample_rate, t.analysis_completed_at, t."key" IS NOT NULL AS has_key,
                   COALESCE(t.metadata_edited, 0) AS metadata_edited,
                   g.track_id IS NOT NULL AS has_grid,
                   a.id AS analysis_id, a.bpm AS detector_bpm, a.analyzed_at,
                   a.sample_rate AS raw_sample_rate, a.loudness_lufs AS raw_loudness,
                   a.{beat_grid_json} AS {beat_grid_json}
            FROM {TRACKS_TABLE} t
            LEFT JOIN {TRACK_BEAT_GRID_TABLE} g ON g.track_id = t.id
            LEFT JOIN {TRACK_ANALYSIS_TABLE} a ON a.id = (
                SELECT MAX(id) FROM {TRACK_ANALYSIS_TABLE} WHERE track_id = t.id
            )
            WHERE g.track_id IS NOT NULL
               OR (a.id IS NOT NULL AND (t.loudness_lufs IS NULL OR t.sample_rate IS NULL
                   OR t.sample_rate NOT BETWEEN 1 AND 2147483647
                   OR t.analysis_completed_at IS NULL))
               OR (t.bpm_source IS NULL AND (t.bpm IS NOT NULL OR t.metadata_edited))
               OR (t.key_source IS NULL AND (t."key" IS NOT NULL OR t.metadata_edited))
            ORDER BY t.id
            "#
            ),
        ))
        .map_err(migration_error)?;

    for row in rows {
        let track_id: String = row.try_get("", "track_id").map_err(migration_error)?;
        let metadata_edited: bool = row
            .try_get("", "metadata_edited")
            .map_err(migration_error)?;
        let has_key: bool = row.try_get("", "has_key").map_err(migration_error)?;
        let has_grid: bool = row.try_get("", "has_grid").map_err(migration_error)?;
        let original = Facts {
            bpm: row.try_get("", "bpm").map_err(migration_error)?,
            bpm_source: row.try_get("", "bpm_source").map_err(migration_error)?,
            key_source: row.try_get("", "key_source").map_err(migration_error)?,
            loudness_lufs: row.try_get("", "loudness_lufs").map_err(migration_error)?,
            sample_rate: row.try_get("", "sample_rate").map_err(migration_error)?,
            analysis_completed_at: row
                .try_get("", "analysis_completed_at")
                .map_err(migration_error)?,
        };
        let mut facts = original.clone();
        if facts.loudness_lufs.is_none() {
            let raw_loudness: Option<f64> =
                row.try_get("", "raw_loudness").map_err(migration_error)?;
            facts.loudness_lufs = raw_loudness.filter(|lufs| lufs.is_finite());
        }
        if !facts.sample_rate.is_some_and(valid_sample_rate) {
            let raw_sample_rate: Option<i64> = row
                .try_get("", "raw_sample_rate")
                .map_err(migration_error)?;
            if let Some(raw_sample_rate) = raw_sample_rate.filter(|&rate| valid_sample_rate(rate)) {
                facts.sample_rate = Some(raw_sample_rate);
            }
        }
        if facts.analysis_completed_at.is_none() {
            facts.analysis_completed_at =
                row.try_get("", "analyzed_at").map_err(migration_error)?;
        }

        if has_grid {
            let track_id: TrackId = track_id.as_str().into();
            if let Some(grid) =
                super::grid_io::load_grid(conn, VERSION, &track_id).map_err(migration_error)?
            {
                let analysis = AnalysisInput {
                    id: row.try_get("", "analysis_id").map_err(migration_error)?,
                    detector_bpm: row.try_get("", "detector_bpm").map_err(migration_error)?,
                    beat_grid_json: row
                        .try_get("", beat_grid_json.as_str())
                        .map_err(migration_error)?,
                };
                reconcile_grid(conn, &track_id, grid, &mut facts, metadata_edited, analysis)?;
            }
        }
        infer_source(&mut facts.bpm_source, facts.bpm.is_some(), metadata_edited);
        infer_source(&mut facts.key_source, has_key, metadata_edited);

        if facts != original {
            conn.execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    "UPDATE {TRACKS_TABLE} SET bpm = ?, bpm_source = ?, key_source = ?, \
                     loudness_lufs = ?, sample_rate = ?, analysis_completed_at = ? WHERE id = ?"
                ),
                [
                    facts.bpm.into(),
                    facts.bpm_source.into(),
                    facts.key_source.into(),
                    facts.loudness_lufs.into(),
                    facts.sample_rate.into(),
                    facts.analysis_completed_at.into(),
                    track_id.into(),
                ],
            ))
            .map_err(migration_error)?;
        }
    }
    Ok(())
}

fn valid_sample_rate(rate: i64) -> bool {
    (1..=i64::from(i32::MAX)).contains(&rate)
}

fn infer_source(source: &mut Option<String>, has_value: bool, metadata_edited: bool) {
    if source.is_none() {
        *source = if has_value {
            Some("legacy".to_owned())
        } else if metadata_edited {
            Some(SOURCE_MANUAL.to_owned())
        } else {
            None
        };
    }
}

/// The newest audit row's inputs for one track.
struct AnalysisInput {
    id: Option<i64>,
    detector_bpm: Option<f64>,
    beat_grid_json: Option<String>,
}

fn reconcile_grid(
    conn: &DatabaseConnection,
    track_id: &TrackId,
    grid: CanonicalBeatGrid,
    facts: &mut Facts,
    metadata_edited: bool,
    analysis: AnalysisInput,
) -> Result<(), MigrationError> {
    let owned = matches!(grid.source.as_str(), SOURCE_MANUAL | SOURCE_IMPORT);
    if let Err(err) = grid.validate() {
        tracing::warn!(track_id = track_id.as_str(), error = %err, "legacy compact grid is invalid");
        return if owned {
            Ok(())
        } else {
            super::grid_io::delete_grid(conn, VERSION, track_id).map_err(migration_error)
        };
    }

    let first = &grid.segments[0];
    if owned {
        // Restore the mirror without relabeling an already-matching BPM.
        if facts.bpm != Some(first.bpm) || facts.bpm_source.is_none() {
            facts.bpm = Some(first.bpm);
            facts.bpm_source = Some(grid.source);
        }
        return Ok(());
    }

    let cleared = facts.bpm_source.as_deref() == Some(SOURCE_MANUAL)
        || (metadata_edited && facts.bpm_source.is_none());
    let preferred_bpm = match facts.bpm {
        None if !cleared => {
            facts.bpm = Some(first.bpm);
            facts
                .bpm_source
                .get_or_insert_with(|| SOURCE_ANALYSIS.to_owned());
            return Ok(());
        }
        Some(bpm) if usable_bpm(bpm) => bpm,
        _ => return skip_automated_grid(conn, track_id, "cleared or invalid track BPM"),
    };

    let original_first = first.clone();
    let updated_first = if grid.segments.len() == 1 {
        let repaired = if analysis.id.is_some() {
            repaired_phase(
                track_id,
                analysis.beat_grid_json,
                analysis.detector_bpm,
                preferred_bpm,
                grid.covered_to_ms,
            )?
        } else {
            None
        };
        repaired.unwrap_or_else(|| {
            let mut first = original_first.clone();
            first.bpm = preferred_bpm;
            first
        })
    } else {
        if first.bpm == preferred_bpm {
            return Ok(());
        }
        let Some(aligned) = align_segmented_bpm(grid, preferred_bpm) else {
            return skip_automated_grid(conn, track_id, "segmented grid contradicts track BPM");
        };
        aligned
            .segments
            .into_iter()
            .next()
            .expect("validated grid has segment 0")
    };

    // Coverage, header timestamps and all segment IDs survive repair. Later
    // segments were validated as-is by the shared alignment helper.
    if updated_first != original_first {
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            format!(
                "UPDATE {TRACK_BEAT_SEGMENT_TABLE} SET bpm = ?, start_time_ms = ?, \
                 beats_per_bar = ?, beat_unit = ?, downbeat_offset_beats = ? \
                 WHERE track_id = ? AND segment_index = 0"
            ),
            [
                updated_first.bpm.into(),
                updated_first.start_time_ms.into(),
                i32::from(updated_first.beats_per_bar).into(),
                i32::from(updated_first.beat_unit).into(),
                i32::from(updated_first.downbeat_offset_beats).into(),
                track_id.as_str().into(),
            ],
        ))
        .map_err(migration_error)?;
    }
    Ok(())
}

fn repaired_phase(
    track_id: &TrackId,
    raw_json: Option<String>,
    detector_bpm: Option<f64>,
    preferred_bpm: f64,
    covered_to_ms: Option<i64>,
) -> Result<Option<BeatSegment>, MigrationError> {
    let Some(json) = raw_json.filter(|json| !matches!(json.as_str(), "null" | "")) else {
        return Ok(None);
    };

    // Recognize the mismatch from the measured BPM, or from the raw geometry
    // when the estimate does not describe it. Do not use unrelated raw phase.
    let repair = || -> Result<Option<BeatSegment>, GridError> {
        let detector = if let Some(bpm) =
            detector_bpm.filter(|&bpm| usable_bpm(bpm) && is_family_mismatch(bpm, preferred_bpm))
        {
            Some(bpm)
        } else {
            let Some(detected) = from_analysis_json(&json, None, None, covered_to_ms)? else {
                return Ok(None);
            };
            if detected.segments.len() != 1
                || !is_family_mismatch(detected.segments[0].bpm, preferred_bpm)
            {
                return Ok(None);
            }
            None
        };
        Ok(
            from_analysis_json(&json, detector, Some(preferred_bpm), covered_to_ms)?
                .filter(|grid| grid.segments.len() == 1)
                .and_then(|grid| grid.segments.into_iter().next()),
        )
    };
    match repair() {
        Ok(segment) => Ok(segment),
        Err(err) => {
            tracing::warn!(track_id = track_id.as_str(), error = %err, "legacy raw grid skipped during phase repair");
            Ok(None)
        }
    }
}

fn skip_automated_grid(
    conn: &DatabaseConnection,
    track_id: &TrackId,
    reason: &str,
) -> Result<(), MigrationError> {
    tracing::warn!(
        track_id = track_id.as_str(),
        reason,
        "legacy automated grid removed"
    );
    super::grid_io::delete_grid(conn, VERSION, track_id).map_err(migration_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, SEAQL_MIGRATIONS_TABLE};
    use crate::entity::{track_analysis, track_beat_grid, track_beat_segment, tracks};
    use crate::migrations::{
        beat_grid_backfill, legacy_collection_tracks, legacy_track_stem_cache,
        track_analysis_append_only,
    };
    use library_core::from_constant;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Value};

    const FAMILY_JSON: &str = r#"{"beats":[1.0,1.5128205,2.025641,2.5384615,3.051282,3.5641026,4.076923,4.5897436,5.102564],"bars":[1.5128205],"downbeats":[1.5128205,3.5641026],"beats_per_bar":4}"#;
    const CONSTANT_JSON: &str =
        r#"{"beats":[0.5,1.0,1.5,2.0,2.5,3.0],"downbeats":[1.0,3.0],"beats_per_bar":4}"#;
    const ANALYZED_AT: &str = "2025-06-04T03:02:01Z";

    fn execute(
        conn: &DatabaseConnection,
        sql: impl Into<String>,
        values: impl IntoIterator<Item = Value>,
    ) {
        conn.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            sql,
            values,
        ))
        .unwrap();
    }

    fn seed_track(conn: &DatabaseConnection, id: &str, bpm: Option<f64>, edited: bool) {
        execute(
            conn,
            "INSERT INTO tracks (id, source_type, source_ref, bpm, duration_ms, metadata_edited, added_at, updated_at)
             VALUES (?, 'file', ?, ?, 180000, ?, 'old', 'old')",
            [id.into(), id.into(), bpm.into(), edited.into()],
        );
    }

    fn seed_raw(
        conn: &DatabaseConnection,
        id: &str,
        backend: &str,
        bpm: f64,
        sample_rate: i64,
        json: &str,
    ) {
        execute(
            conn,
            format!(
                r#"INSERT INTO {TRACK_ANALYSIS_TABLE}
               (track_id, backend, backend_version, analyzed_at, bpm, bpm_confidence, "key",
                key_confidence, key_clarity, grid_stability, sample_rate, duration_analyzed_ms,
                loudness_lufs, beat_grid_json)
               VALUES (?, ?, 'old-backend', ?, ?, 0.81, 'F#m', 0.82, 0.83, 0.84, ?, 9000, -14.25, ?)"#
            ),
            [
                id.into(),
                backend.into(),
                ANALYZED_AT.into(),
                bpm.into(),
                sample_rate.into(),
                json.into(),
            ],
        );
    }

    fn seed_grid(conn: &DatabaseConnection, id: &str, grid: &CanonicalBeatGrid) {
        grid.validate().unwrap();
        execute(
            conn,
            "INSERT INTO track_beat_grid (track_id, source, covered_to_ms, created_at, updated_at)
             VALUES (?, ?, ?, 'old-created', 'old-updated')",
            [
                id.into(),
                grid.source.as_str().into(),
                grid.covered_to_ms.into(),
            ],
        );
        for segment in &grid.segments {
            execute(
                conn,
                "INSERT INTO track_beat_segment
                 (track_id, segment_index, start_time_ms, bpm, beats_per_bar, beat_unit,
                  start_beat_index, beats_to_next, downbeat_offset_beats)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                [
                    id.into(),
                    segment.segment_index.into(),
                    segment.start_time_ms.into(),
                    segment.bpm.into(),
                    i32::from(segment.beats_per_bar).into(),
                    i32::from(segment.beat_unit).into(),
                    segment.start_beat_index.into(),
                    segment.beats_to_next.into(),
                    i32::from(segment.downbeat_offset_beats).into(),
                ],
            );
        }
    }

    fn seed_old_file(conn: &DatabaseConnection, first_backfill_applied: bool) {
        // Replace the fresh audit table with the original track_id primary key
        // layout (including the original FK and all detector columns).
        execute(conn, format!("DROP TABLE {TRACK_ANALYSIS_TABLE}"), []);
        execute(
            conn,
            format!(
                r#"CREATE TABLE {TRACK_ANALYSIS_TABLE} (
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
            )"#
            ),
            [],
        );
        execute(conn, format!("DELETE FROM {SEAQL_MIGRATIONS_TABLE}"), []);
        if first_backfill_applied {
            execute(
                conn,
                format!(
                    "INSERT INTO {SEAQL_MIGRATIONS_TABLE} (version, applied_at) VALUES (?, 'old')"
                ),
                [beat_grid_backfill::VERSION.into()],
            );
        }
    }

    fn track(conn: &DatabaseConnection, id: &str) -> tracks::Model {
        tracks::Entity::find_by_id(id).one(conn).unwrap().unwrap()
    }

    fn grid(conn: &DatabaseConnection, id: &str) -> Option<CanonicalBeatGrid> {
        crate::migrations::grid_io::load_grid(conn, VERSION, &id.into()).unwrap()
    }

    fn segments(conn: &DatabaseConnection, id: &str) -> Vec<track_beat_segment::Model> {
        track_beat_segment::Entity::find()
            .filter(track_beat_segment::Column::TrackId.eq(id))
            .order_by_asc(track_beat_segment::Column::SegmentIndex)
            .all(conn)
            .unwrap()
    }

    fn all_raw(conn: &DatabaseConnection) -> Vec<track_analysis::Model> {
        track_analysis::Entity::find()
            .order_by_asc(track_analysis::Column::Id)
            .all(conn)
            .unwrap()
    }

    fn versions(conn: &DatabaseConnection) -> Vec<(String, String)> {
        conn.query_all_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT version, applied_at FROM {SEAQL_MIGRATIONS_TABLE} ORDER BY version"),
        ))
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                row.try_get("", "version").unwrap(),
                row.try_get("", "applied_at").unwrap(),
            )
        })
        .collect()
    }

    fn check_old_upgrade(first_backfill_applied: bool) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite");
        let db = Db::open(&path).unwrap();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        seed_old_file(conn, first_backfill_applied);

        seed_track(conn, "family", Some(78.0), false);
        seed_raw(conn, "family", SOURCE_ANALYSIS, 117.0, 48_000, FAMILY_JSON);
        execute(
            conn,
            "UPDATE tracks SET sample_rate = -1 WHERE id = 'family'",
            [],
        );
        seed_track(conn, "constant", Some(120.0), false);
        seed_raw(
            conn,
            "constant",
            SOURCE_ANALYSIS,
            120.0,
            48_000,
            CONSTANT_JSON,
        );
        execute(
            conn,
            "UPDATE tracks SET sample_rate = 44100 WHERE id = 'constant'",
            [],
        );
        if first_backfill_applied {
            seed_grid(
                conn,
                "family",
                &from_constant(117.0, 2000, 4, 4, 2000, Some(9000), SOURCE_ANALYSIS),
            );
            seed_grid(
                conn,
                "constant",
                &from_constant(120.0, 500, 4, 4, 1000, Some(9000), SOURCE_ANALYSIS),
            );
            seed_track(conn, "mirrored", Some(78.0), false);
            seed_raw(
                conn,
                "mirrored",
                SOURCE_ANALYSIS,
                117.0,
                48_000,
                FAMILY_JSON,
            );
            seed_grid(
                conn,
                "mirrored",
                &from_constant(78.0, 2000, 4, 4, 2000, Some(9000), SOURCE_ANALYSIS),
            );
        } else {
            // Manual audit BPM wins over the stale track BPM in the first migration.
            seed_track(conn, "manual-raw", Some(78.0), false);
            seed_raw(
                conn,
                "manual-raw",
                SOURCE_MANUAL,
                120.0,
                48_000,
                CONSTANT_JSON,
            );
        }
        let old_family = segments(conn, "family");
        let old_constant = segments(conn, "constant");
        for (id, source) in [("manual", SOURCE_MANUAL), ("import", SOURCE_IMPORT)] {
            seed_track(conn, id, None, true);
            seed_raw(conn, id, SOURCE_ANALYSIS, 117.0, 48_000, FAMILY_JSON);
            seed_grid(
                conn,
                id,
                &from_constant(125.0, 123, 3, 8, 603, None, source),
            );
        }
        execute(
            conn,
            "UPDATE tracks SET bpm = 78, bpm_source = 'tag' WHERE id = 'manual'",
            [],
        );
        for id in ["matching-known", "matching-unknown"] {
            seed_track(conn, id, Some(125.0), false);
            seed_grid(
                conn,
                id,
                &from_constant(125.0, 123, 3, 8, 603, None, SOURCE_IMPORT),
            );
        }
        execute(
            conn,
            "UPDATE tracks SET bpm_source = 'tag' WHERE id = 'matching-known'",
            [],
        );
        let old_manual = segments(conn, "manual");
        let old_import = segments(conn, "import");

        seed_track(conn, "bad", Some(78.0), false);
        seed_raw(conn, "bad", SOURCE_ANALYSIS, 117.0, i64::MAX, "{not json");
        seed_track(conn, "bad-compact", Some(78.0), false);
        seed_raw(conn, "bad-compact", SOURCE_ANALYSIS, 117.0, 0, "{not json");
        seed_grid(
            conn,
            "bad-compact",
            &from_constant(117.0, 530, 3, 8, 1043, Some(3200), SOURCE_ANALYSIS),
        );
        let old_bad_compact = segments(conn, "bad-compact");
        seed_track(conn, "no-raw", None, false);
        seed_grid(
            conn,
            "no-raw",
            &from_constant(123.0, 417, 3, 8, 417, None, SOURCE_ANALYSIS),
        );
        seed_track(conn, "clear", None, true);
        seed_raw(conn, "clear", SOURCE_ANALYSIS, 117.0, 48_000, FAMILY_JSON);
        seed_grid(
            conn,
            "clear",
            &from_constant(117.0, 0, 4, 4, 0, None, SOURCE_ANALYSIS),
        );
        seed_track(conn, "clear-no-grid", None, true);
        seed_raw(
            conn,
            "clear-no-grid",
            SOURCE_ANALYSIS,
            117.0,
            48_000,
            FAMILY_JSON,
        );
        seed_track(conn, "invalid", Some(401.0), false);
        seed_raw(conn, "invalid", SOURCE_ANALYSIS, 117.0, 48_000, FAMILY_JSON);
        if first_backfill_applied {
            seed_grid(
                conn,
                "invalid",
                &from_constant(117.0, 0, 4, 4, 0, None, SOURCE_ANALYSIS),
            );
        }

        let mut segmented = from_constant(120.1, 0, 4, 4, 0, Some(7000), SOURCE_ANALYSIS);
        segmented.segments[0].beats_to_next = Some(4);
        segmented.segments.push(BeatSegment {
            segment_index: 1,
            start_time_ms: 2000,
            bpm: 100.0,
            start_beat_index: 4,
            beats_to_next: None,
            ..segmented.segments[0].clone()
        });
        seed_track(conn, "segmented-incompatible", Some(78.0), false);
        seed_grid(conn, "segmented-incompatible", &segmented);
        seed_track(conn, "segmented-compatible", Some(120.0), false);
        seed_grid(conn, "segmented-compatible", &segmented);
        let old_compatible = segments(conn, "segmented-compatible");

        seed_track(conn, "known", Some(117.0), false);
        seed_raw(conn, "known", SOURCE_ANALYSIS, 117.0, 48_000, "{not json");
        execute(
            conn,
            r#"UPDATE tracks SET bpm_source = 'tag', "key" = 'Am', key_source = 'import',
                          loudness_lufs = -9.0, analysis_completed_at = 'existing' WHERE id = 'known'"#,
            [],
        );
        execute(
            conn,
            r#"UPDATE tracks SET "key" = 'F#m' WHERE id = 'family'"#,
            [],
        );
        drop(handle);
        drop(db);

        let db = Db::open(&path).unwrap();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        let family = track(conn, "family");
        assert_eq!(family.bpm, Some(78.0));
        assert_eq!(family.bpm_source.as_deref(), Some("legacy"));
        assert_eq!(family.key.as_deref(), Some("F#m"));
        assert_eq!(family.key_source.as_deref(), Some("legacy"));
        assert_eq!(family.loudness_lufs, Some(-14.25));
        assert_eq!(family.sample_rate, Some(48_000));
        assert_eq!(family.analysis_completed_at.as_deref(), Some(ANALYZED_AT));
        let family_grid = grid(conn, "family").unwrap();
        assert_eq!(family_grid.bpm(), Some(78.0));
        assert_eq!(family_grid.covered_to_ms, Some(9000));
        assert_eq!(
            Some(&family_grid),
            from_analysis_json(FAMILY_JSON, Some(117.0), Some(78.0), Some(9000))
                .unwrap()
                .as_ref(),
        );
        if first_backfill_applied {
            assert_eq!(segments(conn, "family")[0].id, old_family[0].id);
            assert_ne!(family_grid.first_beat_ms(), Some(2000));
            assert_eq!(grid(conn, "mirrored"), Some(family_grid));
        } else {
            let manual_raw = grid(conn, "manual-raw").unwrap();
            assert_eq!(manual_raw.bpm(), Some(120.0));
            assert_eq!(manual_raw.covered_to_ms, None);
            assert_eq!(
                track(conn, "manual-raw").bpm_source.as_deref(),
                Some(SOURCE_MANUAL)
            );
        }

        let constant = grid(conn, "constant").unwrap();
        assert_eq!(
            constant.materialize_beats(3000, 20),
            [500, 1000, 1500, 2000, 2500, 3000]
        );
        assert_eq!(constant.first_downbeat_ms(), Some(1000));
        assert_eq!(constant.bar_ms(), Some(2000.0));
        assert_eq!(track(conn, "constant").sample_rate, Some(44_100));
        if first_backfill_applied {
            assert_eq!(segments(conn, "constant"), old_constant);
        }
        for (id, source, old_segments) in [
            ("manual", SOURCE_MANUAL, old_manual),
            ("import", SOURCE_IMPORT, old_import),
        ] {
            assert_eq!(segments(conn, id), old_segments);
            let header = track_beat_grid::Entity::find_by_id(id)
                .one(conn)
                .unwrap()
                .unwrap();
            assert_eq!(header.source, source);
            assert_eq!(header.updated_at, "old-updated");
            assert_eq!(header.covered_to_ms, None);
            let track = track(conn, id);
            assert_eq!(track.bpm, Some(125.0));
            assert_eq!(track.bpm_source.as_deref(), Some(source));
        }
        assert_eq!(
            track(conn, "matching-known").bpm_source.as_deref(),
            Some("tag"),
        );
        assert_eq!(
            track(conn, "matching-unknown").bpm_source.as_deref(),
            Some(SOURCE_IMPORT),
        );

        for id in [
            "bad",
            "clear",
            "clear-no-grid",
            "invalid",
            "segmented-incompatible",
            "known",
        ] {
            assert!(grid(conn, id).is_none(), "unexpected grid for {id}");
        }
        assert_eq!(track(conn, "bad").sample_rate, None);
        assert_eq!(track(conn, "bad").key, None);
        let mut retuned = old_bad_compact;
        retuned[0].bpm = 78.0;
        assert_eq!(segments(conn, "bad-compact"), retuned);
        assert_eq!(grid(conn, "bad-compact").unwrap().covered_to_ms, Some(3200));
        assert_eq!(track(conn, "no-raw").bpm, Some(123.0));
        assert_eq!(
            track(conn, "no-raw").bpm_source.as_deref(),
            Some(SOURCE_ANALYSIS)
        );
        assert_eq!(track(conn, "no-raw").analysis_completed_at, None);
        for id in ["clear", "clear-no-grid"] {
            let clear = track(conn, id);
            assert_eq!(clear.bpm, None);
            assert_eq!(clear.bpm_source.as_deref(), Some(SOURCE_MANUAL));
            assert_eq!(clear.key, None);
            assert_eq!(clear.key_source.as_deref(), Some(SOURCE_MANUAL));
        }
        assert_eq!(track(conn, "invalid").bpm, Some(401.0));
        assert_eq!(track(conn, "segmented-incompatible").bpm, Some(78.0));
        assert!(segments(conn, "segmented-incompatible").is_empty());
        let mut aligned = old_compatible;
        aligned[0].bpm = 120.0;
        assert_eq!(segments(conn, "segmented-compatible"), aligned);
        let known = track(conn, "known");
        assert_eq!(known.bpm_source.as_deref(), Some("tag"));
        assert_eq!(known.key.as_deref(), Some("Am"));
        assert_eq!(known.key_source.as_deref(), Some("import"));
        assert_eq!(known.loudness_lufs, Some(-9.0));
        assert_eq!(known.analysis_completed_at.as_deref(), Some("existing"));

        let raw = all_raw(conn);
        assert_eq!(raw.len(), 11);
        let family_raw = raw.iter().find(|row| row.track_id == "family").unwrap();
        assert!(family_raw.id > 0);
        assert_eq!(family_raw.backend, SOURCE_ANALYSIS);
        assert_eq!(family_raw.backend_version, "old-backend");
        assert_eq!(family_raw.analyzed_at, ANALYZED_AT);
        assert_eq!(family_raw.bpm, Some(117.0));
        assert_eq!(family_raw.bpm_confidence, Some(0.81));
        assert_eq!(family_raw.key.as_deref(), Some("F#m"));
        assert_eq!(family_raw.key_confidence, Some(0.82));
        assert_eq!(family_raw.key_clarity, Some(0.83));
        assert_eq!(family_raw.grid_stability, Some(0.84));
        assert_eq!(family_raw.sample_rate, 48_000);
        assert_eq!(family_raw.duration_analyzed_ms, 9000);
        assert_eq!(family_raw.loudness_lufs, Some(-14.25));
        assert_eq!(family_raw.beat_grid_json.as_deref(), Some(FAMILY_JSON));
        let all_segments = |conn: &DatabaseConnection| {
            track_beat_segment::Entity::find()
                .order_by_asc(track_beat_segment::Column::Id)
                .all(conn)
                .unwrap()
        };
        let migrated_segments = all_segments(conn);
        let recorded = versions(conn);
        assert_eq!(
            recorded
                .iter()
                .map(|(version, _)| version.as_str())
                .collect::<Vec<_>>(),
            [
                legacy_collection_tracks::VERSION,
                legacy_track_stem_cache::VERSION,
                crate::migrations::BASELINE_VERSION,
                beat_grid_backfill::VERSION,
                track_analysis_append_only::VERSION,
                VERSION,
            ],
        );
        if first_backfill_applied {
            assert_eq!(
                recorded
                    .iter()
                    .find(|(version, _)| version == beat_grid_backfill::VERSION)
                    .expect("backfill is recorded")
                    .1,
                "old"
            );
        }
        drop(handle);
        drop(db);

        let db = Db::open(&path).unwrap();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        assert_eq!(all_raw(conn), raw);
        assert_eq!(all_segments(conn), migrated_segments);
        assert_eq!(versions(conn), recorded);

        execute(conn, "DELETE FROM tracks WHERE id = 'family'", []);
        assert!(all_raw(conn).iter().all(|row| row.track_id != "family"));
        assert_eq!(all_raw(conn).len(), raw.len() - 1);
    }

    #[test]
    fn upgrades_legacy_file_with_pending_first_backfill() {
        check_old_upgrade(false);
    }

    #[test]
    fn upgrades_legacy_file_with_first_backfill_already_applied() {
        check_old_upgrade(true);
    }

    #[test]
    fn latest_audit_row_is_chosen_by_id_not_timestamp() {
        let db = crate::db::open_in_memory_unmigrated().unwrap();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        seed_track(conn, "latest", None, false);
        seed_raw(conn, "latest", SOURCE_ANALYSIS, 117.0, 48_000, FAMILY_JSON);
        execute(
            conn,
            format!("UPDATE {TRACK_ANALYSIS_TABLE} SET analyzed_at = '2099-01-01T00:00:00Z'"),
            [],
        );
        seed_raw(
            conn,
            "latest",
            SOURCE_ANALYSIS,
            120.0,
            44_100,
            CONSTANT_JSON,
        );
        let raw = all_raw(conn);

        crate::migrations::run_pending_migrations(conn).unwrap();

        let latest = track(conn, "latest");
        assert_eq!(latest.bpm, Some(120.0));
        assert_eq!(latest.bpm_source.as_deref(), Some(SOURCE_ANALYSIS));
        assert_eq!(latest.sample_rate, Some(44_100));
        assert_eq!(latest.analysis_completed_at.as_deref(), Some(ANALYZED_AT));
        assert_eq!(grid(conn, "latest").unwrap().first_beat_ms(), Some(500));
        assert_eq!(all_raw(conn), raw);
    }
}
