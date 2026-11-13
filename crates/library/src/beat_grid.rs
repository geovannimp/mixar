//! Canonical beat-grid model: constant and segmented grids in one shape.
//!
//! A grid is a header plus ordered segments. Segment 0 starts at beat 0. Every
//! non-last segment carries the number of beats until the next segment; only the
//! last segment may be terminal (`beats_to_next = None`). Times are integer
//! milliseconds so the database and UI agree exactly.

use std::fmt;

use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, Statement,
};

use crate::db::Db;
use crate::entity::{
    track_beat_grid, track_beat_segment, tracks, TrackBeatGridEntity, TrackBeatSegmentEntity,
    TrackEntity,
};
use crate::{LibraryError, Result, TrackId};

pub const SOURCE_ANALYSIS: &str = "analysis";
pub const SOURCE_MANUAL: &str = "manual";
pub const SOURCE_IMPORT: &str = "import";

/// One constant-tempo run inside a track.
#[derive(Clone, Debug, PartialEq)]
pub struct BeatSegment {
    pub segment_index: i32,
    pub start_time_ms: i64,
    pub bpm: f64,
    pub beats_per_bar: u8,
    pub beat_unit: u8,
    pub start_beat_index: i64,
    pub beats_to_next: Option<i64>,
    pub downbeat_offset_beats: u8,
}

/// A track's canonical grid.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalBeatGrid {
    pub source: String,
    pub covered_to_ms: Option<i64>,
    pub segments: Vec<BeatSegment>,
}

/// Invalid or empty canonical grid.
#[derive(Debug, PartialEq, Eq)]
pub enum GridError {
    Empty,
    Invalid(String),
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "beat grid is empty"),
            Self::Invalid(message) => write!(f, "invalid beat grid: {message}"),
        }
    }
}

impl std::error::Error for GridError {}

impl CanonicalBeatGrid {
    pub fn first_segment(&self) -> Option<&BeatSegment> {
        self.segments.first()
    }

    pub fn bpm(&self) -> Option<f64> {
        self.first_segment().map(|segment| segment.bpm)
    }

    pub fn first_beat_ms(&self) -> Option<i64> {
        self.first_segment().map(|segment| segment.start_time_ms)
    }

    pub fn beat_ms(&self) -> Option<f64> {
        self.first_segment().map(|segment| 60_000.0 / segment.bpm)
    }

    pub fn bar_ms(&self) -> Option<f64> {
        self.first_segment()
            .map(|segment| f64::from(segment.beats_per_bar) * 60_000.0 / segment.bpm)
    }

    pub fn first_downbeat_ms(&self) -> Option<i64> {
        self.first_segment().map(|segment| {
            (segment.start_time_ms as f64
                + f64::from(segment.downbeat_offset_beats) * 60_000.0 / segment.bpm)
                .round() as i64
        })
    }

    /// The segment that owns `ms`; before the grid starts, segment 0.
    pub fn segment_at(&self, ms: i64) -> Option<&BeatSegment> {
        let mut chosen = self.segments.first()?;
        for segment in &self.segments {
            if segment.start_time_ms <= ms {
                chosen = segment;
            } else {
                break;
            }
        }
        Some(chosen)
    }

    pub fn bpm_at(&self, ms: i64) -> Option<f64> {
        self.segment_at(ms).map(|segment| segment.bpm)
    }

    /// Time of a beat index, or `None` for a negative index or a gap.
    pub fn beat_time_ms(&self, beat_index: i64) -> Option<i64> {
        if beat_index < 0 {
            return None;
        }
        let mut chosen = self.segments.first()?;
        for segment in &self.segments {
            if segment.start_beat_index <= beat_index {
                chosen = segment;
            } else {
                break;
            }
        }
        if let Some(count) = chosen.beats_to_next {
            let end = chosen.start_beat_index + count;
            if beat_index >= end {
                let next = self
                    .segments
                    .iter()
                    .find(|segment| segment.start_beat_index == end)?;
                return Some(beat_time_in_segment(
                    next,
                    beat_index - next.start_beat_index,
                ));
            }
        }
        Some(beat_time_in_segment(
            chosen,
            beat_index - chosen.start_beat_index,
        ))
    }

