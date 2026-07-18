//! stratum-dsp backend for offline audio analysis.

mod mapper;

use analyzer_core::{
    backend_err, AnalysisConfig, AnalysisRunMetadata, AudioAnalyzer, BeatGridAnalysis, BpmAnalysis,
    KeyAnalysis, Result, TrackAnalysis,
};
use stratum_dsp::{analyze_audio, AnalysisConfig as StratumConfig};

pub use mapper::musical_key_from_stratum;

/// Offline analyzer using [stratum-dsp](https://docs.rs/stratum-dsp).
#[derive(Default)]
pub struct StratumAnalyzer {
    inner: StratumConfig,
}

impl StratumAnalyzer {
    /// Create with default stratum-dsp configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a custom stratum-dsp configuration.
    pub fn with_config(inner: StratumConfig) -> Self {
        Self { inner }
    }
}

impl AudioAnalyzer for StratumAnalyzer {
    fn name(&self) -> &'static str {
        "stratum"
    }

    fn analyze_pcm(
        &self,
        samples: &[f32],
        sample_rate: u32,
        config: &AnalysisConfig,
    ) -> Result<TrackAnalysis> {
        if samples.is_empty() {
            return Err(analyzer_core::AnalyzerError::Analysis(
                "empty audio buffer".into(),
            ));
        }

        let result = analyze_audio(samples, sample_rate, self.inner.clone())
            .map_err(|e| backend_err("stratum", e.to_string()))?;

        let duration_analyzed_secs = samples.len() as f64 / f64::from(sample_rate.max(1));
        let analyzed_at = time::now_rfc3339();

        let mut track = TrackAnalysis {
            bpm: None,
            key: None,
            beat_grid: None,
            loudness_lufs: None,
            metadata: AnalysisRunMetadata {
                backend: self.name().to_string(),
                backend_version: env!("CARGO_PKG_VERSION").to_string(),
                analyzed_at,
                sample_rate,
                duration_analyzed_secs,
            },
        };

        if config.targets.bpm {
            track.bpm = Some(BpmAnalysis {
                bpm: f64::from(result.bpm),
                confidence: result.bpm_confidence,
            });
        }

        if config.targets.key {
            track.key = Some(KeyAnalysis {
                musical: musical_key_from_stratum(&result.key),
                confidence: result.key_confidence,
                clarity: result.key_clarity,
            });
        }

        if config.targets.beat_grid {
            track.beat_grid = Some(BeatGridAnalysis {
                beats: result.beat_grid.beats,
                bars: result.beat_grid.bars,
                downbeats: result.beat_grid.downbeats,
                grid_stability: result.grid_stability,
            });
        }

        Ok(track)
    }
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
        let analyzer = StratumAnalyzer::new();
        let err = analyzer
            .analyze_pcm(&[], 44100, &AnalysisConfig::default())
            .unwrap_err();
        assert!(matches!(err, analyzer_core::AnalyzerError::Analysis(_)));
    }
}
