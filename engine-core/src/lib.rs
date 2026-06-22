//! Core engine orchestration for rust-dj-engine
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

use anyhow::Result;
use audio_core::{
    AudioCallback, AudioStream, BusConfig, BusId, DeviceId, DeviceInfo, Sample, StreamParams,
};
use engine_dsp::DspEngine;
use rtrb::{Consumer, Producer, RingBuffer};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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

/// Main engine struct
pub struct Engine {
    /// Engine configuration
    config: EngineConfig,
    /// DSP engine
    dsp_engine: Arc<Mutex<DspEngine>>,
    /// Audio backend
    backend: Box<dyn audio_core::AudioBackend>,
    /// Audio stream
    stream: Option<Box<dyn AudioStream>>,
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
        let backend = create_backend(&config.backend)?;

        Ok(Self {
            config,
            dsp_engine: Arc::new(Mutex::new(dsp_engine)),
            backend,
            stream: None,
            producer_thread: None,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start the engine
    pub fn start(&mut self) -> Result<()> {
        log::info!("Starting engine with backend: {}", self.backend.name());

        // Ring buffer: spec §5.2 — preallocate N * frames_per_buffer (N ≥ 8) to tolerate producer jitter.
        const RING_BUFFER_MULTIPLIER: usize = 24;
        let stereo_samples_per_buffer = self.config.buffer_size as usize * 2;
        let ring_buffer_capacity = stereo_samples_per_buffer * RING_BUFFER_MULTIPLIER;
        let (mut producer, consumer) = RingBuffer::new(ring_buffer_capacity);

        // Pre-fill with silence so callbacks have data before the producer thread starts (no allocations in callback).
        // Leave 2 buffers free for producer to fill immediately.
        let prefill = ring_buffer_capacity.saturating_sub(2 * stereo_samples_per_buffer);
        for _ in 0..prefill {
            let _ = producer.push(0.0);
        }
        log::info!(
            "Ring buffer: capacity={}, pre-filled={} samples (spec: N×frames_per_buffer, zero alloc in callback)",
            ring_buffer_capacity,
            prefill
        );

        // Set running state
        *self.running.lock().unwrap() = true;

        // Get default device from list (first with is_default, else first device) and open stream.
        let devices = self.backend.list_output_devices()?;
        let device = devices
            .iter()
            .find(|d| d.is_default)
            .or_else(|| devices.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No output device available"))?;

        let params = StreamParams::new(
            self.config.sample_rate,
            2, // Stereo
            self.config.buffer_size,
            self.config.low_latency,
        );

        let callback_count = Arc::new(AtomicU64::new(0));
        let callback = Box::new(ConsumerCallback::new(consumer, Arc::clone(&callback_count)));

        let mut stream = self
            .backend
            .open_output_stream(&device.id, &params, callback)?;

        // Get buffer size and sample rate from stream (backend may report requested or actual).
        let actual_buffer_size = stream
            .actual_buffer_size()
            .unwrap_or(self.config.buffer_size) as usize;
        let stream_sample_rate = stream
            .actual_sample_rate()
            .unwrap_or(self.config.sample_rate);

        let sample_rate = stream_sample_rate;
        log::info!(
            "Audio stream opened: {} Hz, {} frames/buffer (config: {} Hz, {} frames)",
            stream_sample_rate,
            actual_buffer_size,
            self.config.sample_rate,
            self.config.buffer_size
        );
        if actual_buffer_size != self.config.buffer_size as usize {
            return Err(anyhow::anyhow!(
                "Device buffer size {} frames does not match configured {} frames",
                actual_buffer_size,
                self.config.buffer_size
            ));
        }
        if stream_sample_rate != self.config.sample_rate {
            return Err(anyhow::anyhow!(
                "Device sample rate {} Hz does not match configured {} Hz",
                stream_sample_rate,
                self.config.sample_rate
            ));
        }
        {
            let mut dsp = self.dsp_engine.lock().unwrap();
            dsp.set_sample_rate(sample_rate);
            dsp.set_output_chunk_frames(actual_buffer_size as u32);
        }

        // Start producer before the stream so the ring buffer is full when the first callback runs (spec §5.2: tolerate jitter).
        let callback_frames_atomic = stream.callback_frames_atomic();
        let callback_frames_for_producer = callback_frames_atomic.clone();
        let dsp_engine = self.dsp_engine.clone();
        let running = self.running.clone();
        let fallback_buffer_size = actual_buffer_size;
        let producer_thread = thread::spawn(move || {
            Self::producer_thread_loop(
                dsp_engine,
                producer,
                running,
                sample_rate,
                fallback_buffer_size,
                ring_buffer_capacity,
                callback_frames_for_producer,
                callback_count,
            );
        });

        // Let the producer fill the buffer before we start the stream (avoids startup underruns).
        const PRODUCER_WARMUP_MS: u64 = 200;
        std::thread::sleep(Duration::from_millis(PRODUCER_WARMUP_MS));
        log::info!(
            "Producer warmup done ({} ms), starting stream",
            PRODUCER_WARMUP_MS
        );

        stream.start()?;

        if let Some(frames_atomic) = callback_frames_atomic {
            if let Err(e) = Self::wait_for_callback_frames(frames_atomic, self.config.buffer_size) {
                *self.running.lock().unwrap() = false;
                producer_thread
                    .join()
                    .map_err(|_| anyhow::anyhow!("Failed to join producer thread"))?;
                return Err(e);
            }
        }

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

    /// Wait for the audio device to report its callback frame count, then verify it matches config.
    fn wait_for_callback_frames(
        frames_atomic: Arc<std::sync::atomic::AtomicU32>,
        expected_frames: u32,
    ) -> Result<()> {
        const TIMEOUT: Duration = Duration::from_secs(2);
        let deadline = Instant::now() + TIMEOUT;

        while Instant::now() < deadline {
            let frames = frames_atomic.load(Ordering::Relaxed);
            if frames > 0 {
                if frames != expected_frames {
                    return Err(anyhow::anyhow!(
                        "Device callback size is {} frames but {} frames were configured",
                        frames,
                        expected_frames
                    ));
                }
                log::info!("Device callback size verified: {} frames", frames);
                return Ok(());
            }
            thread::sleep(Duration::from_millis(1));
        }

        Err(anyhow::anyhow!(
            "Timed out waiting for audio device callback (expected {} frames)",
            expected_frames
        ))
    }

    /// Producer thread loop (spec §5.1: writes decoded/resampled audio into ring buffer).
    /// Production is paced by the audio device callback count, not wall clock.
    #[allow(clippy::too_many_arguments)]
    fn producer_thread_loop(
        dsp_engine: Arc<Mutex<DspEngine>>,
        mut producer: Producer<Sample>,
        running: Arc<Mutex<bool>>,
        sample_rate: u32,
        fallback_buffer_size: usize,
        ring_buffer_capacity: usize,
        callback_frames_atomic: Option<Arc<std::sync::atomic::AtomicU32>>,
        callback_count: Arc<AtomicU64>,
    ) {
        log::info!(
            "Producer thread started (fallback_buffer_size={}, ring_capacity={}, sample_rate={})",
            fallback_buffer_size,
            ring_buffer_capacity,
            sample_rate
        );

        let master_bus_id = BusId::new("master");
        let mut output_buses = HashMap::new();
        output_buses.insert(master_bus_id.clone(), vec![0.0; fallback_buffer_size * 2]);

        const MAX_AHEAD_CHUNKS: u64 = 2;
        let mut produced_chunks: u64 = 0;

        while *running.lock().unwrap() {
            let chunk_frames = callback_frames_atomic
                .as_ref()
                .and_then(|a| {
                    let v = a.load(Ordering::Relaxed);
                    if v > 0 {
                        Some(v as usize)
                    } else {
                        None
                    }
                })
                .unwrap_or(fallback_buffer_size);
            let samples_per_chunk = chunk_frames * 2;

            let buffer_duration = Duration::from_secs_f64(chunk_frames as f64 / sample_rate as f64);

            let device_callbacks = callback_count.load(Ordering::Relaxed);

            // Never run more than MAX_AHEAD_CHUNKS ahead of the device callback clock.
            if produced_chunks > device_callbacks.saturating_add(MAX_AHEAD_CHUNKS) {
                thread::sleep(buffer_duration / 4);
                continue;
            }

            let filled = ring_buffer_capacity.saturating_sub(producer.slots());
            let target_fill = samples_per_chunk * 2;
            if filled >= target_fill || producer.slots() < samples_per_chunk {
                thread::sleep(buffer_duration / 8);
                continue;
            }

            {
                let mut dsp = dsp_engine.lock().unwrap();
                if let Err(e) = dsp.process(chunk_frames as u32, &mut output_buses) {
                    log::error!("DSP processing error: {}", e);
                }
            }

            let mut pushed_chunk = false;
            if let Some(master_bus) = output_buses.get(&master_bus_id) {
                pushed_chunk = true;
                for &sample in master_bus.iter().take(samples_per_chunk) {
                    if producer.push(sample).is_err() {
                        pushed_chunk = false;
                        break;
                    }
                }
            }

            if pushed_chunk {
                produced_chunks += 1;
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

        // Load entire audio file into memory (native sample rate; deck resamples at playback).
        let audio_samples = decoder.load_entire_file()?;
        log::info!("Loaded {} samples from audio file", audio_samples.len());

        let mut dsp = self.dsp_engine.lock().unwrap();
        let engine_rate = dsp.sample_rate();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_sample_rate(engine_rate);
            deck.set_output_chunk_frames(self.config.buffer_size);
            log::info!(
                "Deck {} configured for {} Hz (engine/stream rate)",
                deck_id,
                engine_rate
            );

            deck.load_audio_samples(audio_samples, sample_rate, path.to_string())?;
            log::info!(
                "Track loaded into deck {} (file: {} Hz, engine/stream: {} Hz)",
                deck_id,
                sample_rate,
                engine_rate
            );
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

    /// List available audio devices for the engine's current backend
    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        self.backend.list_output_devices()
    }

    /// Get the default audio device (first device with `is_default`, or first in list).
    pub fn default_device(&self) -> Result<DeviceInfo> {
        let devices = self.backend.list_output_devices()?;
        devices
            .iter()
            .find(|d| d.is_default)
            .cloned()
            .or_else(|| devices.first().cloned())
            .ok_or_else(|| anyhow::anyhow!("No output device available"))
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
}

/// Create backend by name (used by Engine and by AudioBackend factory).
fn create_backend(backend_name: &str) -> Result<Box<dyn audio_core::AudioBackend>> {
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
            #[cfg(feature = "backend-cpal")]
            {
                let backend = backend_cpal::CpalBackend::new()?;
                Ok(Box::new(backend))
            }
            #[cfg(not(feature = "backend-cpal"))]
            Err(anyhow::anyhow!(
                "CPAL backend not compiled in. Build with default features or enable 'backend-cpal'."
            ))
        }
        "auto" => {
            // Try to detect the best available backend
            #[cfg(feature = "backend-cpal")]
            match backend_cpal::CpalBackend::new() {
                Ok(backend) => {
                    log::info!("Using CPAL backend");
                    return Ok(Box::new(backend));
                }
                Err(e) => log::warn!("Failed to initialize CPAL backend: {}, trying miniaudio", e),
            }
            match backend_miniaudio::MiniaudioBackend::new() {
                Ok(backend) => {
                    log::info!("Using miniaudio backend");
                    Ok(Box::new(backend))
                }
                Err(e) => {
                    log::warn!(
                        "Failed to initialize miniaudio backend: {}, falling back to null backend",
                        e
                    );
                    let backend = backend_null::NullBackend::new();
                    Ok(Box::new(backend))
                }
            }
        }
        _ => Err(anyhow::anyhow!("Unknown backend: {}", backend_name)),
    }
}

// ---------------------------------------------------------------------------
// Backend factory: list names and create backend instances without an engine
// ---------------------------------------------------------------------------

/// Factory for listing and creating audio backends. Use this to discover
/// backends and devices before building config and creating an engine.
pub struct AudioBackend;

impl AudioBackend {
    /// Returns the list of available backend names (e.g. `["null", "miniaudio", "cpal"]`).
    /// Use one of these with `AudioBackend::new()` and for `EngineConfig::backend` (or use `"auto"` for config).
    pub fn list_names() -> Vec<String> {
        let mut backends = vec!["null".to_string(), "miniaudio".to_string()];
        #[cfg(feature = "backend-cpal")]
        backends.push("cpal".to_string());
        backends
    }

    /// Creates a backend instance by name. Use `list_names()` for valid names.
    /// Returns a boxed backend on which you can call `list_output_devices()` (devices include `is_default`).
    /// Bring the `AudioBackendTrait` trait into scope to call those methods.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(name: &str) -> Result<Box<dyn audio_core::AudioBackend>> {
        create_backend(name)
    }
}

/// Re-export of the backend trait so callers can use backend methods without depending on audio-core.
pub use audio_core::AudioBackend as AudioBackendTrait;

/// Consumer audio callback implementation for the engine
struct ConsumerCallback {
    consumer: Consumer<Sample>,
    callback_count: Arc<AtomicU64>,
}

impl ConsumerCallback {
    fn new(consumer: Consumer<Sample>, callback_count: Arc<AtomicU64>) -> Self {
        Self {
            consumer,
            callback_count,
        }
    }
}

impl AudioCallback for ConsumerCallback {
    fn render(&mut self, out: &mut [Sample], _frames: u32, _sample_rate: u32) {
        self.callback_count.fetch_add(1, Ordering::Relaxed);

        for sample in out.iter_mut() {
            match self.consumer.pop() {
                Ok(value) => {
                    *sample = value;
                }
                Err(_) => {
                    *sample = 0.0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_list_names() {
        let names = AudioBackend::list_names();
        assert!(!names.is_empty());
        assert!(names.contains(&"null".to_string()));
        assert!(names.contains(&"miniaudio".to_string()));
    }

    #[test]
    fn test_backend_new_and_list_devices() {
        let backend = AudioBackend::new("null").unwrap();
        let devices = backend.list_output_devices();
        assert!(devices.is_ok());
        assert!(!devices.unwrap().is_empty());
    }

    #[test]
    fn test_backend_new_and_default_from_list() {
        let backend = AudioBackend::new("null").unwrap();
        let devices = backend.list_output_devices().unwrap();
        let default = devices.iter().find(|d| d.is_default).or(devices.first());
        assert!(default.is_some());
    }

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
