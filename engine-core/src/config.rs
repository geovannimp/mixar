use anyhow::Result;
use audio_core::BusConfig;
use std::path::Path;

/// Engine configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EngineConfig {
    /// Sample rate for the engine
    pub sample_rate: u32,
    /// Buffer size in frames
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
    /// Enable resampling
    pub enable_resampling: Option<bool>,
    /// Resampler quality
    pub resampler_quality: Option<String>,
    /// Enable BPM analysis
    pub enable_bpm_analysis: Option<bool>,
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
        }
    }
}

impl EngineConfig {
    /// Load configuration from a TOML file
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: EngineConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_toml_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
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
    }
}
