//! Core engine orchestration for rust-dj-engine
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

use anyhow::Result;
use audio_core::{
    AudioBackend, AudioCallback, AudioStream, BusConfig, BusId, DeviceId, Sample, StreamParams,
};
use engine_dsp::DspEngine;
use std::collections::HashMap;
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
    /// Backend to use ("auto", "cpal", "miniaudio", "pipewire", "null")
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

/// Main engine struct
pub struct Engine {
    /// Engine configuration
    config: EngineConfig,
    /// DSP engine
    dsp_engine: DspEngine,
    /// Audio backend
    backend: Box<dyn AudioBackend>,
    /// Audio stream
    stream: Option<Box<dyn AudioStream>>,
    /// Output buses
    output_buses: HashMap<BusId, Vec<Sample>>,
}

impl Engine {
    /// Create a new engine with the given configuration
    pub fn new(config: EngineConfig) -> Result<Self> {
        // Create DSP engine
        let dsp_engine = DspEngine::new(config.sample_rate, 2); // 2 decks for MVP

        // Create backend based on configuration
        let backend = Self::create_backend(&config.backend)?;

        // Initialize output buses
        let mut output_buses = HashMap::new();
        for bus_config in &config.buses {
            let buffer_size = config.buffer_size as usize * 2; // Stereo
            output_buses.insert(bus_config.id.clone(), vec![0.0; buffer_size]);
        }

        Ok(Self {
            config,
            dsp_engine,
            backend,
            stream: None,
            output_buses,
        })
    }

    /// Start the engine
    pub fn start(&mut self) -> Result<()> {
        log::info!("Starting engine with backend: {}", self.backend.name());

        // Get default device
        let device = self.backend.default_output_device()?;

        // Create stream parameters
        let params = StreamParams::new(
            self.config.sample_rate,
            2, // Stereo
            self.config.buffer_size,
            self.config.low_latency,
        );

        // Create audio callback
        let callback = Box::new(SimpleCallback::new());

        // Open audio stream
        let mut stream = self
            .backend
            .open_output_stream(&device.id, &params, callback)?;
        stream.start()?;

        self.stream = Some(stream);

        log::info!("Engine started successfully");
        Ok(())
    }

    /// Stop the engine
    pub fn stop(&mut self) -> Result<()> {
        log::info!("Stopping engine");

        if let Some(_stream) = self.stream.take() {
            // Note: This is a simplified approach. In a real implementation,
            // we would need to properly handle the stream lifecycle.
            log::info!("Audio stream stopped");
        }

        log::info!("Engine stopped");
        Ok(())
    }

    /// Load a track into a deck
    pub fn load_track(&mut self, deck_id: usize, _path: &str) -> Result<()> {
        log::info!("Loading track into deck {}", deck_id);

        if let Some(_deck) = self.dsp_engine.deck_mut(deck_id) {
            // TODO: Implement actual track loading in Sprint 1
            log::info!("Track loaded into deck {}", deck_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Play a deck
    pub fn play(&mut self, deck_id: usize) -> Result<()> {
        log::info!("Playing deck {}", deck_id);

        if let Some(deck) = self.dsp_engine.deck_mut(deck_id) {
            deck.play()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Pause a deck
    pub fn pause(&mut self, deck_id: usize) -> Result<()> {
        log::info!("Pausing deck {}", deck_id);

        if let Some(deck) = self.dsp_engine.deck_mut(deck_id) {
            deck.pause()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set bus device mapping
    pub fn set_bus_device(
        &mut self,
        bus: BusId,
        device: DeviceId,
        channels: [u16; 2],
    ) -> Result<()> {
        log::info!(
            "Setting bus {} to device {} channels {:?}",
            bus.as_str(),
            device.as_str(),
            channels
        );

        // TODO: Implement bus device mapping in Sprint 2
        Ok(())
    }

    /// Create backend based on configuration
    fn create_backend(backend_name: &str) -> Result<Box<dyn AudioBackend>> {
        match backend_name {
            "null" => {
                let backend = backend_null::NullBackend::new();
                Ok(Box::new(backend))
            }
            "miniaudio" => {
                let backend = backend_miniaudio::MiniaudioBackend::new()?;
                Ok(Box::new(backend))
            }
            "cpal" => {
                let backend = backend_cpal::CpalBackend::new()?;
                Ok(Box::new(backend))
            }
            "pipewire" => {
                // TODO: Implement in Sprint 4
                Err(anyhow::anyhow!("PipeWire backend not yet implemented"))
            }
            "auto" => {
                // Try to detect the best available backend
                // First try CPAL (more reliable), then miniaudio, then null
                match backend_cpal::CpalBackend::new() {
                    Ok(backend) => {
                        log::info!("Using CPAL backend");
                        Ok(Box::new(backend))
                    }
                    Err(e) => {
                        log::warn!("Failed to initialize CPAL backend: {}, trying miniaudio", e);
                        match backend_miniaudio::MiniaudioBackend::new() {
                            Ok(backend) => {
                                log::info!("Using miniaudio backend");
                                Ok(Box::new(backend))
                            }
                            Err(e) => {
                                log::warn!("Failed to initialize miniaudio backend: {}, falling back to null backend", e);
                                let backend = backend_null::NullBackend::new();
                                Ok(Box::new(backend))
                            }
                        }
                    }
                }
            }
            _ => Err(anyhow::anyhow!("Unknown backend: {}", backend_name)),
        }
    }
}

/// Simple audio callback implementation for the engine
struct SimpleCallback {
    // Simple callback that generates silence for now
}

impl SimpleCallback {
    fn new() -> Self {
        Self {}
    }
}

impl AudioCallback for SimpleCallback {
    fn render(&mut self, out: &mut [Sample], _frames: u32, _sample_rate: u32) {
        // For now, just fill with silence
        // TODO: In Sprint 2, this will be connected to the DSP engine
        out.fill(0.0);
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

    #[test]
    fn test_engine_creation() {
        let config = EngineConfig::default();
        let engine = Engine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_engine_deck_operations() {
        let config = EngineConfig::default();
        let mut engine = Engine::new(config).unwrap();

        // Test deck operations
        assert!(engine.play(0).is_ok());
        assert!(engine.pause(0).is_ok());
        assert!(engine.load_track(0, "test.mp3").is_ok());

        // Test invalid deck
        assert!(engine.play(2).is_err());
    }
}