    /// Fractional beat index at `ms`; before the grid starts, this is negative.
    pub fn beat_index_at(&self, ms: i64) -> Option<f64> {
        let segment = self.segment_at(ms)?;
        Some(
            segment.start_beat_index as f64
                + (ms - segment.start_time_ms) as f64 * segment.bpm / 60_000.0,
        )
    }

    /// Beat times from the grid start up to `until_ms`, capped at `max_beats`.
    pub fn materialize_beats(&self, until_ms: i64, max_beats: usize) -> Vec<i64> {
        self.materialize_beat_times(until_ms, max_beats)
            .into_iter()
            .map(|time_ms| time_ms.round() as i64)
            .collect()
    }

    /// Beat times in seconds, without millisecond rounding, for exporters.
    pub fn materialize_beat_secs(&self, until_ms: i64, max_beats: usize) -> Vec<f32> {
        self.materialize_beat_times(until_ms, max_beats)
            .into_iter()
            .map(|time_ms| (time_ms / 1000.0) as f32)
            .collect()
    }

    fn materialize_beat_times(&self, until_ms: i64, max_beats: usize) -> Vec<f64> {
        let mut beats = Vec::new();
        for segment in &self.segments {
            if beats.len() >= max_beats {
                break;
            }
            let count = match segment.beats_to_next {
                Some(count) => count.max(0) as usize,
                None => {
                    let beat_ms = 60_000.0 / segment.bpm;
                    if beat_ms <= 0.0 || until_ms < segment.start_time_ms {
                        break;
                    }
                    let remaining = (until_ms - segment.start_time_ms) as f64;
                    (remaining / beat_ms).floor() as usize + 1
                }
            };
            for offset in 0..count {
                if beats.len() >= max_beats {
                    return beats;
                }
                let time_ms = segment.start_time_ms as f64 + offset as f64 * 60_000.0 / segment.bpm;
                if time_ms > until_ms as f64 {
                    return beats;
                }
                beats.push(time_ms);
            }
        }
        beats
    }

    /// Reject structurally invalid grids before they reach the database.
    pub fn validate(&self) -> std::result::Result<(), GridError> {
        if self.segments.is_empty() {
            return Err(GridError::Empty);
        }
        if self.source.is_empty() {
            return Err(GridError::Invalid("source is empty".into()));
        }
        if self.covered_to_ms.is_some_and(|covered| covered < 0) {
            return Err(GridError::Invalid("covered_to_ms is negative".into()));
        }

        let mut previous: Option<&BeatSegment> = None;
        for (index, segment) in self.segments.iter().enumerate() {
            if segment.segment_index != index as i32 {
                return Err(GridError::Invalid(format!(
                    "segment {index} has index {}",
                    segment.segment_index
                )));
            }
            if segment.start_time_ms < 0 {
                return Err(GridError::Invalid(format!(
                    "segment {index} starts before zero"
                )));
            }
            if segment.start_beat_index < 0 {
                return Err(GridError::Invalid(format!(
                    "segment {index} starts at a negative beat"
                )));
            }
            if !segment.bpm.is_finite() || !(20.0..=400.0).contains(&segment.bpm) {
                return Err(GridError::Invalid(format!(
                    "segment {index} has bpm {}",
                    segment.bpm
                )));
            }
            if !(1..=16).contains(&segment.beats_per_bar) {
                return Err(GridError::Invalid(format!(
                    "segment {index} has {} beats per bar",
                    segment.beats_per_bar
                )));
            }
            if !matches!(segment.beat_unit, 2 | 4 | 8 | 16) {
                return Err(GridError::Invalid(format!(
                    "segment {index} has beat unit {}",
                    segment.beat_unit
                )));
            }
            if segment.downbeat_offset_beats >= segment.beats_per_bar {
                return Err(GridError::Invalid(format!(
                    "segment {index} downbeat offset {} is outside its bar",
                    segment.downbeat_offset_beats
                )));
            }
            if index == 0 && segment.start_beat_index != 0 {
                return Err(GridError::Invalid(
                    "the first segment must start at beat 0".into(),
                ));
            }

            if let Some(previous) = previous {
                let Some(count) = previous.beats_to_next else {
                    return Err(GridError::Invalid(format!(
                        "segment {} is not last but has no beat count",
                        previous.segment_index
                    )));
                };
                if count <= 0 {
                    return Err(GridError::Invalid(format!(
                        "segment {} has a non-positive beat count",
                        previous.segment_index
                    )));
                }
                if segment.start_beat_index != previous.start_beat_index + count {
                    return Err(GridError::Invalid(format!(
                        "segment {} does not continue beat {}",
                        segment.segment_index,
                        previous.start_beat_index + count
                    )));
                }
                if segment.start_time_ms < previous.start_time_ms {
                    return Err(GridError::Invalid(format!(
                        "segment {} starts before segment {}",
                        segment.segment_index, previous.segment_index
                    )));
                }
                let expected =
                    previous.start_time_ms as f64 + count as f64 * 60_000.0 / previous.bpm;
                let tolerance = (60_000.0 / previous.bpm * 0.05).max(2.0);
                if (segment.start_time_ms as f64 - expected).abs() > tolerance {
                    return Err(GridError::Invalid(format!(
                        "segment {} start does not match segment {} tempo",
                        segment.segment_index, previous.segment_index
                    )));
                }
            }
            previous = Some(segment);
        }

        if let Some(last) = self.segments.last() {
            if last.beats_to_next.is_some_and(|count| count <= 0) {
                return Err(GridError::Invalid(
                    "the last segment has a non-positive beat count".into(),
                ));
            }
        }
        Ok(())
    }
}

