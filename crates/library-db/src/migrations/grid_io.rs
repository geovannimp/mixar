//! Canonical-grid rows for migrations.
//!
//! The application's grid reader and writer carry product policy (BPM mirroring
//! rules, source ownership). Migrations only need the rows, so they use these
//! frozen operations and keep working when the policy moves on.

use library_core::{BeatSegment, CanonicalBeatGrid, TrackId};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set};

use super::migration_error as err;
use super::MigrationError;
use crate::entity::{
    track_beat_grid, track_beat_segment, tracks, TrackBeatGridEntity, TrackBeatSegmentEntity,
    TrackEntity,
};

/// A stored meter value, narrowed to the model's byte range.
fn narrow_u8(version: &'static str, value: i32) -> Result<u8, MigrationError> {
    u8::try_from(value).map_err(|cause| err(version, cause))
}

/// Read the header and its segments, if the track has a grid.
pub fn load_grid(
    conn: &DatabaseConnection,
    version: &'static str,
    track_id: &TrackId,
) -> Result<Option<CanonicalBeatGrid>, MigrationError> {
    let Some(header) = TrackBeatGridEntity::find_by_id(track_id.as_str())
        .one(conn)
        .map_err(|e| err(version, e))?
    else {
        return Ok(None);
    };
    let rows = TrackBeatSegmentEntity::find()
        .filter(track_beat_segment::Column::TrackId.eq(track_id.as_str()))
        .order_by_asc(track_beat_segment::Column::SegmentIndex)
        .all(conn)
        .map_err(|e| err(version, e))?;

    let segments = rows
        .into_iter()
        .map(|row| {
            Ok(BeatSegment {
                segment_index: row.segment_index,
                start_time_ms: row.start_time_ms,
                bpm: row.bpm,
                beats_per_bar: narrow_u8(version, row.beats_per_bar)?,
                beat_unit: narrow_u8(version, row.beat_unit)?,
                start_beat_index: row.start_beat_index,
                beats_to_next: row.beats_to_next,
                downbeat_offset_beats: narrow_u8(version, row.downbeat_offset_beats)?,
            })
        })
        .collect::<Result<Vec<_>, MigrationError>>()?;

    Ok(Some(CanonicalBeatGrid {
        source: header.source,
        covered_to_ms: header.covered_to_ms,
        segments,
    }))
}

/// Replace a track's grid, keeping the header's original `created_at`, and
/// mirror segment 0's tempo onto the track.
pub fn write_grid(
    conn: &DatabaseConnection,
    version: &'static str,
    track_id: &TrackId,
    grid: &CanonicalBeatGrid,
    bpm_source: &str,
) -> Result<(), MigrationError> {
    grid.validate().map_err(|e| err(version, e))?;
    let now = jiff::Timestamp::now()
        .display_with_offset(jiff::tz::Offset::UTC)
        .to_string();

    delete_segments(conn, version, track_id)?;

    TrackBeatGridEntity::insert(track_beat_grid::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        source: Set(grid.source.clone()),
        covered_to_ms: Set(grid.covered_to_ms),
        created_at: Set(now.clone()),
        updated_at: Set(now.clone()),
    })
    // `created_at` stays out of the update set: a rewrite keeps the moment the
    // grid first appeared, only the shape and `updated_at` move.
    .on_conflict(
        sea_orm::sea_query::OnConflict::column(track_beat_grid::Column::TrackId)
            .update_columns([
                track_beat_grid::Column::Source,
                track_beat_grid::Column::CoveredToMs,
                track_beat_grid::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(conn)
    .map_err(|e| err(version, e))?;

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
        .map_err(|e| err(version, e))?;

    let Some(first) = grid.segments.first() else {
        return Ok(());
    };
    TrackEntity::update(tracks::ActiveModel {
        id: Set(track_id.as_str().to_string()),
        bpm: Set(Some(first.bpm)),
        bpm_source: Set(Some(bpm_source.to_string())),
        ..Default::default()
    })
    .exec(conn)
    .map_err(|e| err(version, e))?;
    Ok(())
}

/// Remove a track's grid header and segments.
pub fn delete_grid(
    conn: &DatabaseConnection,
    version: &'static str,
    track_id: &TrackId,
) -> Result<(), MigrationError> {
    delete_segments(conn, version, track_id)?;
    TrackBeatGridEntity::delete_by_id(track_id.as_str())
        .exec(conn)
        .map(|_| ())
        .map_err(|e| err(version, e))
}

/// Drop every segment a track owns, so a rewrite starts from a clean slate.
fn delete_segments(
    conn: &DatabaseConnection,
    version: &'static str,
    track_id: &TrackId,
) -> Result<(), MigrationError> {
    TrackBeatSegmentEntity::delete_many()
        .filter(track_beat_segment::Column::TrackId.eq(track_id.as_str()))
        .exec(conn)
        .map(|_| ())
        .map_err(|e| err(version, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    fn column_name(column: impl sea_orm::ColumnTrait) -> String {
        column.as_column_ref().1.to_string()
    }

    #[test]
    fn rewrite_keeps_the_original_created_at() {
        let db = crate::db::open_in_memory().unwrap();
        let handle = db.conn().unwrap();
        let conn = handle.as_connection();
        use sea_orm::EntityName;

        let tracks_table = crate::entity::tracks::Entity.table_name();
        let id = column_name(crate::entity::tracks::Column::Id);
        let source_type = column_name(crate::entity::tracks::Column::SourceType);
        let source_ref = column_name(crate::entity::tracks::Column::SourceRef);
        let added_at = column_name(crate::entity::tracks::Column::AddedAt);
        let updated_at = column_name(crate::entity::tracks::Column::UpdatedAt);
        conn.execute_unprepared(&format!(
            "INSERT INTO {tracks_table} ({id}, {source_type}, {source_ref}, {added_at}, {updated_at}) \
             VALUES ('t1', '{}', 't1', 'old', 'old')",
            library_core::SOURCE_TYPE_FILE
        ))
        .unwrap();

        let grid_table = crate::entity::track_beat_grid::Entity.table_name();
        let created_at = column_name(crate::entity::track_beat_grid::Column::CreatedAt);
        let grid_track_id = column_name(crate::entity::track_beat_grid::Column::TrackId);
        let grid =
            library_core::from_constant(120.0, 0, 4, 4, 0, None, library_core::SOURCE_ANALYSIS);

        write_grid(
            conn,
            "test",
            &"t1".into(),
            &grid,
            library_core::SOURCE_ANALYSIS,
        )
        .unwrap();
        conn.execute_unprepared(&format!(
            "UPDATE {grid_table} SET {created_at} = '2001-01-01T00:00:00+00:00'"
        ))
        .unwrap();

        write_grid(
            conn,
            "test",
            &"t1".into(),
            &grid,
            library_core::SOURCE_ANALYSIS,
        )
        .unwrap();

        let row = conn
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("SELECT {created_at} FROM {grid_table} WHERE {grid_track_id} = 't1'"),
            ))
            .unwrap()
            .unwrap();
        assert_eq!(
            row.try_get::<String>("", created_at.as_str()).unwrap(),
            "2001-01-01T00:00:00+00:00"
        );
    }
}
