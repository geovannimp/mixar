use analyzer_core::AnalysisDurationMode;
use anyhow::{ensure, Result};
use audio_core::BusConfig;
use std::path::Path;

/// Engine callback size must be a multiple of this (matches `dasp_graph::Buffer::LEN`).
pub const BUFFER_SIZE_MULTIPLE: u32 = 64;

/// Reject buffer sizes the mixer graph cannot process in whole chunks.
pub fn validate_buffer_size(buffer_size: u32) -> Result<()> {
    ensure!(
        buffer_size > 0 && buffer_size.is_multiple_of(BUFFER_SIZE_MULTIPLE),
        "buffer_size must be a positive multiple of {BUFFER_SIZE_MULTIPLE} (got {buffer_size})"
    );
    Ok(())
}

/// Engine configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineConfig {
    /// Sample rate for the engine
    pub sample_rate: u32,
    /// Buffer size in frames (must be a multiple of [`BUFFER_SIZE_MULTIPLE`])
    pub buffer_size: u32,
    /// Low latency hint
    pub low_latency: bool,
    /// Backend to use ("auto", "cpal", "miniaudio", "null")
    pub backend: String,
    /// Bus configurations
    pub buses: Vec<BusConfig>,
    /// Device configurations
    pub devices: Option<Vec<DeviceConfig>>,
    /// Advanced settings
    pub advanced: Option<AdvancedConfig>,
    /// Audio processing settings
    pub audio: Option<AudioConfig>,
    /// How much of each track to analyze when running offline analysis.
    pub analysis_duration: AnalysisDurationMode,
}

/// Device configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceConfig {
    /// Device name
    pub name: String,
    /// Device ID
    pub id: String,
}

/// Advanced configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvancedConfig {
    /// Maximum number of decks
    pub max_decks: Option<usize>,
    /// Enable debug logging
    pub debug: Option<bool>,
}

/// Audio processing configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioConfig {
    /// Resampler quality
    pub resampler_quality: Option<String>,
    /// Whether sampler pads mix before or after the channel strip.
    #[serde(default)]
    pub sampler_strip_route: Option<SamplerStripRouteSetting>,
}

/// Sampler ↔ channel-strip routing (persisted in engine config).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplerStripRouteSetting {
    #[default]
    Before,
    After,
}

impl SamplerStripRouteSetting {
    pub fn to_dsp(self) -> engine_dsp::SamplerStripRoute {
        match self {
            Self::Before => engine_dsp::SamplerStripRoute::BeforeStrip,
            Self::After => engine_dsp::SamplerStripRoute::AfterStrip,
        }
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            buffer_size: 512,
            low_latency: false,
            backend: "auto".to_string(),
            buses: vec![],
            devices: None,
            advanced: None,
            audio: None,
            analysis_duration: AnalysisDurationMode::Precise,
        }
    }
}

impl EngineConfig {
    /// Effective resampler quality (`low`, `medium`, or `high`).
    pub fn resampler_quality(&self) -> String {
        let quality = self
            .audio
            .as_ref()
            .and_then(|audio| audio.resampler_quality.as_deref());
        resampler::normalize_resampler_quality(quality).to_string()
    }

    /// Sampler strip routing used when constructing the DSP graph.
    pub fn sampler_strip_route(&self) -> engine_dsp::SamplerStripRoute {
        self.audio
            .as_ref()
            .and_then(|audio| audio.sampler_strip_route)
            .unwrap_or_default()
            .to_dsp()
    }

    /// Validate fields that the engine requires to be well-formed.
    pub fn validate(&self) -> Result<()> {
        validate_buffer_size(self.buffer_size)
    }

    /// Load configuration from a TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: EngineConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_toml_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.validate()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_config_default() {
        let config = EngineConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.buffer_size, 512);
        assert!(!config.low_latency);
        assert_eq!(config.backend, "auto");
        assert_eq!(config.analysis_duration, AnalysisDurationMode::Precise);
        config.validate().unwrap();
    }

    #[test]
    fn validate_buffer_size_requires_multiple_of_chunk() {
        assert_eq!(BUFFER_SIZE_MULTIPLE, 64);
        assert!(validate_buffer_size(64).is_ok());
        assert!(validate_buffer_size(512).is_ok());
        assert!(validate_buffer_size(0).is_err());
        assert!(validate_buffer_size(100).is_err());
        assert!(validate_buffer_size(65).is_err());
    }

    #[test]
    fn from_toml_rejects_non_multiple_buffer_size() {
        let toml = r#"
sample_rate = 48000
buffer_size = 100
low_latency = false
backend = "null"
buses = []
analysis_duration = "precise"
"#;
        let err = toml::from_str::<EngineConfig>(toml)
            .unwrap()
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("multiple of"));
    }
}