use serde::Deserialize;

/// Legacy `track_analysis.beat_grid_json` rows. BPM is not stored here.
#[derive(Debug, Deserialize)]
struct LegacyGrid {
    beats: Vec<f32>,
    #[serde(default, rename = "bars")]
    _bars: Vec<f32>,
    #[serde(default)]
    downbeats: Vec<f32>,
    #[serde(default = "default_beats_per_bar")]
    beats_per_bar: u8,
}

fn default_beats_per_bar() -> u8 {
    4
}

/// Build a one-segment constant grid.
pub(crate) fn from_constant(
    bpm: f64,
    first_beat_ms: i64,
    beats_per_bar: u8,
    beat_unit: u8,
    first_downbeat_ms: i64,
    covered_to_ms: Option<i64>,
    source: &str,
) -> CanonicalBeatGrid {
    let beats_per_bar = beats_per_bar.clamp(1, 16);
    let beat_ms = 60_000.0 / bpm;
    let downbeat_offset_beats = if beat_ms.is_finite() && beat_ms > 0.0 {
        (((first_downbeat_ms - first_beat_ms) as f64 / beat_ms).round() as i64)
            .rem_euclid(i64::from(beats_per_bar)) as u8
    } else {
        0
    };
    CanonicalBeatGrid {
        source: source.to_string(),
        covered_to_ms,
        segments: vec![BeatSegment {
            segment_index: 0,
            start_time_ms: first_beat_ms,
            bpm,
            beats_per_bar,
            beat_unit,
            start_beat_index: 0,
            beats_to_next: None,
            downbeat_offset_beats,
        }],
    }
}

