//! Mixxx qm-dsp backend for Mixar analysis (spike).
//!
//! Calls Mixxx's vendored qm-dsp through a small C ABI shim (`src/qm_shim.cpp`):
//! complex-domain onset detection + `TempoTrackV2` for beats, and `GetKeyMode`
//! for key — the same defaults Mixxx uses.
//!
//! Build requirements: a C++ toolchain and a qm-dsp checkout pointed to by
//! `QMDSP_SOURCE_DIR` (default `~/.cache/mixar/mixxx-src/lib/qm-dsp`). When the
//! source is absent the crate builds empty and the backend reports unavailable.
//!
//! **Licensing:** qm-dsp is GPL-2.0-or-later (compatible with Mixar's GPLv3).
//! Like the other experimental backends this is only wired into `analyzer-probe`.

use analyzer_core::{
    AnalysisConfig, AnalysisRunMetadata, AnalyzerError, AudioAnalyzer, BeatGridAnalysis,
    BpmAnalysis, KeyAnalysis, Result, TrackAnalysis,
};

#[cfg(qm_dsp_available)]
extern "C" {
    fn qm_beats(mono: *const f32, n: i32, sr: i32, beats_out: *mut *mut f64) -> i32;
    fn qm_key(mono: *const f32, n: i32, sr: i32) -> i32;
    fn qm_free(p: *mut std::os::raw::c_void);
}

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Offline analyzer using Mixxx's qm-dsp (beats + key), via the C++ shim.
///
/// This is the differential-test oracle for the pure-Rust [`analyzer-qmdsp`]
/// port; it is not part of the product.
#[derive(Default)]
pub struct QmdspFfiAnalyzer;

impl QmdspFfiAnalyzer {
    /// Build the backend if the qm-dsp shim was compiled in, else `None`.
    pub fn from_env() -> Option<Self> {
        #[cfg(qm_dsp_available)]
        {
            eprintln!("qmdsp-ffi: loaded (Mixxx qm-dsp)");
            Some(Self)
        }
        #[cfg(not(qm_dsp_available))]
        {
            eprintln!("qmdsp-ffi: unavailable (build qm-dsp and set QMDSP_SOURCE_DIR)");
            None
        }
    }
}

impl AudioAnalyzer for QmdspFfiAnalyzer {
    fn name(&self) -> &'static str {
        "qmdsp-ffi"
    }

    fn analyze_pcm(
        &self,
        samples: &[f32],
        sample_rate: u32,
        config: &AnalysisConfig,
    ) -> Result<TrackAnalysis> {
        #[cfg(not(qm_dsp_available))]
        {
            let _ = (samples, sample_rate, config);
            return Err(AnalyzerError::Unsupported(
                "qmdsp: not built (QMDSP_SOURCE_DIR)",
            ));
        }
        #[cfg(qm_dsp_available)]
        {
            if samples.is_empty() {
                return Err(AnalyzerError::Analysis("empty audio buffer".into()));
            }

            let sr = sample_rate.max(1);
            let n = samples.len() as i32;

            let mut beats_ptr: *mut f64 = std::ptr::null_mut();
            let count = unsafe { qm_beats(samples.as_ptr(), n, sr as i32, &mut beats_ptr) };
            let beat_samples: Vec<f64> = if count > 0 && !beats_ptr.is_null() {
                let values =
                    unsafe { std::slice::from_raw_parts(beats_ptr, count as usize) }.to_vec();
                unsafe { qm_free(beats_ptr as *mut std::os::raw::c_void) };
                values
            } else {
                Vec::new()
            };

            let key_index = unsafe { qm_key(samples.as_ptr(), n, sr as i32) };

            let beats: Vec<f32> = beat_samples
                .iter()
                .map(|&s| (s / f64::from(sr)) as f32)
                .collect();
            let bpm = analyzer_core::calculate_bpm(&beat_samples, f64::from(sr))
                .or_else(|| bpm_from_beats(&beats));
            let regularity = beat_regularity(&beats);

            let duration_analyzed_ms =
                (samples.len() as f64 / f64::from(sr) * 1000.0).round() as i32;

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
                    bars: Vec::new(),
                    downbeats: Vec::new(),
                    grid_stability: regularity as f32,
                });
            }

            if config.targets.key {
                track.key = key_from_index(key_index);
            }

            Ok(track)
        }
    }
}

/// Map a Mixxx `GetKeyMode` index (1..12 major, 13..24 minor) to musical notation.
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
    fn maps_mixxx_key_indices() {
        // 1 = C major, 6 = F major, 10 = A major, 13 = C minor, 24 = B minor.
        assert_eq!(key_from_index(1).unwrap().musical, "C");
        assert_eq!(key_from_index(6).unwrap().musical, "F");
        assert_eq!(key_from_index(10).unwrap().musical, "A");
        assert_eq!(key_from_index(13).unwrap().musical, "Cm");
        assert_eq!(key_from_index(24).unwrap().musical, "Bm");
        assert!(key_from_index(0).is_none());
        assert!(key_from_index(25).is_none());
    }
}
