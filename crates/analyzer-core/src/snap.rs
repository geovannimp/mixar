//! Post-process a raw detected beat list into a clean constant-tempo grid.
//!
//! Backends return whatever beats they detected: noisy onsets (stratum), a
//! near-grid (rosa/beat-this), sometimes the wrong tempo octave (half/double),
//! and often a fractional BPM that drifts against the rest of the world.
//!
//! [`snap_grid`] turns that into what a DJ tool actually wants:
//!
//! 1. take the backend's reported tempo (falling back to the median interval),
//! 2. fold it into a canonical range by octaves (undo half/double-time),
//! 3. round it to `SnapConfig::decimals` decimal places,
//! 4. re-anchor a constant grid to best fit the detected beats (Mixxx-style
//!    phase adjustment).
//!
//! This mirrors what Mixxx does when it reduces the QM detector's beats to a
//! constant `BeatGrid`. It cannot recover a wrong metrical family (e.g. a 3:2
//! lock); that needs a better detector.

use crate::config::SnapConfig;
use crate::result::{BeatGridAnalysis, TrackAnalysis};

/// Rewrite `track`'s beat grid in place. No-op when disabled, when there is no
/// grid, or when there are too few beats to estimate a tempo.
pub fn snap_grid(track: &mut TrackAnalysis, cfg: &SnapConfig) {
    if !cfg.enabled {
        return;
    }
    let Some(grid) = track.beat_grid.as_ref() else {
        return;
    };
    if grid.beats.is_empty() || grid.beats.len() < cfg.min_beats {
        return;
    }

    // Prefer the backend's own tempo estimate; fall back to the median interval
    // (the beat list can be noisier than the reported BPM, e.g. stratum).
    let mut bpm = match track.bpm.as_ref().map(|b| b.bpm) {
        Some(bpm) if bpm.is_finite() && bpm > 0.0 => bpm,
        _ => {
            let Some(period) = median_period(&grid.beats) else {
                return;
            };
            60.0 / period
        }
    };
    if !bpm.is_finite() || bpm <= 0.0 {
        return;
    }

    // Fold into the canonical range by octaves (undo half/double-time).
    let min_bpm = cfg.min_bpm.max(1.0);
    let max_bpm = cfg.max_bpm.max(min_bpm + 1.0);
    while bpm < min_bpm {
        bpm *= 2.0;
    }
    while bpm >= max_bpm {
        bpm /= 2.0;
    }
    // A window narrower than an octave (`max_bpm < 2 * min_bpm`) can leave `bpm`
    // below `min_bpm` after folding; clamp so the documented range holds.
    bpm = bpm.clamp(min_bpm, max_bpm.max(min_bpm));

    // Round to the configured number of decimal places. Rounding can push a
    // folded value back up to (or over) the exclusive upper bound, so fold again
    // and clamp into `[min_bpm, max_bpm)`.
    let snapped = round_to_decimals(bpm, cfg.decimals);
    if !snapped.is_finite() || snapped <= 0.0 {
        return;
    }
    let snapped = if snapped >= max_bpm {
        snapped / 2.0
    } else {
        snapped
    }
    .max(min_bpm);
    let snapped_period = 60.0 / snapped;

    // Best-fit anchor (Mixxx `adjustPhase`).
    let anchor = fit_anchor(&grid.beats, snapped_period);

    let first = f64::from(grid.beats[0]);
    let last = f64::from(*grid.beats.last().unwrap());
    let k0 = ((first - anchor) / snapped_period).floor() as i64;
    let k1 = ((last - anchor) / snapped_period).ceil() as i64;
    // Guard against a misconfigured (very high) `max_bpm` producing an enormous grid.
    const MAX_GRID_BEATS: i64 = 1_000_000;
    if k1 - k0 + 1 > MAX_GRID_BEATS {
        return;
    }

    // Downbeat phase: the bar position the detected downbeats agree on.
    let beats_per_bar = grid.beats_per_bar.max(1);
    let phase = downbeat_phase(grid, i64::from(beats_per_bar));

    let mut beats: Vec<f32> = Vec::with_capacity((k1 - k0 + 1).max(0) as usize);
    let mut downbeats: Vec<f32> = Vec::new();
    for k in k0..=k1 {
        let t = anchor + k as f64 * snapped_period;
        if t < 0.0 {
            continue;
        }
        let t = t as f32;
        beats.push(t);
        if k.rem_euclid(i64::from(beats_per_bar)) == phase {
            downbeats.push(t);
        }
    }
    if beats.is_empty() {
        return;
    }

    // Keep the backend's own stability — recomputing it against the changed
    // period would be dominated by the tiny BPM edit over a full track.
    let stability = grid.grid_stability;

    track.beat_grid = Some(BeatGridAnalysis {
        beats,
        bars: downbeats.clone(),
        downbeats,
        grid_stability: stability,
        beats_per_bar,
    });
    if let Some(bpm) = track.bpm.as_mut() {
        bpm.bpm = snapped;
    }
}