/// Convert a legacy JSON grid into canonical segments.
///
/// `fallback_bpm` is the track's stored BPM (then the analysis BPM); it wins for
/// a constant grid. A genuinely variable grid uses each run's median interval.
pub(crate) fn from_legacy_json(
    json: &str,
    fallback_bpm: Option<f64>,
    backend: &str,
    duration_ms: Option<i64>,
) -> std::result::Result<Option<CanonicalBeatGrid>, GridError> {
    let legacy: LegacyGrid = serde_json::from_str(json)
        .map_err(|err| GridError::Invalid(format!("legacy grid JSON: {err}")))?;
    if legacy.beats.len() < 2 {
        return Ok(None);
    }

    let beats: Vec<i64> = legacy
        .beats
        .iter()
        .map(|seconds| (f64::from(*seconds) * 1000.0).round() as i64)
        .collect();
    let downbeats: Vec<i64> = legacy
        .downbeats
        .iter()
        .map(|seconds| (f64::from(*seconds) * 1000.0).round() as i64)
        .collect();
    let intervals: Vec<f64> = beats
        .windows(2)
        .map(|pair| (pair[1] - pair[0]) as f64)
        .collect();
    if intervals.iter().any(|interval| *interval <= 0.0) {
        return Err(GridError::Invalid(
            "legacy grid has non-increasing beat times".into(),
        ));
    }
    let median_interval =
        median(intervals.clone()).ok_or_else(|| GridError::Invalid("no beat intervals".into()))?;
    let bpm = fallback_bpm
        .filter(|bpm| bpm.is_finite() && (20.0..=400.0).contains(bpm))
        .unwrap_or(60_000.0 / median_interval);
    if !bpm.is_finite() || !(20.0..=400.0).contains(&bpm) {
        return Err(GridError::Invalid("legacy grid has no usable BPM".into()));
    }

    let beats_per_bar = legacy.beats_per_bar.clamp(1, 16);
    let source = if backend == "manual" {
        SOURCE_MANUAL
    } else {
        SOURCE_ANALYSIS
    };
    let last_beat = *beats.last().ok_or(GridError::Empty)?;
    let covered_to_ms = if backend == "manual" {
        None
    } else {
        duration_ms.filter(|duration| *duration > last_beat)
    };
    let tolerance = (median_interval * 0.02).max(5.0);
    let constant = intervals
        .iter()
        .all(|interval| (*interval - median_interval).abs() <= tolerance);

    if constant {
        let first_beat_ms = beats[0];
        let offset = downbeat_offset(&beats, &downbeats, 0, bpm, beats_per_bar);
        let grid = CanonicalBeatGrid {
            source: source.to_string(),
            covered_to_ms,
            segments: vec![BeatSegment {
                segment_index: 0,
                start_time_ms: first_beat_ms,
                bpm,
                beats_per_bar,
                beat_unit: 4,
                start_beat_index: 0,
                beats_to_next: None,
                downbeat_offset_beats: offset,
            }],
        };
        grid.validate()?;
        return Ok(Some(grid));
    }

    let mut runs: Vec<(usize, usize, f64)> = Vec::new();
    let mut start = 0usize;
    let mut cursor = 0usize;
    while cursor < intervals.len() {
        if cursor == start {
            cursor += 1;
            continue;
        }
        let run_intervals = &intervals[start..cursor];
        let run_median = median(run_intervals.to_vec()).unwrap_or(median_interval);
        if (intervals[cursor] - run_median).abs() <= tolerance {
            cursor += 1;
        } else {
            runs.push((start, cursor, 60_000.0 / run_median));
            start = cursor;
        }
    }
    let tail = &intervals[start..];
    let tail_bpm = median(tail.to_vec()).map_or(bpm, |median| 60_000.0 / median);
    runs.push((start, beats.len(), tail_bpm));

    let mut segments = Vec::with_capacity(runs.len());
    let run_count = runs.len();
    for (index, (start_beat, end_beat, run_bpm)) in runs.into_iter().enumerate() {
        let is_last = index + 1 == run_count;
        let count = if is_last {
            None
        } else {
            Some((end_beat - start_beat) as i64)
        };
        segments.push(BeatSegment {
            segment_index: index as i32,
            start_time_ms: beats[start_beat],
            bpm: run_bpm,
            beats_per_bar,
            beat_unit: 4,
            start_beat_index: start_beat as i64,
            beats_to_next: count,
            downbeat_offset_beats: downbeat_offset(
                &beats,
                &downbeats,
                start_beat,
                run_bpm,
                beats_per_bar,
            ),
        });
    }

    let grid = CanonicalBeatGrid {
        source: source.to_string(),
        covered_to_ms,
        segments,
    };
    grid.validate()?;
    Ok(Some(grid))
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    })
}

