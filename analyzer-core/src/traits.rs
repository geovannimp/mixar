use crate::config::AnalysisConfig;
use crate::error::{AnalyzerError, Result};
use crate::result::TrackAnalysis;

/// Offline analyzer backend (PCM in → analysis out).
pub trait AudioAnalyzer: Send + Sync {
    /// Backend identifier (e.g. `"stratum"`).
    fn name(&self) -> &'static str;

    /// Analyze mono `f32` PCM normalized to ±1.0.
    fn analyze_pcm(
        &self,
        samples: &[f32],
        sample_rate: u32,
        config: &AnalysisConfig,
    ) -> Result<TrackAnalysis>;
}

/// Boxed analyzer for dynamic dispatch in the facade crate.
pub type DynAudioAnalyzer = dyn AudioAnalyzer;

impl AudioAnalyzer for Box<DynAudioAnalyzer> {
    fn name(&self) -> &'static str {
        (**self).name()
    }

    fn analyze_pcm(
        &self,
        samples: &[f32],
        sample_rate: u32,
        config: &AnalysisConfig,
    ) -> Result<TrackAnalysis> {
        (**self).analyze_pcm(samples, sample_rate, config)
    }
}

/// Map backend errors through a named backend.
pub fn backend_err(backend: &'static str, message: impl ToString) -> AnalyzerError {
    AnalyzerError::Backend {
        backend,
        message: message.to_string(),
    }
}
