//! Pure-Rust port of Mixxx's qm-dsp analysis (beats + key).
//!
//! Mixxx runs the [qm-dsp](https://github.com/c4dm/qm-dsp) library for beat and
//! key detection (`AnalyzerQueenMaryBeats` / `AnalyzerQueenMaryKey`). This crate
//! reproduces those two analyses in Rust, using established crates for the
//! generic DSP pieces and porting only the qm-dsp-specific algorithms:
//!
//! - **FFT** — [`rustfft`]/[`realfft`] instead of the bundled kissfft.
//! - **Beats** — complex-domain onset detection (`DF_COMPLEXSD`) feeding
//!   `TempoTrackV2` (comb-filter bank + Viterbi tempo contour + Ellis DP beat
//!   tracking), exactly as `AnalyzerQueenMaryBeats`.
//! - **Key** — `GetKeyMode`: 8× decimation, constant-Q chromagram, HPCP mean,
//!   Krumhansl profile correlation, median filter, exactly as
//!   `AnalyzerQueenMaryKey`.
//! - Window framing mirrors Mixxx's `DownmixAndOverlapHelper`.
//!
//! This crate is Mixar's only analysis backend. It has no FFI and no C++
//! toolchain requirement.
//!
//! **Licensing:** a port of qm-dsp (GPL-2.0-or-later) is a derivative work and
//! stays under Mixar's GPLv3.

// This is a faithful numeric port of index-based DSP code; the C++ uses explicit
// indices (kept for parity/readability) and full-precision coefficient literals.
#![allow(clippy::needless_range_loop, clippy::excessive_precision)]

mod downbeat;
mod fft;
mod key;
mod math;
mod onset;
mod overlap;
mod tempo;
mod window;

use analyzer_core::{
    AnalysisConfig, AnalysisRunMetadata, AnalyzerError, AudioAnalyzer, BeatGridAnalysis,
    BpmAnalysis, KeyAnalysis, Result, TrackAnalysis,
};

/// Mixxx's onset step (`kStepSecs`): ~12 ms, 86 Hz resolution.
const STEP_SECS: f64 = 0.01161;
/// Mixxx's `kMaximumBinSizeHz` (max analysis window frequency).
const MAXIMUM_BIN_SIZE_HZ: u32 = 50;
/// Assumed meter. Stored on the grid; a future meter estimator will replace it.
const BEATS_PER_BAR: u8 = 4;

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Offline analyzer reproducing Mixxx's qm-dsp beats + key.
#[derive(Default)]
pub struct QmdspAnalyzer;

impl QmdspAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Always available (no external source needed, unlike the FFI oracle).
    pub fn from_env() -> Option<Self> {
        Some(Self::new())
    }
}

impl AudioAnalyzer for QmdspAnalyzer {
    fn name(&self) -> &'static str {
        "qmdsp"
    }

    fn analyze_pcm(
        &self,
        samples: &[f32],
        sample_rate: u32,
        config: &AnalysisConfig,
    ) -> Result<TrackAnalysis> {
        if samples.is_empty() {
            return Err(AnalyzerError::Analysis("empty audio buffer".into()));
        }

        let sr = sample_rate.max(1);
        let y: Vec<f64> = samples.iter().map(|&s| f64::from(s)).collect();

        let need_beats = config.targets.bpm || config.targets.beat_grid;
        let beats = if need_beats {
            detect_beats(&y, sr)
        } else {
            Vec::new()
        };
        let downbeats = if config.targets.beat_grid && beats.len() >= 2 {
            downbeat::detect_downbeats(&y, sr, &beats, BEATS_PER_BAR)
        } else {
            Vec::new()
        };
        let key_index = if config.targets.key {
            detect_key(&y, sr)
        } else {
            0
        };

        let bpm = analyzer_core::calculate_bpm_from_seconds(&beats, sr)
            .or_else(|| bpm_from_beats(&beats));
        let regularity = beat_regularity(&beats);

        let duration_analyzed_ms = (samples.len() as f64 / f64::from(sr) * 1000.0).round() as i32;

        let mut track = TrackAnalysis {
            bpm: None,
            key: None,
            beat_grid: None,
            loudness_lufs: None,
            metadata: AnalysisRunMetadata {
                backend: self.name().to_string(),
                backend_version: env!("CARGO_PKG_VERSION").to_string(),
                analyzed_at: time::now_rfc3339(),
                sample_rate: sr,
                duration_analyzed_ms,
            },
        };

        if config.targets.bpm {
            track.bpm = bpm.map(|bpm| BpmAnalysis {
                bpm,
                confidence: regularity as f32,
            });
        }

        if config.targets.beat_grid {
            track.beat_grid = Some(BeatGridAnalysis {
                beats,
                bars: downbeats.clone(),
                downbeats,
                grid_stability: regularity as f32,
                beats_per_bar: BEATS_PER_BAR,
            });
        }

        if config.targets.key {
            track.key = key_from_index(key_index);
        }

        Ok(track)
    }
}

