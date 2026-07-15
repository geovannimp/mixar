use audio_core::LoadedAudio;
use crate::backend::create_backend;
use crate::callback::ConsumerCallback;
use crate::config::{DeviceConfig, EngineConfig};
use crate::producer::{
    create_device_ring_buffer, producer_thread_loop, start_device_streams, DeviceStreamSetup,
};
use crate::routing::DeviceStreamPlan;
use crate::transport::TransportEvent;
use anyhow::Result;
use audio_core::{
    AudioStream, BusConfig, BusId, ChannelMapping, DeviceId, DeviceInfo, Sample, StreamParams,
};
use engine_dsp::DspEngine;
use engine_dsp::DeckEqGains;
use rtrb::Producer;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Main engine struct
pub struct Engine {
    config: EngineConfig,
    dsp_engine: Option<Arc<Mutex<DspEngine>>>,
    backend: Box<dyn audio_core::AudioBackend>,
    /// One opened stream per resolved device plan (spec §5.3: buses sharing a device share a stream).
    streams: Vec<Box<dyn AudioStream>>,
    producer_thread: Option<JoinHandle<()>>,
    running: Arc<Mutex<bool>>,
    transport_events: Arc<Mutex<Vec<TransportEvent>>>,
}

impl Engine {
    /// Create a new engine with the given configuration
    pub fn new(config: EngineConfig) -> Result<Self> {
        let backend = create_backend(&config.backend)?;

        Ok(Self {
            config,
            dsp_engine: None,
            backend,
            streams: Vec::new(),
            producer_thread: None,
            running: Arc::new(Mutex::new(false)),
            transport_events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Start the engine
    pub fn start(&mut self) -> Result<()> {
        if !self.streams.is_empty() {
            return Err(anyhow::anyhow!("Engine is already running"));
        }

        log::info!("Starting engine with backend: {}", self.backend.name());
        *self.running.lock().unwrap() = true;

        let device_streams = match self.open_device_streams() {
            Ok(setups) => setups,
            Err(e) => {
                *self.running.lock().unwrap() = false;
                return Err(e);
            }
        };

        // Index 0 is the pacing ("master clock") device: `resolve_device_stream_plans` sorts a
        // master-hosting plan first when one exists, so the producer paces off its callback.
        let pacing_callback_count = Arc::clone(&device_streams[0].callback_count);
        let pacing_callback_frames_atomic = device_streams[0].callback_frames_atomic.clone();
        let pacing_sample_rate = device_streams[0].sample_rate;
        let pacing_buffer_size = device_streams[0].buffer_size;
        let pacing_ring_buffer_capacity = device_streams[0].ring_buffer_capacity;

        let mut streams = Vec::with_capacity(device_streams.len());
        let mut device_producers = Vec::with_capacity(device_streams.len());
        for setup in device_streams {
            streams.push(setup.stream);
            device_producers.push((setup.plan, setup.producer));
        }

        let producer_thread = match self.start_dsp_producer(
            device_producers,
            pacing_sample_rate,
            pacing_buffer_size,
            pacing_ring_buffer_capacity,
            pacing_callback_frames_atomic.clone(),
            pacing_callback_count,
        ) {
            Ok(thread) => thread,
            Err(e) => {
                self.abort_start(None)?;
                return Err(e);
            }
        };

        if let Err(e) =
            start_device_streams(&mut streams, self.config.buffer_size, pacing_callback_frames_atomic)
        {
            self.abort_start(Some(producer_thread))?;
            return Err(e);
        }

        let stream_count = streams.len();
        self.streams = streams;
        self.producer_thread = Some(producer_thread);

        log::info!(
            "Engine started successfully with producer/consumer model ({} device stream(s))",
            stream_count
        );
        Ok(())
    }

    /// Resolve bus/device plans and open one output stream + ring per device, verifying each
    /// device's negotiated sample rate and buffer size match config.
    fn open_device_streams(&mut self) -> Result<Vec<DeviceStreamSetup>> {
        let devices = self.backend.list_output_devices()?;
        let plans = crate::routing::resolve_device_stream_plans(&self.config.buses, &devices)?;

        let mut device_streams = Vec::with_capacity(plans.len());
        for plan in plans {
            let (producer, consumer, ring_buffer_capacity) =
                create_device_ring_buffer(self.config.buffer_size, plan.channels);

            let callback_count = Arc::new(AtomicU64::new(0));
            let callback = Box::new(ConsumerCallback::new(consumer, Arc::clone(&callback_count)));
            let params = StreamParams::new(
                self.config.sample_rate,
                plan.channels,
                self.config.buffer_size,
                self.config.low_latency,
            );

            let stream = self
                .backend
                .open_output_stream(&plan.device, &params, callback)?;

            let buffer_size = stream
                .actual_buffer_size()
                .unwrap_or(self.config.buffer_size) as usize;
            let sample_rate = stream
                .actual_sample_rate()
                .unwrap_or(self.config.sample_rate);

            log::info!(
                "Audio stream opened on device '{}': {} Hz, {} frames/buffer, {} channel(s) (config: {} Hz, {} frames)",
                plan.device.as_str(),
                sample_rate,
                buffer_size,
                plan.channels,
                self.config.sample_rate,
                self.config.buffer_size
            );

            if buffer_size != self.config.buffer_size as usize {
                return Err(anyhow::anyhow!(
                    "Device '{}' buffer size {} frames does not match configured {} frames",
                    plan.device.as_str(),
                    buffer_size,
                    self.config.buffer_size
                ));
            }
            if sample_rate != self.config.sample_rate {
                return Err(anyhow::anyhow!(
                    "Device '{}' sample rate {} Hz does not match configured {} Hz",
                    plan.device.as_str(),
                    sample_rate,
                    self.config.sample_rate
                ));
            }

            let callback_frames_atomic = stream.callback_frames_atomic();

            device_streams.push(DeviceStreamSetup {
                plan,
                stream,
                producer,
                callback_count,
                callback_frames_atomic,
                sample_rate,
                buffer_size,
                ring_buffer_capacity,
            });
        }

        if device_streams.is_empty() {
            return Err(anyhow::anyhow!("No output device stream plans resolved"));
        }

        Ok(device_streams)
    }

    /// Create the DSP engine and start the producer thread (before any stream starts playback).
    #[allow(clippy::too_many_arguments)]
    fn start_dsp_producer(
        &mut self,
        device_producers: Vec<(DeviceStreamPlan, Producer<Sample>)>,
        sample_rate: u32,
        buffer_size: usize,
        ring_buffer_capacity: usize,
        callback_frames_atomic: Option<Arc<AtomicU32>>,
        callback_count: Arc<AtomicU64>,
    ) -> Result<JoinHandle<()>> {
        let dsp_engine = Arc::new(Mutex::new(DspEngine::new(
            sample_rate,
            buffer_size as u32,
            2,
            &self.config.resampler_quality(),
        )));
        self.dsp_engine = Some(Arc::clone(&dsp_engine));

        let running = self.running.clone();
        let transport_events = Arc::clone(&self.transport_events);
        let producer_thread = thread::spawn(move || {
            producer_thread_loop(
                dsp_engine,
                device_producers,
                running,
                sample_rate,
                buffer_size,
                ring_buffer_capacity,
                callback_frames_atomic,
                callback_count,
                transport_events,
            );
        });

        const PRODUCER_WARMUP_MS: u64 = 200;
        thread::sleep(Duration::from_millis(PRODUCER_WARMUP_MS));
        log::info!(
            "Producer warmup done ({} ms), starting stream(s)",
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

        let stream_count = self.streams.len();
        self.streams.clear();
        if stream_count > 0 {
            log::info!("Audio stream(s) stopped ({})", stream_count);
        }

        self.dsp_engine = None;
        self.transport_events.lock().unwrap().clear();

        log::info!("Engine stopped");
        Ok(())
    }

    /// Drain transport events posted by the producer thread (track ended, etc.).
    pub fn drain_transport_events(&mut self) -> Vec<TransportEvent> {
        std::mem::take(&mut *self.transport_events.lock().unwrap())
    }

    /// Snapshot playback positions for all decks that currently have loaded audio.
    pub fn deck_playback_snapshot(&self) -> Vec<(usize, f64, f64)> {
        let Some(dsp_engine) = self.dsp_engine.as_ref() else {
            return Vec::new();
        };
        let dsp = match dsp_engine.lock() {
            Ok(dsp) => dsp,
            Err(_) => return Vec::new(),
        };

        let mut snapshot = Vec::new();
        for deck_id in 0..dsp.num_decks() {
            let Some(deck) = dsp.deck(deck_id) else {
                continue;
            };
            let Some(duration) = deck.duration_seconds() else {
                continue;
            };
            let position = deck.position_seconds().unwrap_or(0.0);
            snapshot.push((deck_id, position, duration));
        }
        snapshot
    }

    /// Snapshot pre-fader stereo peaks for all decks.
    pub fn deck_level_snapshot(&self) -> Vec<(usize, f32, f32)> {
        let Some(dsp_engine) = self.dsp_engine.as_ref() else {
            return Vec::new();
        };
        let dsp = match dsp_engine.lock() {
            Ok(dsp) => dsp,
            Err(_) => return Vec::new(),
        };

        let mut snapshot = Vec::with_capacity(dsp.num_decks());
        for deck_id in 0..dsp.num_decks() {
            let Some(deck) = dsp.deck(deck_id) else {
                continue;
            };
            let peaks = deck.level_peaks();
            snapshot.push((deck_id, peaks.peak_l, peaks.peak_r));
        }
        snapshot
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

    /// Set DJ filter position for a deck (negative = LP, positive = HP).
    pub fn set_deck_filter_db(&mut self, deck_id: usize, filter_db: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_filter_db(filter_db)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set pre-fader gain trim for a deck in decibels.
    pub fn set_deck_gain_trim_db(&mut self, deck_id: usize, gain_db: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_gain_trim_db(gain_db)?;
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

    fn validate_bus_device_mapping(
        &self,
        bus_id: &BusId,
        device: &DeviceId,
        mapping: &ChannelMapping,
    ) -> Result<DeviceId> {
        let devices = self.backend.list_output_devices()?;
        let resolved = crate::routing::resolve_device_id(device, &devices)?;
        let info = devices
            .iter()
            .find(|d| d.id == resolved)
            .ok_or_else(|| anyhow::anyhow!("Output device not found: {}", resolved.as_str()))?;
        crate::routing::ensure_channels_in_range(mapping, info.max_channels, &resolved)?;
        crate::routing::ensure_no_channel_conflicts(
            &self.config.buses,
            bus_id,
            &resolved,
            mapping,
        )?;
        Ok(resolved)
    }

    /// Update bus configuration
    pub fn update_bus_config(&mut self, bus_id: &BusId, new_config: BusConfig) -> Result<()> {
        let mapping =
            crate::routing::validate_channel_pair([new_config.channels.left, new_config.channels.right])?;
        let resolved = self.validate_bus_device_mapping(bus_id, &new_config.device, &mapping)?;

        if let Some(bus) = self.config.buses.iter_mut().find(|bus| &bus.id == bus_id) {
            bus.name = new_config.name;
            bus.device = resolved;
            bus.channels = mapping;
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
        let mapping = crate::routing::validate_channel_pair(channels)?;
        let resolved = self.validate_bus_device_mapping(&bus, &device, &mapping)?;

        if let Some(existing) = self.config.buses.iter_mut().find(|b| b.id == bus) {
            existing.device = resolved;
            existing.channels = mapping;
        } else {
            let name = match bus.as_str() {
                "master" => "Master".to_string(),
                "cue" => "Preview".to_string(),
                other => other.to_string(),
            };
            self.config.buses.push(BusConfig::new(
                bus,
                name,
                resolved,
                mapping,
            ));
        }
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
    fn set_bus_device_updates_master_config() {
        let mut config = EngineConfig::default();
        config.backend = "null".into();
        let mut engine = Engine::new(config).unwrap();
        engine
            .set_bus_device(
                BusId::new("master"),
                DeviceId::new("null-device"),
                [3, 4],
            )
            .unwrap();
        let bus = engine.get_bus_config(&BusId::new("master")).unwrap();
        assert_eq!(bus.channels.left, 3);
        assert_eq!(bus.channels.right, 4);
        assert_eq!(bus.device.as_str(), "null-device");
    }

    #[test]
    fn set_bus_device_rejects_overlap_on_same_device() {
        let mut config = EngineConfig::default();
        config.backend = "null".into();
        config.buses = vec![
            BusConfig::new(
                BusId::new("master"),
                "Master".into(),
                DeviceId::new("null-device"),
                ChannelMapping::new(1, 2),
            ),
            BusConfig::new(
                BusId::new("cue"),
                "Preview".into(),
                DeviceId::new("null-device"),
                ChannelMapping::new(3, 4),
            ),
        ];
        let mut engine = Engine::new(config).unwrap();
        let err = engine
            .set_bus_device(BusId::new("cue"), DeviceId::new("null-device"), [2, 3])
            .unwrap_err();
        assert!(err.to_string().contains("overlaps"));
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
