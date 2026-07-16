//! Shared types and traits for offline audio analysis.

mod config;
mod error;
mod loudness;
mod merge;
mod result;
mod traits;

pub use config::{AnalysisConfig, AnalysisDurationMode, AnalysisTargets};
pub use error::{AnalyzerError, Result};
pub use loudness::{
    auto_gain_db, loudness_lufs_from_replaygain_track_gain_db, AUTO_GAIN_CLAMP_DB,
    REPLAYGAIN_REFERENCE_LUFS,
};
pub use merge::{merge_track_metadata, TagMetadata};
pub use result::{
    AnalysisRunMetadata, BeatGridAnalysis, BpmAnalysis, KeyAnalysis, TrackAnalysis,
};
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
            metadata: AnalysisRunMetadata {
                backend: "test".into(),
                backend_version: "0".into(),
                analyzed_at: "1".into(),
                sample_rate: 44100,
                duration_analyzed_secs: 1.0,
            },
        };
        let json = serde_json::to_string(&analysis).unwrap();
        let back: TrackAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(analysis, back);
    }
}
