//! Beat This! (ONNX) spike backend for Mixar offline analysis.
//!
//! Wraps the pure-Rust [`beat_this`] port of CPJKU's Beat This! transformer
//! model. Unlike stratum/rosa this is a *learned* beat tracker: it returns
//! beats **and downbeats** directly, so `bars`/`downbeats` no longer need a
//! heuristic. It has no key model, so `keys` is left `None`.
//!
//! The ONNX models are external files (the crate does not bundle them). Point
//! at them with `BEATTHIS_MEL_MODEL` / `BEATTHIS_BEAT_MODEL`, or `BEATTHIS_MODEL_DIR`
//! (default `~/.cache/mixar/beatthis-models`, preferring `beat_this.onnx` then
//! `beat_this_small.onnx`).
//!
//! Spike quality: nothing here is wired into `library`, the FRB wire or the UI.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use analyzer_core::{
    AnalysisConfig, AnalysisRunMetadata, AnalyzerError, AudioAnalyzer, BeatGridAnalysis,
    BpmAnalysis, Result, TrackAnalysis,
};
use beat_this::{BeatThis, RtenRuntime, Runtime};

type RtenModel = <RtenRuntime as Runtime>::Model;

/// Offline beat/downbeat analyzer using Beat This! via the pure-Rust rten runtime.
pub struct BeatThisAnalyzer {
    inner: Mutex<BeatThis<RtenModel>>,
}

impl BeatThisAnalyzer {
    /// Load both ONNX models (mel front end + beat model).
    pub fn new(mel_model: &Path, beat_model: &Path) -> std::result::Result<Self, AnalyzerError> {
        let tracker = BeatThis::new(&RtenRuntime, mel_model, beat_model).map_err(|err| {
            AnalyzerError::Backend {
                backend: "beatthis",
                message: format!("failed to load models: {err}"),
            }
        })?;
        Ok(Self {
            inner: Mutex::new(tracker),
        })
    }

    /// Build from `BEATTHIS_*` environment variables / a conventional model dir.
    ///
    /// Returns `None` when the models are missing, so the probe can skip this
    /// backend instead of failing the whole run.
    pub fn from_env() -> Option<Self> {
        let mel = env_path("BEATTHIS_MEL_MODEL").unwrap_or_else(default_mel_model);
        let beat = env_path("BEATTHIS_BEAT_MODEL").unwrap_or_else(default_beat_model);
        if !mel.is_file() || !beat.is_file() {
            eprintln!(
                "beat-this: models missing (mel={}, beat={}); skipping",
                mel.display(),
                beat.display()
            );
            return None;
        }
        match Self::new(&mel, &beat) {
            Ok(analyzer) => {
                eprintln!(
                    "beat-this: loaded mel={} beat={}",
                    mel.display(),
                    beat.display()
                );
                Some(analyzer)
            }
            Err(err) => {
                eprintln!("beat-this: init failed: {err}");
                None
            }
        }
    }
}

fn model_dir() -> PathBuf {
    env_path("BEATTHIS_MODEL_DIR").unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join(".cache/mixar/beatthis-models")
    })
}

fn default_mel_model() -> PathBuf {
    model_dir().join("mel_spectrogram.onnx")
}

fn default_beat_model() -> PathBuf {
    let dir = model_dir();
    let full = dir.join("beat_this.onnx");
    if full.is_file() {
        full
    } else {
        dir.join("beat_this_small.onnx")
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

impl AudioAnalyzer for BeatThisAnalyzer {
    fn name(&self) -> &'static str {
        "beatthis"
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

        let analysis = {
            let mut tracker = self
                .inner
                .lock()
                .map_err(|_| AnalyzerError::Analysis("beat-this lock poisoned".into()))?;
            tracker
                .analyze_audio(samples, sample_rate)
                .map_err(|err| AnalyzerError::Backend {
                    backend: "beatthis",
                    message: err.to_string(),
                })?
        };

        let beat_times: Vec<f64> = analysis.beats.iter().map(|&b| f64::from(b)).collect();
        let regularity = beat_regularity(&beat_times);

        let duration_analyzed_ms =
            (samples.len() as f64 / f64::from(sample_rate.max(1)) * 1000.0).round() as i32;

        let mut track = TrackAnalysis {
            bpm: None,
            key: None,
            beat_grid: None,
            loudness_lufs: None,
            metadata: AnalysisRunMetadata {
                backend: self.name().to_string(),
                backend_version: env!("CARGO_PKG_VERSION").to_string(),
                analyzed_at: time::now_rfc3339(),
                sample_rate,
                duration_analyzed_ms,
            },
        };

        if config.targets.bpm {
            let bpm = analyzer_core::calculate_bpm_from_seconds(&analysis.beats, sample_rate)
                .or_else(|| bpm_from_beats(&analysis.beats));
            track.bpm = bpm.map(|bpm| BpmAnalysis {
                bpm,
                confidence: regularity as f32,
            });
        }

        if config.targets.beat_grid {
            track.beat_grid = Some(BeatGridAnalysis {
                beats: analysis.beats,
                bars: analysis.downbeats.clone(),
                downbeats: analysis.downbeats,
                grid_stability: regularity as f32,
            });
        }

        // Beat This! tracks beats/downbeats only; there is no key model.
        Ok(track)
    }
}

/// Median inter-beat interval → BPM (matches `beat_this::calculate_bpm`).
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

/// Heuristic beat-interval regularity in 0..1 (1 = perfectly even).
fn beat_regularity(beat_times: &[f64]) -> f64 {
    if beat_times.len() < 3 {
        return 0.0;
    }
    let intervals: Vec<f64> = beat_times
        .windows(2)
        .map(|w| w[1] - w[0])
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
        // Constructing without models is avoided; test the pure helper instead.
        assert!(bpm_from_beats(&[]).is_none());
        assert!(bpm_from_beats(&[1.0]).is_none());
    }

    #[test]
    fn bpm_from_even_beats() {
        let beats: Vec<f32> = (0..9).map(|i| i as f32 * 0.5).collect();
        let bpm = bpm_from_beats(&beats).unwrap();
        assert!((bpm - 120.0).abs() < 1e-6);
    }
}
