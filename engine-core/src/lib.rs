//! Core engine orchestration for rust-dj-engine
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

use anyhow::Result;
use audio_core::{
    AudioBackend, AudioCallback, AudioStream, BusConfig, BusId, DeviceId, Sample, StreamParams,
};
use engine_dsp::DspEngine;
use rtrb::{Consumer, Producer, RingBuffer};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

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
    dsp_engine: Arc<Mutex<DspEngine>>,
    /// Audio backend
    backend: Box<dyn AudioBackend>,
    /// Audio stream
    stream: Option<Box<dyn AudioStream>>,
    /// Output buses
    output_buses: HashMap<BusId, Vec<Sample>>,
    /// Producer thread handle
    producer_thread: Option<JoinHandle<()>>,
    /// Engine running state
    running: Arc<Mutex<bool>>,
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
            dsp_engine: Arc::new(Mutex::new(dsp_engine)),
            backend,
            stream: None,
            output_buses,
            producer_thread: None,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start the engine
    pub fn start(&mut self) -> Result<()> {
        log::info!("Starting engine with backend: {}", self.backend.name());

        // Create producer/consumer pair for ring buffer
        // Use larger buffer to prevent underruns (8x buffer size for safety)
        let ring_buffer_capacity = self.config.buffer_size as usize * 8;
        let (producer, consumer) = RingBuffer::new(ring_buffer_capacity);

        // Set running state
        *self.running.lock().unwrap() = true;

        // Start producer thread
        let dsp_engine = self.dsp_engine.clone();
        let running = self.running.clone();
        let sample_rate = self.config.sample_rate;
        let buffer_size = self.config.buffer_size as usize;

        let producer_thread = thread::spawn(move || {
            Self::producer_thread_loop(dsp_engine, producer, running, sample_rate, buffer_size);
        });

        // Get default device
        let device = self.backend.default_output_device()?;

        // Create stream parameters
        let params = StreamParams::new(
            self.config.sample_rate,
            2, // Stereo
            self.config.buffer_size,
            self.config.low_latency,
        );

        // Create audio callback with consumer
        let callback = Box::new(ConsumerCallback::new(consumer));

        // Open audio stream
        let mut stream = self
            .backend
            .open_output_stream(&device.id, &params, callback)?;
        stream.start()?;

        self.stream = Some(stream);
        self.producer_thread = Some(producer_thread);

        log::info!("Engine started successfully with producer/consumer model");
        Ok(())
    }

    /// Stop the engine
    pub fn stop(&mut self) -> Result<()> {
        log::info!("Stopping engine");

        // Stop producer thread
        *self.running.lock().unwrap() = false;
        if let Some(thread) = self.producer_thread.take() {
            thread
                .join()
                .map_err(|_| anyhow::anyhow!("Failed to join producer thread"))?;
        }

        // Stop audio stream
        if let Some(_stream) = self.stream.take() {
            // Note: This is a simplified approach. In a real implementation,
            // we would need to properly handle the stream lifecycle.
            log::info!("Audio stream stopped");
        }

        log::info!("Engine stopped");
        Ok(())
    }

    /// Producer thread loop
    fn producer_thread_loop(
        dsp_engine: Arc<Mutex<DspEngine>>,
        mut producer: Producer<Sample>,
        running: Arc<Mutex<bool>>,
        sample_rate: u32,
        buffer_size: usize,
    ) {
        log::info!("Producer thread started");

        let mut output_buses = HashMap::new();
        output_buses.insert(BusId::new("master".to_string()), vec![0.0; buffer_size * 2]);

        // Calculate timing for rate limiting
        let buffer_duration_ms = (buffer_size as f64 * 1000.0) / sample_rate as f64;
        let sleep_duration = Duration::from_micros((buffer_duration_ms * 1000.0) as u64);

        while *running.lock().unwrap() {
            // Process DSP engine
            {
                let mut dsp = dsp_engine.lock().unwrap();
                if let Err(e) = dsp.process(buffer_size as u32, &mut output_buses) {
                    log::error!("DSP processing error: {}", e);
                }
            }

            // Get master bus output and write to ring buffer
            if let Some(master_bus) = output_buses.get(&BusId::new("master".to_string())) {
                let mut written = 0;
                for &sample in master_bus {
                    match producer.push(sample) {
                        Ok(()) => written += 1,
                        Err(_) => {
                            // Buffer is full, wait a bit longer
                            thread::sleep(Duration::from_millis(2));
                            break;
                        }
                    }
                }

                // Rate limit the producer to match audio callback timing
                if written > 0 {
                    thread::sleep(sleep_duration);
                } else {
                    // No samples written, wait a bit
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }

        log::info!("Producer thread stopped");
    }

    /// Load a track into a deck
    pub fn load_track(&mut self, deck_id: usize, path: &str) -> Result<()> {
        log::info!("Loading track into deck {} from: {}", deck_id, path);

        // Check if file exists
        if !std::path::Path::new(path).exists() {
            return Err(anyhow::anyhow!("Audio file not found: {}", path));
        }

        // Create decoder and load audio
        let mut decoder = codec::AudioDecoder::from_file(path)?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();

        log::info!("Audio file info: {} Hz, {} channels", sample_rate, channels);

        // Load entire audio file into memory
        let audio_samples = decoder.load_entire_file()?;
        log::info!("Loaded {} samples from audio file", audio_samples.len());

        // Load samples into the deck
        let mut dsp = self.dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.load_audio_samples(audio_samples, sample_rate, path.to_string())?;
            log::info!("Track loaded into deck {}", deck_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Play a deck
    pub fn play(&mut self, deck_id: usize) -> Result<()> {
        log::info!("Playing deck {}", deck_id);

        let mut dsp = self.dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.play()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Pause a deck
    pub fn pause(&mut self, deck_id: usize) -> Result<()> {
        log::info!("Pausing deck {}", deck_id);

        let mut dsp = self.dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.pause()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// List available audio devices
    pub fn list_devices(&self) -> Result<Vec<audio_core::DeviceInfo>> {
        self.backend.list_output_devices()
    }

    /// Get the default audio device
    pub fn default_device(&self) -> Result<audio_core::DeviceInfo> {
        self.backend.default_output_device()
    }

    /// Get bus configuration
    pub fn get_bus_config(&self, bus_id: &BusId) -> Option<&BusConfig> {
        self.config.buses.iter().find(|bus| &bus.id == bus_id)
    }

    /// Update bus configuration
    pub fn update_bus_config(&mut self, bus_id: &BusId, new_config: BusConfig) -> Result<()> {
        if let Some(bus) = self.config.buses.iter_mut().find(|bus| &bus.id == bus_id) {
            *bus = new_config;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Bus not found: {}", bus_id.as_str()))
        }
    }

    /// Get device configuration
    pub fn get_device_config(&self, device_id: &DeviceId) -> Option<&DeviceConfig> {
        self.config
            .devices
            .as_ref()?
            .iter()
            .find(|device| device.id == device_id.as_str())
    }

    /// Update device configuration
    pub fn update_device_config(
        &mut self,
        device_id: &DeviceId,
        new_config: DeviceConfig,
    ) -> Result<()> {
        if let Some(devices) = self.config.devices.as_mut() {
            if let Some(device) = devices
                .iter_mut()
                .find(|device| device.id == device_id.as_str())
            {
                *device = new_config;
                Ok(())
            } else {
                Err(anyhow::anyhow!("Device not found: {}", device_id.as_str()))
            }
        } else {
            Err(anyhow::anyhow!("No devices configured"))
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

/// Consumer audio callback implementation for the engine
struct ConsumerCallback {
    consumer: Consumer<Sample>,
}

impl ConsumerCallback {
    fn new(consumer: Consumer<Sample>) -> Self {
        Self { consumer }
    }
}

impl AudioCallback for ConsumerCallback {
    fn render(&mut self, out: &mut [Sample], _frames: u32, _sample_rate: u32) {
        // Read samples from ring buffer
        let mut read = 0;
        for sample in out.iter_mut() {
            match self.consumer.pop() {
                Ok(value) => {
                    *sample = value;
                    read += 1;
                }
                Err(_) => {
                    // Buffer is empty - this can cause audio glitches
                    // Fill remaining samples with silence to prevent clicks/pops
                    break;
                }
            }
        }

        // If we didn't get enough samples, fill the rest with silence
        // This prevents audio glitches when the producer can't keep up
        if read < out.len() {
            out[read..].fill(0.0);
        }
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

    #[test]
    fn test_engine_device_operations() {
        let config = EngineConfig::default();
        let engine = Engine::new(config).unwrap();

        // Test device listing
        let devices = engine.list_devices();
        assert!(devices.is_ok());

        // Test default device
        let default_device = engine.default_device();
        assert!(default_device.is_ok());
    }

    #[test]
    fn test_engine_bus_operations() {
        let mut config = EngineConfig::default();
        // Add a master bus to the config
        config.buses.push(BusConfig::new(
            BusId::new("master".to_string()),
            "Master Bus".to_string(),
            DeviceId::new("default".to_string()),
            audio_core::ChannelMapping::new(1, 2),
        ));

        let mut engine = Engine::new(config).unwrap();

        let master_bus_id = BusId::new("master".to_string());

        // Test getting bus config
        let bus_config = engine.get_bus_config(&master_bus_id);
        assert!(bus_config.is_some());

        // Test updating bus config
        let new_config = BusConfig::new(
            master_bus_id.clone(),
            "Updated Master".to_string(),
            DeviceId::new("default".to_string()),
            audio_core::ChannelMapping::new(1, 2),
        );
        assert!(engine.update_bus_config(&master_bus_id, new_config).is_ok());
    }

    #[test]
    fn test_engine_producer_consumer_architecture() {
        let config = EngineConfig::default();
        let mut engine = Engine::new(config).unwrap();

        // Test that we can start the engine with producer/consumer model
        assert!(engine.start().is_ok());

        // Test that we can stop the engine
        assert!(engine.stop().is_ok());
    }
}
