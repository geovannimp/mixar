//! rosa (librosa port) spike backend for Mixar offline audio analysis.
//!
//! This crate exists to answer one question: *does a librosa-style pipeline
//! produce BPM/key/beat results we trust more than stratum-dsp?* It implements
//! the same [`AudioAnalyzer`] boundary as `analyzer-stratum`, but the mapping
//! code is deliberately simple and unproven:
//!
//! - **BPM / beats** — `rosa::beat::beat_track` over a `rosa::onset::onset_strength`
//!   envelope (the real librosa algorithm).
//! - **Key** — `rosa::cqt::chroma_cqt` + Krumhansl–Kessler profiles in [`key`]
//!   (librosa has no key detector, so this is our own thin layer).
//! - **Bars / downbeats** — a 4/4 phase heuristic over beat onset energy; librosa
//!   has no downbeat tracking at all.
//! - **Confidence / stability** — beat-interval regularity, not a calibrated metric.
//!
//! Nothing here is wired into `library`, the FRB wire, or the UI. Drive it from
//! `examples/probe.rs`.

mod key;

use analyzer_core::{
    AnalysisConfig, AnalysisRunMetadata, AnalyzerError, AudioAnalyzer, BeatGridAnalysis,
    BpmAnalysis, KeyAnalysis, Result, TrackAnalysis,
};
use rosa::beat::{beat_track, BeatTrackParams};
use rosa::cqt::{chroma_cqt, ChromaCqtParams};
use rosa::onset::{onset_strength, OnsetStrengthParams};

pub use key::{estimate_key, KeyEstimate};

/// librosa's default analysis frame geometry.
const HOP_LENGTH: usize = 512;
const N_FFT: usize = 2048;

/// Frame geometry shared by the onset, beat and chroma stages.
#[derive(Debug, Clone, Copy)]
pub struct RosaConfig {
    pub hop_length: usize,
    pub n_fft: usize,
}

impl Default for RosaConfig {
    fn default() -> Self {
        Self {
            hop_length: HOP_LENGTH,
            n_fft: N_FFT,
        }
    }
}

/// Offline analyzer using [rosa](https://docs.rs/rosa) (a librosa port).
#[derive(Default)]
pub struct RosaAnalyzer {
    config: RosaConfig,
}

impl RosaAnalyzer {
    /// Create with librosa's default frame geometry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom frame geometry.
    pub fn with_config(config: RosaConfig) -> Self {
        Self { config }
    }
}

impl AudioAnalyzer for RosaAnalyzer {
    fn name(&self) -> &'static str {
        "rosa"
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
        let sr_f = f64::from(sr);
        let y: Vec<f64> = samples.iter().map(|&s| f64::from(s)).collect();

        // --- Onset envelope + beat tracking (the actual librosa path). ---
        let mut onset_params = OnsetStrengthParams::new(sr_f);
        onset_params.n_fft = self.config.n_fft;
        onset_params.hop_length = self.config.hop_length;
        let onset_env = onset_strength(&y, &onset_params);

        let mut beat_params = BeatTrackParams::new(sr_f);
        beat_params.hop_length = self.config.hop_length;
        let (bpm, beat_frames) = beat_track(&onset_env, &beat_params);

        let beat_times: Vec<f64> =
            rosa::convert::frames_to_time(&beat_frames, sr, self.config.hop_length, None);
        let regularity = beat_regularity(&beat_times);
        // Derive BPM from the beats with Mixxx's BeatUtils for a like-for-like
        // comparison with the qmdsp backend (fall back to librosa's tempo).
        let bpm =
            analyzer_core::calculate_bpm_from_seconds(&beat_times, sample_rate).unwrap_or(bpm);

        let duration_analyzed_ms = (samples.len() as f64 / sr_f * 1000.0).round() as i32;
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
            track.bpm = Some(BpmAnalysis {
                bpm,
                confidence: regularity as f32,
            });
        }

        if config.targets.beat_grid {
            let (bars, downbeats) = bar_grid(&beat_frames, &onset_env, sr, &self.config);
            track.beat_grid = Some(BeatGridAnalysis {
                beats: beat_times.iter().map(|&t| t as f32).collect(),
                bars,
                downbeats,
                grid_stability: regularity as f32,
            });
        }

        if config.targets.key {
            let estimate = estimate_key_from_audio(&y, sr_f, &self.config);
            track.key = Some(KeyAnalysis {
                musical: estimate.musical,
                confidence: estimate.confidence,
                clarity: estimate.clarity,
            });
        }

        Ok(track)
    }
}

/// Chroma-CQT over the whole buffer, averaged per pitch class, then matched.
///
/// (Tried `chroma_cens` and a naive 36-bin → 12-bin fold; both regressed on the
/// Hercules pack, so the plain 12-bin CQT mean stays. A faithful 36-bin HPCP +
/// shift correlation, as in Mixxx's QM key detector, is future work.)
fn estimate_key_from_audio(y: &[f64], sr: f64, config: &RosaConfig) -> KeyEstimate {
    let params = ChromaCqtParams {
        sr,
        hop_length: config.hop_length,
        ..Default::default()
    };
    let chroma = chroma_cqt(y, &params);

    let mut avg = [0.0f64; 12];
    let frames = chroma.cols();
    if frames > 0 {
        for (pitch_class, value) in avg.iter_mut().enumerate() {
            *value = chroma.row(pitch_class).iter().sum::<f64>() / frames as f64;
        }
    }
    estimate_key(&avg)
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

/// Pick the 4/4 downbeat phase with the most onset energy, assuming 4/4.
///
/// librosa/rosa have no downbeat tracker; this is a placeholder so the spike has
/// bar positions to compare against stratum's grid. Returns `(bars, downbeats)`,
/// which are identical here.
fn bar_grid(
    beat_frames: &[usize],
    onset_env: &[f64],
    sr: u32,
    config: &RosaConfig,
) -> (Vec<f32>, Vec<f32>) {
    if beat_frames.len() < 4 {
        return (Vec::new(), Vec::new());
    }

    let mut energy = [0.0f64; 4];
    for (i, &frame) in beat_frames.iter().enumerate() {
        if let Some(&value) = onset_env.get(frame) {
            energy[i % 4] += value.max(0.0);
        }
    }
    let phase = (0..4)
        .max_by(|&a, &b| energy[a].total_cmp(&energy[b]))
        .unwrap_or(0);

    let downbeat_frames: Vec<usize> = beat_frames
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 4 == phase)
        .map(|(_, &frame)| frame)
        .collect();

    let times: Vec<f32> =
        rosa::convert::frames_to_time(&downbeat_frames, sr, config.hop_length, None)
            .iter()
            .map(|&t| t as f32)
            .collect();

    (times.clone(), times)
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
        let analyzer = RosaAnalyzer::new();
        let err = analyzer
            .analyze_pcm(&[], 44100, &AnalysisConfig::default())
            .unwrap_err();
        assert!(matches!(err, AnalyzerError::Analysis(_)));
    }

    #[test]
    fn beat_regularity_is_one_for_even_beats() {
        let beats: Vec<f64> = (0..8).map(|i| i as f64 * 0.5).collect();
        assert!((beat_regularity(&beats) - 1.0).abs() < 1e-9);
    }
}