/// Run the Mixxx beat pipeline and return beat times in seconds.
fn detect_beats(y: &[f64], sample_rate: u32) -> Vec<f32> {
    let step = (sample_rate as f64 * STEP_SECS) as i32;
    let window = math::next_power_of_two((sample_rate / MAXIMUM_BIN_SIZE_HZ) as i32);
    if step <= 0 || window <= 0 {
        return Vec::new();
    }
    let step_size = step as usize;
    let window_size = window as usize;
    if step_size > window_size {
        return Vec::new();
    }

    let mut df = onset::DetectionFunction::new(window_size);
    let mut results: Vec<f64> = Vec::new();
    overlap::feed(y, window_size, step_size, |frame| {
        results.push(df.process_time_domain(frame));
    });

    // Trim trailing non-positive values, then skip the first two (noise) frames,
    // exactly as Mixxx does.
    let mut nonzero = results.len();
    while nonzero > 0 && results[nonzero - 1] <= 0.0 {
        nonzero -= 1;
    }
    let df_slice: &[f64] = if nonzero >= 2 {
        &results[2..nonzero]
    } else {
        &[]
    };

    let beat_period = tempo::calculate_beat_period(df_slice);
    let beat_frames = tempo::calculate_beats(df_slice, &beat_period);

    beat_frames
        .iter()
        .map(|&b| {
            let samples = b * step_size as f64 + (step_size / 2) as f64;
            (samples / f64::from(sample_rate)) as f32
        })
        .collect()
}

/// Run the Mixxx key pipeline and return the dominant key index (0 = none).
fn detect_key(y: &[f64], sample_rate: u32) -> i32 {
    let mut key_mode = key::GetKeyMode::new(sample_rate);
    let block = key_mode.block_size();
    let hop = key_mode.hop_size();
    let mut counts = [0i32; 25];

    overlap::feed(y, block, hop, |frame| {
        let k = key_mode.process(frame);
        if (1..=24).contains(&k) {
            counts[k as usize] += 1;
        }
    });

    let mut best = 0i32;
    let mut best_count = 0i32;
    for (k, &count) in counts.iter().enumerate().skip(1) {
        if count > best_count {
            best_count = count;
            best = k as i32;
        }
    }
    best
}

/// Map a Mixxx `GetKeyMode` index (1..12 major, 13..24 minor) to notation.
fn key_from_index(index: i32) -> Option<KeyAnalysis> {
    if !(1..=24).contains(&index) {
        return None;
    }
    let zero = index - 1;
    let note = NOTE_NAMES[(zero % 12) as usize];
    let musical = if zero < 12 {
        note.to_string()
    } else {
        format!("{note}m")
    };
    Some(KeyAnalysis {
        musical,
        confidence: 0.0,
        clarity: 0.0,
    })
}

fn bpm_from_beats(beats: &[f32]) -> Option<f64> {
    if beats.len() < 2 {
        return None;
    }
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
    (median > 0.0).then(|| 60.0 / median)
}

fn beat_regularity(beats: &[f32]) -> f64 {
    if beats.len() < 3 {
        return 0.0;
    }
    let intervals: Vec<f64> = beats
        .windows(2)
        .map(|w| f64::from(w[1] - w[0]))
        .filter(|d| *d > 0.0)
        .collect();
    if intervals.len() < 2 {
        return 0.0;
    }
    let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
    if mean <= f64::MIN_POSITIVE {
        return 0.0;
    }
    let variance =
        intervals.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / intervals.len() as f64;
    (1.0 - variance.sqrt() / mean).clamp(0.0, 1.0)
}

mod time {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn now_rfc3339() -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{secs}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_errors() {
        let err = QmdspAnalyzer::new()
            .analyze_pcm(&[], 44100, &AnalysisConfig::default())
            .unwrap_err();
        assert!(matches!(err, AnalyzerError::Analysis(_)));
    }

    #[test]
    fn maps_mixxx_key_indices() {
        assert_eq!(key_from_index(1).unwrap().musical, "C");
        assert_eq!(key_from_index(6).unwrap().musical, "F");
        assert_eq!(key_from_index(10).unwrap().musical, "A");
        assert_eq!(key_from_index(13).unwrap().musical, "Cm");
        assert_eq!(key_from_index(24).unwrap().musical, "Bm");
        assert!(key_from_index(0).is_none());
        assert!(key_from_index(25).is_none());
    }

    #[test]
    fn click_track_yields_the_click_tempo() {
        // 120 BPM clicks (every 0.5 s) for 20 s.
        let sr = 44100u32;
        let mut y = vec![0.0f32; sr as usize * 20];
        let period = (sr as f64 * 0.5) as usize;
        let mut i = 0;
        while i < y.len() {
            for k in 0..64 {
                if i + k < y.len() {
                    // short click
                    y[i + k] = (1.0 - k as f32 / 64.0).sin();
                }
            }
            i += period;
        }
        let beats = detect_beats(&y.iter().map(|&s| f64::from(s)).collect::<Vec<_>>(), sr);
        let bpm = bpm_from_beats(&beats).expect("a tempo");
        assert!((bpm - 120.0).abs() < 5.0, "bpm = {bpm}");
    }
}
