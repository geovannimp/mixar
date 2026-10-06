use serde::{Deserialize, Serialize};

/// Complete analysis output for one track.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackAnalysis {
    pub bpm: Option<BpmAnalysis>,
    pub key: Option<KeyAnalysis>,
    pub beat_grid: Option<BeatGridAnalysis>,
    pub loudness_lufs: Option<f64>,
    pub metadata: AnalysisRunMetadata,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BpmAnalysis {
    pub bpm: f64,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyAnalysis {
    /// Musical key, e.g. `"F#m"` or `"Bb"`. Canonical form for library storage.
    pub musical: String,
    pub confidence: f32,
    pub clarity: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeatGridAnalysis {
    /// Beat positions in seconds from start.
    pub beats: Vec<f32>,
    /// Bar (measure) start positions in seconds.
    pub bars: Vec<f32>,
    /// Downbeat positions in seconds.
    pub downbeats: Vec<f32>,
    pub grid_stability: f32,
    /// Beats per bar (meter). Defaults to 4; a future meter estimator can set it.
    #[serde(default = "default_beats_per_bar")]
    pub beats_per_bar: u8,
}

/// Serde default for [`BeatGridAnalysis::beats_per_bar`].
fn default_beats_per_bar() -> u8 {
    4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beats_per_bar_defaults_to_four_when_absent() {
        // Older persisted JSON has no `beats_per_bar` field.
        let json = r#"{
            "beats": [0.0, 0.5],
            "bars": [0.0],
            "downbeats": [0.0],
            "grid_stability": 0.9
        }"#;
        let grid: BeatGridAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(grid.beats_per_bar, 4);
    }

    #[test]
    fn beats_per_bar_round_trips() {
        let grid = BeatGridAnalysis {
            beats: vec![0.0, 0.5],
            bars: vec![0.0],
            downbeats: vec![0.0],
            grid_stability: 0.9,
            beats_per_bar: 3,
        };
        let json = serde_json::to_string(&grid).unwrap();
        let back: BeatGridAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(back, grid);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalysisRunMetadata {
    pub backend: String,
    pub backend_version: String,
    pub analyzed_at: String,
    pub sample_rate: u32,
    pub duration_analyzed_ms: i32,
}
