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

/// How much of a track to decode for offline analysis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisDurationMode {
    /// First 30 seconds.
    Fast,
    /// Half of the track length (from metadata when available).
    #[default]
    Precise,
    /// Full track.
    Complete,
}

impl AnalysisDurationMode {
    /// Resolve to a decode cap in seconds (`None` = entire file).
    pub fn resolve_max_duration_secs(self, track_duration_secs: Option<f64>) -> Option<f64> {
        match self {
            Self::Fast => Some(30.0),
            Self::Precise => track_duration_secs.map(|duration| (duration * 0.5).max(1.0)),
            Self::Complete => None,
        }
    }
}

#[cfg(test)]
mod duration_mode_tests {
    use super::AnalysisDurationMode;

    #[test]
    fn fast_is_30_seconds() {
        assert_eq!(
            AnalysisDurationMode::Fast.resolve_max_duration_secs(None),
            Some(30.0)
        );
    }

    #[test]
    fn precise_is_half_track() {
        assert_eq!(
            AnalysisDurationMode::Precise.resolve_max_duration_secs(Some(240.0)),
            Some(120.0)
        );
        assert_eq!(
            AnalysisDurationMode::Precise.resolve_max_duration_secs(Some(1.0)),
            Some(1.0)
        );
    }

    #[test]
    fn precise_without_duration_is_none() {
        assert_eq!(
            AnalysisDurationMode::Precise.resolve_max_duration_secs(None),
            None
        );
    }

    #[test]
    fn complete_is_full_file() {
        assert_eq!(
            AnalysisDurationMode::Complete.resolve_max_duration_secs(Some(300.0)),
            None
        );
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
