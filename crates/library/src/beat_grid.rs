//! Canonical beat grids: the model lives in `library-core`, the reads and
//! writes live here.

use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set};

use library_core::{GridError, LibraryError, Result, TrackId};

#[cfg(test)]
pub(crate) use library_core::from_legacy_json;
pub(crate) use library_core::{from_analysis_json, from_constant};
pub use library_core::{
    BeatSegment, CanonicalBeatGrid, SOURCE_ANALYSIS, SOURCE_IMPORT, SOURCE_MANUAL,
};

use crate::db::Db;
use crate::entity::{
    track_beat_grid, track_beat_segment, tracks, TrackBeatGridEntity, TrackBeatSegmentEntity,
    TrackEntity,
};

/// Write a canonical grid and set `tracks.bpm` to segment 0 in one transaction.
pub(crate) fn write_track_beat_grid(
    db: &Db,
    track_id: &TrackId,
    grid: &CanonicalBeatGrid,
    bpm_source: &str,
) -> Result<()> {
    db.transaction(|conn| write_track_beat_grid_tx(conn, track_id, grid, bpm_source))
}

/// Same writes as [`write_track_beat_grid`], without beginning a transaction.
///
/// The migration runner already holds one; callers outside a transaction should
/// use [`write_track_beat_grid`].
pub(crate) fn write_track_beat_grid_tx(
    conn: &DatabaseConnection,
    track_id: &TrackId,
    grid: &CanonicalBeatGrid,
    bpm_source: &str,
) -> Result<()> {
    grid.validate().map_err(grid_error)?;
    let now = chrono::Utc::now().to_rfc3339();

    TrackBeatSegmentEntity::delete_many()
        .filter(track_beat_segment::Column::TrackId.eq(track_id.as_str()))
        .exec(conn)
        .map_err(crate::db::db_err)?;

    let header = track_beat_grid::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        source: Set(grid.source.clone()),
        covered_to_ms: Set(grid.covered_to_ms),
        created_at: Set(now.clone()),
        updated_at: Set(now.clone()),
    };
    TrackBeatGridEntity::insert(header)
        .on_conflict(
            OnConflict::column(track_beat_grid::Column::TrackId)
                .update_columns([
                    track_beat_grid::Column::Source,
                    track_beat_grid::Column::CoveredToMs,
                    track_beat_grid::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(conn)
        .map_err(crate::db::db_err)?;

    let segments: Vec<track_beat_segment::ActiveModel> = grid
        .segments
        .iter()
        .map(|segment| track_beat_segment::ActiveModel {
            track_id: Set(track_id.as_str().to_string()),
            segment_index: Set(segment.segment_index),
            start_time_ms: Set(segment.start_time_ms),
            bpm: Set(segment.bpm),
            beats_per_bar: Set(i32::from(segment.beats_per_bar)),
            beat_unit: Set(i32::from(segment.beat_unit)),
            start_beat_index: Set(segment.start_beat_index),
            beats_to_next: Set(segment.beats_to_next),
            downbeat_offset_beats: Set(i32::from(segment.downbeat_offset_beats)),
            ..Default::default()
        })
        .collect();
    TrackBeatSegmentEntity::insert_many(segments)
        .exec(conn)
        .map_err(crate::db::db_err)?;

    let track = tracks::ActiveModel {
        id: Set(track_id.as_str().to_string()),
        bpm: Set(Some(grid.segments[0].bpm)),
        bpm_source: Set(Some(bpm_source.to_string())),
        ..Default::default()
    };
    TrackEntity::update(track)
        .exec(conn)
        .map_err(crate::db::db_err)?;
    Ok(())
}

pub(crate) fn get_track_beat_grid(
    db: &Db,
    track_id: &TrackId,
) -> Result<Option<CanonicalBeatGrid>> {
    let guard = db.conn()?;
    get_track_beat_grid_tx(guard.as_connection(), track_id)
}

/// Same read as [`get_track_beat_grid`] on a caller-provided connection, so it
/// can run inside [`Db::transaction`](crate::db::Db::transaction).
pub(crate) fn get_track_beat_grid_tx(
    conn: &DatabaseConnection,
    track_id: &TrackId,
) -> Result<Option<CanonicalBeatGrid>> {
    let Some(header) = TrackBeatGridEntity::find_by_id(track_id.as_str())
        .one(conn)
        .map_err(crate::db::db_err)?
    else {
        return Ok(None);
    };
    let rows = TrackBeatSegmentEntity::find()
        .filter(track_beat_segment::Column::TrackId.eq(track_id.as_str()))
        .order_by_asc(track_beat_segment::Column::SegmentIndex)
        .all(conn)
        .map_err(crate::db::db_err)?;

    let segments = rows
        .into_iter()
        .map(|row| {
            Ok(BeatSegment {
                segment_index: row.segment_index,
                start_time_ms: row.start_time_ms,
                bpm: row.bpm,
                beats_per_bar: u8::try_from(row.beats_per_bar).map_err(|_| {
                    LibraryError::Backend {
                        backend: "library",
                        message: "beat segment has an invalid beats_per_bar".into(),
                    }
                })?,
                beat_unit: u8::try_from(row.beat_unit).map_err(|_| LibraryError::Backend {
                    backend: "library",
                    message: "beat segment has an invalid beat_unit".into(),
                })?,
                start_beat_index: row.start_beat_index,
                beats_to_next: row.beats_to_next,
                downbeat_offset_beats: u8::try_from(row.downbeat_offset_beats).map_err(|_| {
                    LibraryError::Backend {
                        backend: "library",
                        message: "beat segment has an invalid downbeat offset".into(),
                    }
                })?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Some(CanonicalBeatGrid {
        source: header.source,
        covered_to_ms: header.covered_to_ms,
        segments,
    }))
}

#[cfg(test)]
pub(crate) fn delete_track_beat_grid(db: &Db, track_id: &TrackId) -> Result<()> {
    delete_track_beat_grid_tx(db.conn()?.as_connection(), track_id)
}

pub(crate) fn delete_track_beat_grid_tx(
    conn: &DatabaseConnection,
    track_id: &TrackId,
) -> Result<()> {
    TrackBeatGridEntity::delete_by_id(track_id.as_str())
        .exec(conn)
        .map(|_| ())
        .map_err(crate::db::db_err)
}

pub(crate) fn has_track_beat_grid_tx(
    conn: &DatabaseConnection,
    track_id: &TrackId,
) -> Result<bool> {
    Ok(TrackBeatGridEntity::find_by_id(track_id.as_str())
        .one(conn)
        .map_err(crate::db::db_err)?
        .is_some())
}

fn grid_error(err: GridError) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    fn seg(
        index: i32,
        start_ms: i64,
        bpm: f64,
        start_beat: i64,
        count: Option<i64>,
        offset: u8,
    ) -> BeatSegment {
        BeatSegment {
            segment_index: index,
            start_time_ms: start_ms,
            bpm,
            beats_per_bar: 4,
            beat_unit: 4,
            start_beat_index: start_beat,
            beats_to_next: count,
            downbeat_offset_beats: offset,
        }
    }

    fn constant() -> CanonicalBeatGrid {
        CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: None,
            segments: vec![seg(0, 0, 120.0, 0, None, 1)],
        }
    }

    #[test]
    fn constant_grid_math() {
        let grid = constant();
        assert_eq!(grid.bpm(), Some(120.0));
        assert_eq!(grid.first_beat_ms(), Some(0));
        assert_eq!(grid.beat_ms(), Some(500.0));
        assert_eq!(grid.bar_ms(), Some(2000.0));
        assert_eq!(grid.first_downbeat_ms(), Some(500));
        assert_eq!(grid.beat_time_ms(4), Some(2000));
        assert_eq!(grid.beat_index_at(1000), Some(2.0));
        assert_eq!(
            grid.materialize_beats(2500, 100),
            vec![0, 500, 1000, 1500, 2000, 2500]
        );
        assert_eq!(grid.materialize_beats(2500, 3), vec![0, 500, 1000]);
    }

    #[test]
    fn grid_math_switches_bpm_at_the_segment_boundary() {
        let grid = CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: None,
            segments: vec![
                seg(0, 0, 120.0, 0, Some(16), 0),
                seg(1, 8_000, 140.0, 16, None, 0),
            ],
        };
        assert_eq!(grid.bpm_at(7_999), Some(120.0));
        assert_eq!(grid.bpm_at(8_000), Some(140.0));
        assert_eq!(grid.beat_time_ms(16), Some(8_000));
        assert!((grid.bpm_at(12_000).unwrap() - 140.0).abs() < 1e-9);
        assert_eq!(grid.beat_time_ms(17), Some(8_429));
        assert_eq!(grid.beat_time_ms(15), Some(7_500));
    }

    #[test]
    fn materialize_spans_a_tempo_change() {
        let grid = CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: None,
            segments: vec![
                seg(0, 0, 120.0, 0, Some(16), 0),
                seg(1, 8_000, 140.0, 16, None, 0),
            ],
        };
        let beats = grid.materialize_beats(9_000, 100);
        assert_eq!(beats.len(), 19);
        assert_eq!(beats[15], 7_500);
        assert_eq!(beats[16], 8_000);
        assert_eq!(beats[17], 8_429);
        assert_eq!(beats[18], 8_857);
        let secs = grid.materialize_beat_secs(9_000, 100);
        assert!((secs[17] - 8.428_571).abs() < 1e-5);
    }

    #[test]
    fn validate_rejects_a_broken_beat_chain() {
        let grid = CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: None,
            segments: vec![
                seg(0, 0, 120.0, 0, Some(16), 0),
                seg(1, 8_000, 140.0, 17, None, 0),
            ],
        };
        assert!(matches!(grid.validate(), Err(GridError::Invalid(_))));
    }

    #[test]
    fn validate_rejects_a_non_last_terminal_segment() {
        let grid = CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: None,
            segments: vec![
                seg(0, 0, 120.0, 0, None, 0),
                seg(1, 8_000, 140.0, 16, None, 0),
            ],
        };
        assert!(matches!(grid.validate(), Err(GridError::Invalid(_))));
    }

    #[test]
    fn validate_rejects_an_offset_outside_the_bar() {
        let grid = CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: None,
            segments: vec![seg(0, 0, 120.0, 0, None, 4)],
        };
        assert!(matches!(grid.validate(), Err(GridError::Invalid(_))));
    }

    #[test]
    fn legacy_constant_grid_converts_with_downbeat_offset() {
        let grid = from_legacy_json(
            r#"{"beats":[0.5,1.0,1.5,2.0],"bars":[1.0],"downbeats":[1.0],"beats_per_bar":4}"#,
            Some(120.0),
            "analysis",
            Some(4_000),
        )
        .unwrap()
        .unwrap();
        assert_eq!(grid.bpm(), Some(120.0));
        assert_eq!(grid.first_beat_ms(), Some(500));
        assert_eq!(grid.segments.len(), 1);
        assert_eq!(grid.segments[0].downbeat_offset_beats, 1);
        assert_eq!(grid.covered_to_ms, Some(4_000));
        assert_eq!(grid.source, SOURCE_ANALYSIS);
    }

    #[test]
    fn legacy_variable_grid_splits_segments() {
        let grid = from_legacy_json(
            r#"{"beats":[0.0,0.5,1.0,1.4,1.8,2.2],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#,
            Some(120.0),
            "analysis",
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(grid.segments.len(), 2);
        assert_eq!(grid.segments[0].start_beat_index, 0);
        assert_eq!(grid.segments[0].beats_to_next, Some(2));
        assert!((grid.segments[1].bpm - 150.0).abs() < 1e-9);
        assert_eq!(grid.beat_time_ms(3), Some(1_400));
        assert_eq!(grid.bpm_at(1_400), Some(150.0));
    }

    #[test]
    fn analysis_grid_aligns_only_equivalent_first_segments() {
        let json = r#"{"beats":[0.0,0.5,1.0,1.4,1.8,2.2],"bars":"ignored","downbeats":[0.0]}"#;
        let original = from_analysis_json(json, Some(128.0), None, Some(3_000))
            .unwrap()
            .unwrap();
        let aligned = from_analysis_json(json, Some(128.0), Some(120.1), Some(3_000))
            .unwrap()
            .unwrap();
        assert_eq!(aligned.bpm(), Some(120.1));
        assert_eq!(aligned.segments[1..], original.segments[1..]);
        assert_eq!(aligned.covered_to_ms, Some(3_000));
        assert!(from_analysis_json(json, Some(128.0), Some(78.0), None)
            .unwrap()
            .is_none());
    }

    #[test]
    fn analysis_constant_grid_preserves_exact_anchor_and_meter_without_family_repair() {
        let json = r#"{"beats":[0.5,1.0,1.5,2.0],"downbeats":[1.0],"beats_per_bar":3}"#;
        let grid = from_analysis_json(json, Some(120.0), Some(120.25), None)
            .unwrap()
            .unwrap();
        assert_eq!(grid.bpm(), Some(120.25));
        assert_eq!(grid.first_beat_ms(), Some(500));
        assert_eq!(grid.segments[0].beats_per_bar, 3);
        assert_eq!(grid.segments[0].downbeat_offset_beats, 1);
    }

    #[test]
    fn legacy_malformed_json_is_an_error() {
        assert!(from_legacy_json("{not json", Some(120.0), "analysis", None).is_err());
    }

    fn insert_test_track(db: &Db, id: &str) {
        db.conn()
            .unwrap()
            .as_connection()
            .execute_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("INSERT INTO {} (id, source_type, source_ref, added_at, updated_at) VALUES (?, 'file', ?, 'now', 'now')", crate::db::TRACKS_TABLE),
                [id.into(), id.into()],
            ))
            .unwrap();
    }

    fn track_bpm(db: &Db, id: &str) -> Option<f64> {
        db.conn()
            .unwrap()
            .as_connection()
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!("SELECT bpm FROM {} WHERE id = ?", crate::db::TRACKS_TABLE),
                [id.into()],
            ))
            .unwrap()
            .and_then(|row| row.try_get::<Option<f64>>("", "bpm").unwrap())
    }

    fn segment_count(db: &Db, id: &str) -> i64 {
        db.conn()
            .unwrap()
            .as_connection()
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                format!(
                    "SELECT COUNT(*) AS count FROM {} WHERE track_id = ?",
                    crate::db::TRACK_BEAT_SEGMENT_TABLE
                ),
                [id.into()],
            ))
            .unwrap()
            .unwrap()
            .try_get("", "count")
            .unwrap()
    }

    #[test]
    fn canonical_grid_round_trips_and_updates_tracks_bpm() {
        let db = crate::db::open_in_memory_unmigrated().unwrap();
        insert_test_track(&db, "t1");
        let grid = CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: Some(90_000),
            segments: vec![
                seg(0, 0, 120.0, 0, Some(16), 0),
                seg(1, 8_000, 140.0, 16, None, 2),
            ],
        };

        write_track_beat_grid(&db, &"t1".into(), &grid, &grid.source).unwrap();
        assert_eq!(get_track_beat_grid(&db, &"t1".into()).unwrap(), Some(grid));
        assert_eq!(track_bpm(&db, "t1"), Some(120.0));
    }

    #[test]
    fn canonical_delete_removes_segments() {
        let db = crate::db::open_in_memory_unmigrated().unwrap();
        insert_test_track(&db, "t1");
        write_track_beat_grid(&db, &"t1".into(), &constant(), SOURCE_ANALYSIS).unwrap();
        delete_track_beat_grid(&db, &"t1".into()).unwrap();
        assert!(!has_track_beat_grid_tx(db.conn().unwrap().as_connection(), &"t1".into()).unwrap());
        assert_eq!(segment_count(&db, "t1"), 0);
    }
}
