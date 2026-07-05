use serde::{Deserialize, Serialize};

/// Complete analysis output for one track.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackAnalysis {
    pub bpm: Option<BpmAnalysis>,
    pub key: Option<KeyAnalysis>,
    pub beat_grid: Option<BeatGridAnalysis>,
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnalysisRunMetadata {
    pub backend: String,
    pub backend_version: String,
    pub analyzed_at: String,
    pub sample_rate: u32,
    pub duration_analyzed_secs: f64,
}