fn downbeat_offset(
    beats: &[i64],
    downbeats: &[i64],
    start_beat: usize,
    bpm: f64,
    beats_per_bar: u8,
) -> u8 {
    let Some(&start_ms) = beats.get(start_beat) else {
        return 0;
    };
    let Some(downbeat_ms) = downbeats.iter().copied().find(|ms| *ms >= start_ms) else {
        return 0;
    };
    let beat_ms = 60_000.0 / bpm;
    if !beat_ms.is_finite() || beat_ms <= 0.0 {
        return 0;
    }
    (((downbeat_ms - start_ms) as f64 / beat_ms).round() as i64)
        .rem_euclid(i64::from(beats_per_bar.max(1))) as u8
}

/// Write a canonical grid and set `tracks.bpm` to segment 0 in one transaction.
pub(crate) fn write_track_beat_grid(
    db: &Db,
    track_id: &TrackId,
    grid: &CanonicalBeatGrid,
) -> Result<()> {
    grid.validate().map_err(grid_error)?;
    let guard = db.conn()?;
    let conn = guard.as_connection();
    begin(conn)?;
    match write_track_beat_grid_tx(conn, track_id, grid) {
        Ok(()) => {
            commit(conn)?;
            Ok(())
        }
        Err(err) => {
            rollback(conn);
            Err(err)
        }
    }
}

/// Same writes as [`write_track_beat_grid`], without beginning a transaction.
///
/// The migration runner already holds one; callers outside a transaction should
/// use [`write_track_beat_grid`].
pub(crate) fn write_track_beat_grid_tx(
    conn: &DatabaseConnection,
    track_id: &TrackId,
    grid: &CanonicalBeatGrid,
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
    let conn = guard.as_connection();
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

#[allow(dead_code)]
pub(crate) fn delete_track_beat_grid(db: &Db, track_id: &TrackId) -> Result<()> {
    TrackBeatGridEntity::delete_by_id(track_id.as_str())
        .exec(db.conn()?.as_connection())
        .map(|_| ())
        .map_err(crate::db::db_err)
}

pub(crate) fn has_track_beat_grid(db: &Db, track_id: &TrackId) -> Result<bool> {
    Ok(TrackBeatGridEntity::find_by_id(track_id.as_str())
        .one(db.conn()?.as_connection())
        .map_err(crate::db::db_err)?
        .is_some())
}

/// Mirror a grid's first BPM onto `tracks.bpm` without rewriting the grid.
pub(crate) fn set_track_bpm(db: &Db, track_id: &TrackId, bpm: f64) -> Result<()> {
    let track = tracks::ActiveModel {
        id: Set(track_id.as_str().to_string()),
        bpm: Set(Some(bpm)),
        ..Default::default()
    };
    TrackEntity::update(track)
        .exec(db.conn()?.as_connection())
        .map(|_| ())
        .map_err(crate::db::db_err)
}

fn grid_error(err: GridError) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: err.to_string(),
    }
}

fn begin(conn: &DatabaseConnection) -> Result<()> {
    conn.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        "BEGIN IMMEDIATE".to_string(),
    ))
    .map(|_| ())
    .map_err(crate::db::db_err)
}

fn commit(conn: &DatabaseConnection) -> Result<()> {
    conn.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        "COMMIT".to_string(),
    ))
    .map(|_| ())
    .map_err(crate::db::db_err)
}

fn rollback(conn: &DatabaseConnection) {
    let _ = conn.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        "ROLLBACK".to_string(),
    ));
}

fn beat_time_in_segment(segment: &BeatSegment, offset: i64) -> i64 {
    (segment.start_time_ms as f64 + offset as f64 * 60_000.0 / segment.bpm).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

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

        write_track_beat_grid(&db, &"t1".into(), &grid).unwrap();
        assert_eq!(get_track_beat_grid(&db, &"t1".into()).unwrap(), Some(grid));
        assert_eq!(track_bpm(&db, "t1"), Some(120.0));
    }

    #[test]
    fn canonical_delete_removes_segments() {
        let db = crate::db::open_in_memory_unmigrated().unwrap();
        insert_test_track(&db, "t1");
        write_track_beat_grid(&db, &"t1".into(), &constant()).unwrap();
        delete_track_beat_grid(&db, &"t1".into()).unwrap();
        assert!(!has_track_beat_grid(&db, &"t1".into()).unwrap());
        assert_eq!(segment_count(&db, "t1"), 0);
    }
}
