use serde::{Deserialize, Serialize};

/// Which analysis outputs to compute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnalysisTargets {
    pub bpm: bool,
    pub key: bool,
    pub beat_grid: bool,
}

impl AnalysisTargets {
    pub fn all() -> Self {
        Self {
            bpm: true,
            key: true,
            beat_grid: true,
        }
    }
}

impl Default for AnalysisTargets {
    fn default() -> Self {
        Self::all()
    }
}

/// Parameters for an offline analysis run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Request BPM/key/grid, or a subset.
    pub targets: AnalysisTargets,
    /// Minimum BPM confidence to prefer analysis over file tags when not forced.
    pub min_bpm_confidence: f32,
    /// Minimum key confidence to prefer analysis over file tags when not forced.
    pub min_key_confidence: f32,
    /// Max seconds of audio to decode (`None` = full file).
    pub max_duration_secs: Option<f64>,
    /// Preferred analysis sample rate (`None` = native or backend default).
    pub sample_rate: Option<u32>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            targets: AnalysisTargets::all(),
            min_bpm_confidence: 0.5,
            min_key_confidence: 0.5,
            max_duration_secs: None,
            sample_rate: None,
        }
    }
}
