use analyzer_core::AnalysisDurationMode;
use anyhow::{ensure, Result};
use audio_core::BusConfig;
use std::path::Path;

/// Engine callback size must be a multiple of this (matches `dasp_graph::Buffer::LEN`).
pub const BUFFER_SIZE_MULTIPLE: u32 = 64;

/// Default tempo fader half-span as pitch fraction (`0.06` = ±6%).
pub const DEFAULT_TEMPO_RANGE: f32 = 0.06;

/// Pioneer / Mixxx DDJ-400 tempo-range cycle steps (pitch fraction).
pub const DEFAULT_TEMPO_RANGE_STEPS: &[f32] = &[0.06, 0.10, 0.16, 0.25];

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
    /// Default deck tempo fader half-span (pitch fraction).
    #[serde(default)]
    pub default_tempo_range: Option<f32>,
    /// Cycle steps for tempo-range controls (pitch fractions).
    #[serde(default)]
    pub tempo_range_steps: Option<Vec<f32>>,
    /// Default key lock for new decks.
    #[serde(default)]
    pub default_key_lock: Option<bool>,
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

    /// Default deck tempo range (pitch fraction).
    pub fn default_tempo_range(&self) -> f32 {
        self.audio
            .as_ref()
            .and_then(|audio| audio.default_tempo_range)
            .filter(|range| range.is_finite() && *range > 0.0)
            .unwrap_or(DEFAULT_TEMPO_RANGE)
    }

    /// Tempo-range cycle steps (pitch fractions). Falls back to Pioneer defaults.
    pub fn tempo_range_steps(&self) -> Vec<f32> {
        let steps = self
            .audio
            .as_ref()
            .and_then(|audio| audio.tempo_range_steps.as_ref())
            .map(|steps| {
                steps
                    .iter()
                    .copied()
                    .filter(|step| step.is_finite() && *step > 0.0)
                    .collect::<Vec<_>>()
            })
            .filter(|steps| !steps.is_empty());
        steps.unwrap_or_else(|| DEFAULT_TEMPO_RANGE_STEPS.to_vec())
    }

    /// Default key lock for new decks.
    pub fn default_key_lock(&self) -> bool {
        self.audio
            .as_ref()
            .and_then(|audio| audio.default_key_lock)
            .unwrap_or(false)
    }

    /// Validate fields that the engine requires to be well-formed.
    pub fn validate(&self) -> Result<()> {
        validate_buffer_size(self.buffer_size)?;
        if let Some(range) = self
            .audio
            .as_ref()
            .and_then(|audio| audio.default_tempo_range)
        {
            ensure!(
                range.is_finite() && range > 0.0,
                "default_tempo_range must be finite and > 0 (got {range})"
            );
        }
        if let Some(steps) = self
            .audio
            .as_ref()
            .and_then(|audio| audio.tempo_range_steps.as_ref())
        {
            ensure!(
                !steps.is_empty(),
                "tempo_range_steps must not be empty when set"
            );
            for step in steps {
                ensure!(
                    step.is_finite() && *step > 0.0,
                    "tempo_range_steps entries must be finite and > 0 (got {step})"
                );
            }
        }
        Ok(())
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
        assert_eq!(config.default_tempo_range(), DEFAULT_TEMPO_RANGE);
        assert_eq!(
            config.tempo_range_steps(),
            DEFAULT_TEMPO_RANGE_STEPS.to_vec()
        );
        config.validate().unwrap();
    }

    #[test]
    fn tempo_range_config_overrides_defaults() {
        let config = EngineConfig {
            audio: Some(AudioConfig {
                resampler_quality: None,
                sampler_strip_route: None,
                default_tempo_range: Some(0.16),
                tempo_range_steps: Some(vec![0.08, 0.16]),
                default_key_lock: None,
            }),
            ..EngineConfig::default()
        };
        assert!((config.default_tempo_range() - 0.16).abs() < 1e-6);
        assert_eq!(config.tempo_range_steps(), vec![0.08, 0.16]);
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