/// Round to `decimals` decimal places, half-to-even (so `124.25 -> 124.2` and
/// `124.05 -> 124.0`). Matches the previous formatter-based behavior without
/// the per-call `String` allocation.
fn round_to_decimals(x: f64, decimals: u8) -> f64 {
    let factor = 10f64.powi(i32::from(decimals));
    (x * factor).round_ties_even() / factor
}

fn median_period(beats: &[f32]) -> Option<f64> {
    let mut intervals: Vec<f64> = beats
        .windows(2)
        .map(|w| f64::from(w[1] - w[0]))
        .filter(|d| *d > 0.0)
        .collect();
    if intervals.is_empty() {
        return None;
    }
    intervals.sort_by(f64::total_cmp);
    let median = intervals[intervals.len() / 2];
    (median > 0.0 && median.is_finite()).then_some(median)
}

/// Best-fit phase anchor for `period`: first beat shifted by the mean residual
/// of the inlier beats (±25 ms), falling back to the median residual.
fn fit_anchor(beats: &[f32], period: f64) -> f64 {
    let first = f64::from(beats[0]);
    let mut residuals: Vec<f64> = beats
        .iter()
        .map(|&b| {
            let b = f64::from(b);
            let k = ((b - first) / period).round();
            b - (first + k * period)
        })
        .collect();
    let inliers: Vec<f64> = residuals
        .iter()
        .copied()
        .filter(|r| r.abs() <= 0.025)
        .collect();
    if inliers.is_empty() {
        residuals.sort_by(f64::total_cmp);
        first + residuals[residuals.len() / 2]
    } else {
        first + inliers.iter().sum::<f64>() / inliers.len() as f64
    }
}

/// Bar phase of the detected downbeats: locate each downbeat's position in the
/// backend's *own* beat list and take the most common index `mod beats_per_bar`.
/// Matching by nearest beat (rather than a period estimate) keeps the phase
/// stable when the raw beats and the snapped BPM differ.
fn downbeat_phase(grid: &BeatGridAnalysis, beats_per_bar: i64) -> i64 {
    if grid.downbeats.is_empty() || grid.beats.is_empty() {
        return 0;
    }
    let mut counts = vec![0usize; beats_per_bar as usize];
    for &db in &grid.downbeats {
        if !db.is_finite() {
            continue;
        }
        let idx = nearest_beat_index(&grid.beats, db);
        counts[(idx as i64).rem_euclid(beats_per_bar) as usize] += 1;
    }
    let mut best = 0usize;
    for (i, &c) in counts.iter().enumerate() {
        if c > counts[best] {
            best = i;
        }
    }
    best as i64
}

