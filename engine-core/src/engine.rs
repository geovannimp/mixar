use audio_core::LoadedAudio;
use crate::backend::create_backend;
use crate::callback::ConsumerCallback;
use crate::config::{DeviceConfig, EngineConfig};
use crate::producer::{
    create_ring_buffer, producer_thread_loop, MasterStreamSetup,
};
use anyhow::Result;
use audio_core::{
    AudioStream, BusConfig, BusId, DeviceId, DeviceInfo, Sample, StreamParams,
};
use engine_dsp::DspEngine;
use engine_dsp::DeckEqGains;
use rtrb::Consumer;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Main engine struct
pub struct Engine {
    config: EngineConfig,
    dsp_engine: Option<Arc<Mutex<DspEngine>>>,
    backend: Box<dyn audio_core::AudioBackend>,
    stream: Option<Box<dyn AudioStream>>,
    producer_thread: Option<JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
}

impl Engine {
    /// Create a new engine with the given configuration
    pub fn new(config: EngineConfig) -> Result<Self> {
        let backend = create_backend(&config.backend)?;

        Ok(Self {
            config,
            dsp_engine: None,
            backend,
            stream: None,
            producer_thread: None,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start the engine
    pub fn start(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Err(anyhow::anyhow!("Engine is already running"));
        }

        log::info!("Starting engine with backend: {}", self.backend.name());
        *self.running.lock().unwrap() = true;

        let (producer, consumer, ring_buffer_capacity) =
            create_ring_buffer(self.config.buffer_size);

        let master_stream = self.start_master_stream(consumer)?;

        let producer_thread = match self.start_dsp_producer(
            producer,
            ring_buffer_capacity,
            &master_stream,
        ) {
            Ok(thread) => thread,
            Err(e) => {
                self.abort_start(None)?;
                return Err(e);
            }
        };

        let stream = match master_stream.start_playback(self.config.buffer_size) {
            Ok(stream) => stream,
            Err(e) => {
                self.abort_start(Some(producer_thread))?;
                return Err(e);
            }
        };

        self.stream = Some(stream);
        self.producer_thread = Some(producer_thread);

        log::info!("Engine started successfully with producer/consumer model");
        Ok(())
    }

    /// Open the master output stream and verify device parameters match config.
    fn start_master_stream(
        &mut self,
        consumer: Consumer<Sample>,
    ) -> Result<MasterStreamSetup> {
        let devices = self.backend.list_output_devices()?;
        let device = devices
            .iter()
            .find(|d| d.is_default)
            .or_else(|| devices.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No output device available"))?;

        let params = StreamParams::new(
            self.config.sample_rate,
            2,
            self.config.buffer_size,
            self.config.low_latency,
        );

        let callback_count = Arc::new(AtomicU64::new(0));
        let callback = Box::new(ConsumerCallback::new(consumer, Arc::clone(&callback_count)));

        let stream = self
            .backend
            .open_output_stream(&device.id, &params, callback)?;

        let buffer_size = stream
            .actual_buffer_size()
            .unwrap_or(self.config.buffer_size) as usize;
        let sample_rate = stream
            .actual_sample_rate()
            .unwrap_or(self.config.sample_rate);

        log::info!(
            "Audio stream opened: {} Hz, {} frames/buffer (config: {} Hz, {} frames)",
            sample_rate,
            buffer_size,
            self.config.sample_rate,
            self.config.buffer_size
        );

        if buffer_size != self.config.buffer_size as usize {
            return Err(anyhow::anyhow!(
                "Device buffer size {} frames does not match configured {} frames",
                buffer_size,
                self.config.buffer_size
            ));
        }
        if sample_rate != self.config.sample_rate {
            return Err(anyhow::anyhow!(
                "Device sample rate {} Hz does not match configured {} Hz",
                sample_rate,
                self.config.sample_rate
            ));
        }

        let callback_frames_atomic = stream.callback_frames_atomic();

        Ok(MasterStreamSetup {
            stream,
            callback_count,
            callback_frames_atomic,
            sample_rate,
            buffer_size,
        })
    }

    /// Create the DSP engine and start the producer thread (before master stream playback).
    fn start_dsp_producer(
        &mut self,
        producer: rtrb::Producer<Sample>,
        ring_buffer_capacity: usize,
        master_stream: &MasterStreamSetup,
    ) -> Result<JoinHandle<()>> {
        let dsp_engine = Arc::new(Mutex::new(DspEngine::new(
            master_stream.sample_rate,
            master_stream.buffer_size as u32,
            2,
            &self.config.resampler_quality(),
        )));
        self.dsp_engine = Some(Arc::clone(&dsp_engine));

        let callback_frames_for_producer = master_stream.callback_frames_atomic.clone();
        let sample_rate = master_stream.sample_rate;
        let buffer_size = master_stream.buffer_size;
        let callback_count = Arc::clone(&master_stream.callback_count);
        let running = self.running.clone();
        let producer_thread = thread::spawn(move || {
            producer_thread_loop(
                dsp_engine,
                producer,
                running,
                sample_rate,
                buffer_size,
                ring_buffer_capacity,
                callback_frames_for_producer,
                callback_count,
            );
        });

        const PRODUCER_WARMUP_MS: u64 = 200;
        thread::sleep(Duration::from_millis(PRODUCER_WARMUP_MS));
        log::info!(
            "Producer warmup done ({} ms), starting stream",
            PRODUCER_WARMUP_MS
        );

        Ok(producer_thread)
    }

    /// Tear down a partially started engine after a startup failure.
    fn abort_start(&mut self, producer_thread: Option<JoinHandle<()>>) -> Result<()> {
        *self.running.lock().unwrap() = false;
        if let Some(thread) = producer_thread {
            thread
                .join()
                .map_err(|_| anyhow::anyhow!("Failed to join producer thread"))?;
        }
        self.dsp_engine = None;
        Ok(())
    }

    /// Stop the engine
    pub fn stop(&mut self) -> Result<()> {
        log::info!("Stopping engine");

        *self.running.lock().unwrap() = false;
        if let Some(thread) = self.producer_thread.take() {
            thread
                .join()
                .map_err(|_| anyhow::anyhow!("Failed to join producer thread"))?;
        }

        if let Some(_stream) = self.stream.take() {
            log::info!("Audio stream stopped");
        }

        self.dsp_engine = None;

        log::info!("Engine stopped");
        Ok(())
    }

    /// Load a shared decoded track into a deck.
    pub fn load_track(&mut self, deck_id: usize, audio: Arc<LoadedAudio>) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.load(audio)?;
            log::info!("Track loaded into deck {}", deck_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Play a deck
    pub fn play(&mut self, deck_id: usize) -> Result<()> {
        log::info!("Playing deck {}", deck_id);

        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
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

        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.pause()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set a deck's volume (0.0..=1.0).
    pub fn set_deck_volume(&mut self, deck_id: usize, volume: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_volume(volume)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set a deck's three-band EQ gains in decibels.
    pub fn set_deck_eq(&mut self, deck_id: usize, gains: DeckEqGains) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_eq_gains(gains)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set a deck's three-band EQ gains in decibels (clamped to ±24 dB).
    pub fn set_deck_eq_bands(
        &mut self,
        deck_id: usize,
        low_db: f32,
        mid_db: f32,
        high_db: f32,
    ) -> Result<()> {
        self.set_deck_eq(deck_id, DeckEqGains::clamped(low_db, mid_db, high_db))
    }

    /// Set playback speed for a deck (1.0 = normal tempo).
    pub fn set_deck_speed(&mut self, deck_id: usize, speed: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_speed(speed)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Seek a deck to a position in seconds.
    pub fn seek_deck(&mut self, deck_id: usize, position_secs: f64) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.seek_secs(position_secs)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Unload the track from a deck.
    pub fn unload_deck(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.unload()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set the temporary cue point in seconds.
    pub fn set_deck_cue_point(&mut self, deck_id: usize, position_secs: f64) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_cue_point_secs(position_secs)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Begin cue-hold audition.
    pub fn begin_deck_cue_hold(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.begin_cue_hold()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// End cue-hold audition.
    pub fn end_deck_cue_hold(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.end_cue_hold()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Activate a loop region on a deck.
    pub fn set_deck_loop_region(
        &mut self,
        deck_id: usize,
        in_secs: f64,
        out_secs: f64,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_loop_region_secs(in_secs, out_secs)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Clear the active loop on a deck.
    pub fn clear_deck_loop(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.clear_loop();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Cue point and loop region for status mirroring.
    pub fn deck_transport_state(
        &self,
        deck_id: usize,
    ) -> Option<(Option<f64>, Option<(f64, f64)>)> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        Some((deck.cue_point_secs(), deck.loop_region_secs()))
    }

    /// Whether a deck is currently playing.
    pub fn deck_is_playing(&self, deck_id: usize) -> Option<bool> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        Some(matches!(deck.state(), engine_dsp::DeckState::Playing))
    }

    /// Set crossfader position (0.0 = deck A, 1.0 = deck B).
    pub fn set_crossfader(&mut self, position: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_crossfader(position)
    }

    /// Playback position and duration for a deck (seconds), when the engine is running.
    pub fn deck_playback_secs(&self, deck_id: usize) -> Option<(f64, f64)> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        let duration = deck.duration_seconds()?;
        let position = deck.position_seconds().unwrap_or(0.0);
        Some((position, duration))
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

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::{AudioSource, BusId, ChannelMapping, DeviceId, LoadedAudio};
    use library_core::FileAudioSource;
    use std::sync::Arc;

    fn empty_audio() -> Arc<LoadedAudio> {
        Arc::new(LoadedAudio {
            samples: vec![],
            sample_rate: 48_000,
            channels: 2,
            source_id: "test.wav".to_string(),
        })
    }

    #[test]
    fn test_engine_creation() {
        let config = EngineConfig::default();
        let engine = Engine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_engine_deck_operations() {
        let config = EngineConfig {
            backend: "null".to_string(),
            ..Default::default()
        };
        let mut engine = Engine::new(config).unwrap();

        assert!(engine.play(0).is_err());
        assert!(engine.pause(0).is_err());
        assert!(engine.load_track(0, empty_audio()).is_err());

        engine.start().unwrap();

        assert!(engine.play(0).is_ok());
        assert!(engine.pause(0).is_ok());
        let missing = FileAudioSource::from_path("test.mp3").load();
        assert!(missing.is_err());
        assert!(
            missing
                .unwrap_err()
                .to_string()
                .contains("not found")
        );

        assert!(engine.play(2).is_err());

        engine.stop().unwrap();
    }

    #[test]
    fn test_engine_device_operations() {
        let config = EngineConfig::default();
        let engine = Engine::new(config).unwrap();

        let devices = engine.list_devices();
        assert!(devices.is_ok());

        let default_device = engine.default_device();
        assert!(default_device.is_ok());
    }

    #[test]
    fn test_engine_bus_operations() {
        let mut config = EngineConfig::default();
        config.buses.push(BusConfig::new(
            BusId::new("master".to_string()),
            "Master Bus".to_string(),
            DeviceId::new("default".to_string()),
            ChannelMapping::new(1, 2),
        ));

        let mut engine = Engine::new(config).unwrap();

        let master_bus_id = BusId::new("master".to_string());

        let bus_config = engine.get_bus_config(&master_bus_id);
        assert!(bus_config.is_some());

        let new_config = BusConfig::new(
            master_bus_id.clone(),
            "Updated Master".to_string(),
            DeviceId::new("default".to_string()),
            ChannelMapping::new(1, 2),
        );
        assert!(engine.update_bus_config(&master_bus_id, new_config).is_ok());
    }

    #[test]
    fn test_engine_producer_consumer_architecture() {
        let config = EngineConfig {
            backend: "null".to_string(),
            ..Default::default()
        };
        let mut engine = Engine::new(config).unwrap();

        assert!(engine.start().is_ok());
        assert!(engine.stop().is_ok());
    }
}
