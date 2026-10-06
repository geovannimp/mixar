//! Shared types and traits for offline audio analysis.

mod beat_utils;
mod config;
mod error;
mod loudness;
mod merge;
mod result;
mod snap;
mod traits;

pub use beat_utils::{calculate_bpm, calculate_bpm_from_seconds, ConstRegion};
pub use config::{AnalysisConfig, AnalysisDurationMode, AnalysisTargets, SnapConfig};
pub use error::{AnalyzerError, Result};
pub use loudness::{
    auto_gain_db, loudness_lufs_from_replaygain_track_gain_db, AUTO_GAIN_CLAMP_DB,
    REPLAYGAIN_REFERENCE_LUFS,
};
pub use merge::{merge_track_metadata, TagMetadata};
pub use result::{AnalysisRunMetadata, BeatGridAnalysis, BpmAnalysis, KeyAnalysis, TrackAnalysis};
pub use snap::snap_grid;
pub use traits::{backend_err, AudioAnalyzer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_analysis_serde_round_trip() {
        let analysis = TrackAnalysis {
            bpm: Some(BpmAnalysis {
                bpm: 128.0,
                confidence: 0.9,
            }),
            key: None,
            beat_grid: None,
            loudness_lufs: None,
            metadata: AnalysisRunMetadata {
                backend: "test".into(),
                backend_version: "0".into(),
                analyzed_at: "1".into(),
                sample_rate: 44100,
                duration_analyzed_ms: 1000,
            },
        };
        let json = serde_json::to_string(&analysis).unwrap();
        let back: TrackAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(analysis, back);
    }
}