/// Index of the beat in sorted `beats` (seconds) nearest to `t`.
fn nearest_beat_index(beats: &[f32], t: f32) -> usize {
    match beats.binary_search_by(|b| b.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(i) => i,
        Err(i) => {
            if i == 0 {
                0
            } else if i >= beats.len() {
                beats.len() - 1
            } else if (beats[i] - t).abs() < (t - beats[i - 1]).abs() {
                i
            } else {
                i - 1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::{AnalysisRunMetadata, BpmAnalysis};

    fn track_with(beats: Vec<f32>, bpm: f64) -> TrackAnalysis {
        TrackAnalysis {
            bpm: Some(BpmAnalysis {
                bpm,
                confidence: 0.9,
            }),
            key: None,
            beat_grid: Some(BeatGridAnalysis {
                beats,
                bars: vec![],
                downbeats: vec![],
                grid_stability: 0.0,
                beats_per_bar: 4,
            }),
            loudness_lufs: None,
            metadata: AnalysisRunMetadata {
                backend: "test".into(),
                backend_version: "0".into(),
                analyzed_at: "0".into(),
                sample_rate: 44100,
                duration_analyzed_ms: 0,
            },
        }
    }

    #[test]
    fn snaps_half_time_into_canonical_range() {
        // 48 BPM detected for a 96 BPM track → fold ×2, snap to 96.
        let beats: Vec<f32> = (0..64).map(|i| i as f32 * 1.25).collect();
        let mut track = track_with(beats, 48.0);
        snap_grid(&mut track, &SnapConfig::default());
        let bpm = track.bpm.as_ref().unwrap().bpm;
        assert!((bpm - 96.0).abs() < 1e-9, "got {bpm}");
    }

    #[test]
    fn produces_a_constant_grid() {
        // Slightly noisy 120 BPM with a phase offset (symmetric jitter → median
        // interval is exactly 0.5 s).
        let period = 0.5_f32;
        let beats: Vec<f32> = (0..100)
            .map(|i| {
                let jitter = if i % 3 == 0 { 0.004 } else { 0.0 };
                0.13 + i as f32 * period + jitter
            })
            .collect();
        let mut track = track_with(beats, 120.0);
        snap_grid(&mut track, &SnapConfig::default());
        let grid = track.beat_grid.as_ref().unwrap();
        assert!((track.bpm.as_ref().unwrap().bpm - 120.0).abs() < 1e-9);
        let diffs: Vec<f32> = grid.beats.windows(2).map(|w| w[1] - w[0]).collect();
        let worst = diffs.iter().fold(0.0f32, |a, d| a.max((d - 0.5).abs()));
        assert!(worst < 1e-4, "grid not constant: {worst}");
        assert!(
            (grid.beats[0] - 0.13).abs() < 0.01,
            "anchor drifted: {}",
            grid.beats[0]
        );
    }

    #[test]
    fn reported_bpm_wins_over_noisy_beats() {
        // Beat intervals imply ~94 BPM, but the backend reports 81 (the truth);
        // snap follows the reported BPM and rebuilds a clean 81 BPM grid.
        let beats: Vec<f32> = (0..11).map(|i| i as f32 * 0.64).collect();
        let mut track = track_with(beats, 81.0);
        snap_grid(&mut track, &SnapConfig::default());
        assert!((track.bpm.as_ref().unwrap().bpm - 81.0).abs() < 1e-9);
        let grid = track.beat_grid.as_ref().unwrap();
        let period = 60.0 / 81.0;
        let diffs: Vec<f64> = grid
            .beats
            .windows(2)
            .map(|w| f64::from(w[1] - w[0]))
            .collect();
        let worst = diffs.iter().fold(0.0f64, |a, d| a.max((d - period).abs()));
        assert!(worst < 1e-4, "grid not constant: {worst}");
    }

    #[test]
    fn rounds_bpm_to_configured_decimals() {
        // Rust's float formatter rounds half to even: 124.25 -> 124.2, 124.05 -> 124.0.
        assert!((round_to_decimals(124.25, 1) - 124.2).abs() < 1e-9);
        assert!((round_to_decimals(124.05, 1) - 124.0).abs() < 1e-9);
        assert!((round_to_decimals(124.5, 0) - 124.0).abs() < 1e-9);
    }

    #[test]
    fn rounds_bpm_and_rebuilds_grid() {
        // 124.25 BPM detected → 124.2, and the grid period uses the rounded BPM.
        let period = 60.0_f64 / 124.25;
        let beats: Vec<f32> = (0..64).map(|i| (0.2 + i as f64 * period) as f32).collect();
        let mut track = track_with(beats, 124.25);
        snap_grid(&mut track, &SnapConfig::default());
        assert!((track.bpm.as_ref().unwrap().bpm - 124.2).abs() < 1e-9);
        let grid = track.beat_grid.as_ref().unwrap();
        let out_period = 60.0 / 124.2;
        let worst = grid
            .beats
            .windows(2)
            .map(|w| f64::from(w[1] - w[0]))
            .fold(0.0f64, |a, d| a.max((d - out_period).abs()));
        assert!(worst < 1e-4, "grid period not rebuilt: {worst}");
    }

    #[test]
    fn disabled_is_a_no_op() {
        let beats: Vec<f32> = (0..16).map(|i| i as f32 * 1.25).collect();
        let mut track = track_with(beats.clone(), 48.0);
        snap_grid(
            &mut track,
            &SnapConfig {
                enabled: false,
                ..Default::default()
            },
        );
        assert_eq!(track.beat_grid.as_ref().unwrap().beats, beats);
    }

    #[test]
    fn honors_beats_per_bar() {
        // 3/4 grid: bars fall every 3 beats, not every 4.
        let period = 0.5_f32;
        let beats: Vec<f32> = (0..30).map(|i| i as f32 * period).collect();
        let mut track = track_with(beats, 120.0);
        track.beat_grid.as_mut().unwrap().beats_per_bar = 3;
        snap_grid(&mut track, &SnapConfig::default());
        let grid = track.beat_grid.as_ref().unwrap();
        assert_eq!(grid.beats_per_bar, 3);
        assert!(grid.downbeats.len() >= 3);
        for w in grid.downbeats.windows(2) {
            assert!(
                (w[1] - w[0] - 3.0 * period).abs() < 1e-4,
                "downbeat spacing wrong: {:?}",
                w
            );
        }
    }

    #[test]
    fn empty_grid_is_a_no_op_even_with_min_beats_zero() {
        let mut track = track_with(vec![], 120.0);
        snap_grid(
            &mut track,
            &SnapConfig {
                min_beats: 0,
                ..Default::default()
            },
        );
        assert!(track.beat_grid.as_ref().unwrap().beats.is_empty());
    }

    #[test]
    fn narrow_fold_window_stays_in_range() {
        // max_bpm < 2 * min_bpm: no octave fits; the result is clamped into range.
        let beats: Vec<f32> = (0..64).map(|i| i as f32 * 0.6).collect();
        let mut track = track_with(beats, 60.0);
        snap_grid(
            &mut track,
            &SnapConfig {
                min_bpm: 70.0,
                max_bpm: 100.0,
                ..Default::default()
            },
        );
        let bpm = track.bpm.as_ref().unwrap().bpm;
        assert!((70.0..100.0).contains(&bpm), "bpm {bpm} outside [70, 100)");
    }

    #[test]
    fn rounding_does_not_leave_the_canonical_range() {
        // 139.96 BPM with decimals=0 rounds to 140 (== max, which is exclusive);
        // it must fold back into [70, 140).
        let period = 60.0_f64 / 139.96;
        let beats: Vec<f32> = (0..64).map(|i| (0.2 + i as f64 * period) as f32).collect();
        let mut track = track_with(beats, 139.96);
        snap_grid(
            &mut track,
            &SnapConfig {
                decimals: 0,
                ..Default::default()
            },
        );
        let bpm = track.bpm.as_ref().unwrap().bpm;
        assert!((70.0..140.0).contains(&bpm), "bpm {bpm} outside [70, 140)");
    }
}
