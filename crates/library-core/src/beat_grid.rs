//! Canonical beat-grid model: constant and segmented grids in one shape.
//!
//! A grid is a header plus ordered segments. Segment 0 starts at beat 0. Every
//! non-last segment carries the number of beats until the next segment; only the
//! last segment may be terminal (`beats_to_next = None`). Times are integer
//! milliseconds so the database and UI agree exactly.

use std::fmt;

use serde::Deserialize;

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
        self.last_segment_while(|segment| segment.start_time_ms <= ms)
    }

    /// The last segment whose key still fits, the way both lookups above scan.
    fn last_segment_while(&self, fits: impl Fn(&BeatSegment) -> bool) -> Option<&BeatSegment> {
        let mut chosen = self.segments.first()?;
        for segment in &self.segments {
            if fits(segment) {
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
        let chosen = self.last_segment_while(|segment| segment.start_beat_index <= beat_index)?;
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
                let time_ms = beat_offset_ms(segment, offset as i64);
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
            if !is_valid_bpm(segment.bpm) {
                return Err(GridError::Invalid(format!(
                    "segment {index} has bpm {}",
                    segment.bpm
                )));
            }
            if !valid_beats_per_bar(segment.beats_per_bar) {
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

            if let Some(prev) = previous {
                let Some(count) = prev.beats_to_next else {
                    return Err(GridError::Invalid(format!(
                        "segment {} is not last but has no beat count",
                        prev.segment_index
                    )));
                };
                if count <= 0 {
                    return Err(GridError::Invalid(format!(
                        "segment {} has a non-positive beat count",
                        prev.segment_index
                    )));
                }
                if segment.start_beat_index != prev.start_beat_index + count {
                    return Err(GridError::Invalid(format!(
                        "segment {} does not continue beat {}",
                        segment.segment_index,
                        prev.start_beat_index + count
                    )));
                }
                if segment.start_time_ms < prev.start_time_ms {
                    return Err(GridError::Invalid(format!(
                        "segment {} starts before segment {}",
                        segment.segment_index, prev.segment_index
                    )));
                }
                let expected = beat_offset_ms(prev, count);
                let tolerance = (60_000.0 / prev.bpm * 0.05).max(2.0);
                if (segment.start_time_ms as f64 - expected).abs() > tolerance {
                    return Err(GridError::Invalid(format!(
                        "segment {} start does not match segment {} tempo",
                        segment.segment_index, prev.segment_index
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

/// Legacy `track_analysis.beat_grid_json` rows. BPM is not stored here.
#[derive(Debug, Deserialize)]
struct LegacyGrid {
    beats: Vec<f32>,
    #[serde(default)]
    downbeats: Vec<f32>,
    #[serde(default = "default_beats_per_bar")]
    beats_per_bar: u8,
}

fn default_beats_per_bar() -> u8 {
    4
}

/// Build a one-segment constant grid.
pub fn from_constant(
    bpm: f64,
    first_beat_ms: i64,
    beats_per_bar: u8,
    beat_unit: u8,
    first_downbeat_ms: i64,
    covered_to_ms: Option<i64>,
    source: &str,
) -> CanonicalBeatGrid {
    let beats_per_bar = clamp_beats_per_bar(beats_per_bar);
    let downbeat_offset_beats =
        downbeat_offset_between(first_beat_ms, first_downbeat_ms, bpm, beats_per_bar);
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
pub fn from_legacy_json(
    json: &str,
    fallback_bpm: Option<f64>,
    backend: &str,
    duration_ms: Option<i64>,
) -> std::result::Result<Option<CanonicalBeatGrid>, GridError> {
    let legacy = parse_legacy_grid(json)?;
    compact_legacy_grid(&legacy, fallback_bpm, backend, duration_ms)
}

fn parse_legacy_grid(json: &str) -> std::result::Result<LegacyGrid, GridError> {
    serde_json::from_str(json).map_err(|err| GridError::Invalid(format!("legacy grid JSON: {err}")))
}

fn compact_legacy_grid(
    legacy: &LegacyGrid,
    fallback_bpm: Option<f64>,
    backend: &str,
    duration_ms: Option<i64>,
) -> std::result::Result<Option<CanonicalBeatGrid>, GridError> {
    if legacy.beats.len() < 2 {
        return Ok(None);
    }

    let beats = legacy_times_ms(&legacy.beats)?;
    let downbeats = legacy_times_ms(&legacy.downbeats)?;
    if beats[0] < 0 {
        return Err(GridError::Invalid(
            "legacy grid has negative beat times".into(),
        ));
    }
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
        .filter(|bpm| is_valid_bpm(*bpm))
        .unwrap_or(60_000.0 / median_interval);
    if !is_valid_bpm(bpm) {
        return Err(GridError::Invalid("legacy grid has no usable BPM".into()));
    }

    let beats_per_bar = clamp_beats_per_bar(legacy.beats_per_bar);
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
        let downbeat_ms = first_downbeat_at(&beats, &downbeats, 0).unwrap_or(first_beat_ms);
        let grid = from_constant(
            bpm,
            first_beat_ms,
            beats_per_bar,
            4,
            downbeat_ms,
            covered_to_ms,
            source,
        );
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
    let tail_bpm = median(tail.to_vec()).map_or(bpm, |interval| 60_000.0 / interval);
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

fn legacy_times_ms(seconds: &[f32]) -> std::result::Result<Vec<i64>, GridError> {
    seconds
        .iter()
        .map(|value| {
            let ms = (f64::from(*value) * 1000.0).round();
            if !ms.is_finite() || ms < i64::MIN as f64 || ms >= i64::MAX as f64 {
                return Err(GridError::Invalid(
                    "legacy grid has an invalid timestamp".into(),
                ));
            }
            Ok(ms as i64)
        })
        .collect()
}

/// Compact detector markers at the winning tempo without changing the audit JSON.
pub fn from_analysis_json(
    json: &str,
    detector_bpm: Option<f64>,
    preferred_bpm: Option<f64>,
    covered_to_ms: Option<i64>,
) -> std::result::Result<Option<CanonicalBeatGrid>, GridError> {
    if preferred_bpm.is_some_and(|bpm| !is_valid_bpm(bpm)) {
        return Ok(None);
    }
    let legacy = parse_legacy_grid(json)?;
    let Some(grid) = compact_legacy_grid(&legacy, detector_bpm, SOURCE_ANALYSIS, covered_to_ms)?
    else {
        return Ok(None);
    };
    let Some(preferred) = preferred_bpm else {
        return Ok(Some(grid));
    };
    if grid.segments.len() > 1 {
        return Ok(align_segmented_bpm(grid, preferred));
    }

    let detected = grid.segments[0].bpm;
    if is_family_mismatch(detected, preferred) {
        if let Some((beats, downbeats)) = analyzer_core::rebuild_grid_at(
            &legacy.beats,
            &legacy.downbeats,
            preferred,
            clamp_beats_per_bar(legacy.beats_per_bar),
        ) {
            let rebuilt = LegacyGrid {
                beats,
                downbeats,
                beats_per_bar: legacy.beats_per_bar,
            };
            if let Ok(Some(mut repaired)) =
                compact_legacy_grid(&rebuilt, Some(preferred), SOURCE_ANALYSIS, covered_to_ms)
            {
                if repaired.segments.len() == 1 {
                    repaired.covered_to_ms = grid.covered_to_ms;
                    return Ok(Some(repaired));
                }
            }
        }
    }
    // A non-family change or failed rebuild uses the original first beat and
    // projects the downbeat onto the preferred constant tempo.
    compact_legacy_grid(&legacy, Some(preferred), SOURCE_ANALYSIS, covered_to_ms)
}

/// Meters the model accepts; 1/4 through 16/4.
const BEATS_PER_BAR_MIN: u8 = 1;
const BEATS_PER_BAR_MAX: u8 = 16;

fn clamp_beats_per_bar(beats_per_bar: u8) -> u8 {
    beats_per_bar.clamp(BEATS_PER_BAR_MIN, BEATS_PER_BAR_MAX)
}

fn valid_beats_per_bar(beats_per_bar: u8) -> bool {
    (BEATS_PER_BAR_MIN..=BEATS_PER_BAR_MAX).contains(&beats_per_bar)
}

/// Tempos outside this window describe a mistyped or unanalyzed track.
fn is_valid_bpm(bpm: f64) -> bool {
    bpm.is_finite() && (20.0..=400.0).contains(&bpm)
}

pub fn is_family_mismatch(detected: f64, preferred: f64) -> bool {
    (detected - preferred).abs() > 0.5
        && analyzer_core::is_metrical_tempo_relation(detected, preferred)
}

/// Change only the first tempo, and only when a recognizably equivalent
/// segment remains consistent with the next segment's timestamp.
pub fn align_segmented_bpm(
    mut grid: CanonicalBeatGrid,
    preferred_bpm: f64,
) -> Option<CanonicalBeatGrid> {
    let first = grid.segments.first_mut()?;
    if (first.bpm - preferred_bpm).abs() > 0.5 {
        return None;
    }
    first.bpm = preferred_bpm;
    grid.validate().ok()?;
    Some(grid)
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
    let Some(downbeat_ms) = first_downbeat_at(beats, downbeats, start_beat) else {
        return 0;
    };
    downbeat_offset_between(start_ms, downbeat_ms, bpm, beats_per_bar)
}

/// First downbeat marker at or after `beats[start_beat]`.
fn first_downbeat_at(beats: &[i64], downbeats: &[i64], start_beat: usize) -> Option<i64> {
    let start_ms = *beats.get(start_beat)?;
    downbeats.iter().copied().find(|ms| *ms >= start_ms)
}

/// Bar-relative position of `downbeat_ms`, counted from `start_ms` at `bpm`.
///
/// Clockwise within the bar, so a downbeat before the first beat still lands on
/// a position rather than a negative one.
fn downbeat_offset_between(start_ms: i64, downbeat_ms: i64, bpm: f64, beats_per_bar: u8) -> u8 {
    let beat_ms = 60_000.0 / bpm;
    if !beat_ms.is_finite() || beat_ms <= 0.0 {
        return 0;
    }
    (((downbeat_ms - start_ms) as f64 / beat_ms).round() as i64)
        .rem_euclid(i64::from(beats_per_bar.max(1))) as u8
}

/// Time of `offset` beats into `segment`, unrounded.
fn beat_offset_ms(segment: &BeatSegment, offset: i64) -> f64 {
    segment.start_time_ms as f64 + offset as f64 * 60_000.0 / segment.bpm
}

fn beat_time_in_segment(segment: &BeatSegment, offset: i64) -> i64 {
    beat_offset_ms(segment, offset).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_grid_is_valid_and_keeps_its_tempo() {
        let grid = from_constant(120.0, 1_000, 4, 4, 1_000, Some(60_000), SOURCE_ANALYSIS);

        grid.validate().expect("a constant grid validates");
        assert_eq!(grid.bpm(), Some(120.0));
        assert_eq!(grid.segments.len(), 1);
        assert_eq!(grid.segments[0].downbeat_offset_beats, 0);
    }

    #[test]
    fn legacy_constant_grid_uses_the_fallback_tempo() {
        let json = r#"{"beats":[0.0,0.5,1.0],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#;

        let grid = from_legacy_json(json, Some(117.0), SOURCE_ANALYSIS, None)
            .expect("the grid parses")
            .expect("three beats make a grid");

        assert_eq!(grid.bpm(), Some(117.0), "the stored tempo wins");
    }

    #[test]
    fn legacy_grid_with_non_increasing_beats_is_rejected() {
        let json = r#"{"beats":[0.0,0.5,0.4],"bars":[],"downbeats":[]}"#;

        assert!(
            from_legacy_json(json, None, SOURCE_ANALYSIS, None).is_err(),
            "a grid that goes backwards cannot be converted"
        );
    }

    const ANALYSIS_JSON: &str =
        r#"{"beats":[0.0,0.5,1.0],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#;

    #[test]
    fn analysis_grid_rejects_an_out_of_range_preferred_tempo() {
        assert!(
            from_analysis_json(ANALYSIS_JSON, Some(120.0), Some(10.0), None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn analysis_grid_rejects_a_non_finite_preferred_tempo() {
        assert!(
            from_analysis_json(ANALYSIS_JSON, Some(120.0), Some(f64::NAN), None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn analysis_grid_keeps_the_detector_tempo_without_a_preference() {
        let grid = from_analysis_json(ANALYSIS_JSON, Some(120.0), None, None)
            .unwrap()
            .unwrap();

        assert_eq!(grid.bpm(), Some(120.0));
    }

    #[test]
    fn analysis_grid_projects_a_near_preference() {
        let grid = from_analysis_json(ANALYSIS_JSON, Some(120.0), Some(121.0), None)
            .unwrap()
            .unwrap();

        assert_eq!(grid.segments.len(), 1);
        assert_eq!(grid.bpm(), Some(121.0));
    }

    #[test]
    fn analysis_grid_rebuilds_a_half_tempo_preference() {
        let grid = from_analysis_json(ANALYSIS_JSON, Some(120.0), Some(60.0), None)
            .unwrap()
            .unwrap();

        assert_eq!(grid.bpm(), Some(60.0));
    }

    #[test]
    fn alignment_adopts_a_tempo_within_a_half_bpm() {
        let grid = from_constant(120.0, 0, 4, 4, 0, None, SOURCE_ANALYSIS);

        let aligned = align_segmented_bpm(grid, 120.4).expect("half a BPM is the same tempo");

        assert_eq!(aligned.bpm(), Some(120.4));
    }

    #[test]
    fn alignment_rejects_a_distant_tempo() {
        let grid = from_constant(120.0, 0, 4, 4, 0, None, SOURCE_ANALYSIS);

        assert!(align_segmented_bpm(grid, 124.0).is_none());
    }

    const SEGMENTED_JSON: &str = r#"{"beats":[0.0,0.5,1.0,1.5,2.0,2.4,2.8,3.2],"bars":[0.0],"downbeats":[0.0],"beats_per_bar":4}"#;

    #[test]
    fn legacy_variable_grid_splits_into_segments() {
        let grid = from_legacy_json(SEGMENTED_JSON, None, SOURCE_ANALYSIS, None)
            .unwrap()
            .unwrap();

        assert_eq!(grid.segments.len(), 2);
        assert_eq!(grid.segments[0].bpm, 120.0);
        assert_eq!(grid.segments[1].start_beat_index, 4);
        assert_eq!(grid.segments[1].start_time_ms, 2_000);
        assert_eq!(grid.segments[1].bpm, 150.0);
        grid.validate().expect("both halves are consistent");
    }

    #[test]
    fn beat_times_cross_a_segment_boundary() {
        let grid = from_legacy_json(SEGMENTED_JSON, None, SOURCE_ANALYSIS, None)
            .unwrap()
            .unwrap();

        assert_eq!(grid.beat_time_ms(3), Some(1_500));
        assert_eq!(grid.beat_time_ms(4), Some(2_000));
        assert_eq!(grid.beat_time_ms(5), Some(2_400));
        assert_eq!(grid.beat_time_ms(-1), None);
    }

    #[test]
    fn materialized_beats_stop_at_the_cap() {
        let grid = from_constant(120.0, 0, 4, 4, 0, Some(10_000), SOURCE_ANALYSIS);

        assert_eq!(grid.materialize_beats(10_000, 3), vec![0, 500, 1_000]);
        assert_eq!(grid.materialize_beats(1_000, 100), vec![0, 500, 1_000]);
    }

    #[test]
    fn validation_rejects_a_segment_gap() {
        let mut grid = from_constant(120.0, 0, 4, 4, 0, None, SOURCE_ANALYSIS);
        grid.segments[0].beats_to_next = Some(4);
        grid.segments.push(BeatSegment {
            segment_index: 1,
            start_time_ms: 2_000,
            bpm: 120.0,
            beats_per_bar: 4,
            beat_unit: 4,
            start_beat_index: 8,
            beats_to_next: None,
            downbeat_offset_beats: 0,
        });

        assert!(grid.validate().is_err(), "beat 4 is missing");
    }
}
