use crate::backend::create_backend;
use crate::callback::ConsumerCallback;
use crate::config::{DeviceConfig, EngineConfig};
use crate::producer::{
    create_device_ring_buffer, producer_thread_loop, start_device_streams, DeviceStreamSetup,
};
use crate::routing::DeviceStreamPlan;
use crate::sync::{beat_align_target, snap_ms, target_sync_speed, DeckControlState};
use crate::transport::TransportEvent;
use anyhow::Result;
use audio_core::LoadedAudio;
use audio_core::{
    ms_to_secs, secs_to_ms, AudioStream, BusConfig, BusId, ChannelMapping, DeviceId, DeviceInfo,
    Sample, StreamParams,
};
use engine_api::{
    DeckEq, DeckSnapshot, EngineStatus, LoopRegion, PadMode, SamplerStatus, SyncMode,
};
use engine_dsp::DeckEqGains;
use engine_dsp::DeckState;
use engine_dsp::DspEngine;
use library::{LibraryBus, LibraryManager, PreparedTrackPlayback};
use library_core::{AudioSource, LoadableAudio, TrackId, TrackMetadata};
use rtrb::Producer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const NUM_DECKS: usize = 2;

/// Cue point (ms) and optional loop region `(start, end)` for status mirroring.
type DeckTransportState = (Option<i32>, Option<(i32, i32)>);

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
    library: Option<Arc<Mutex<LibraryManager>>>,
    /// Optional library cmd bus for deck performance persistence (hot cues / loops).
    library_cmd: Option<LibraryBus>,
    /// Decoded PCM cache keyed by track id.
    decode_cache: HashMap<TrackId, Arc<LoadedAudio>>,
    master_deck: usize,
    deck_control: Vec<DeckControlState>,
    soft_takeover: crate::soft_takeover::SoftTakeoverState,
}

/// Absolute-control readback as wire `0..1` (for soft-takeover compares).
#[derive(Clone, Copy, Debug)]
pub struct DeckStripNorms {
    pub volume: f32,
    pub filter: f32,
    pub gain_trim: f32,
    pub eq_low: f32,
    pub eq_mid: f32,
    pub eq_high: f32,
    pub speed: f32,
    pub headphone_cue: bool,
}

impl Engine {
    /// Create a new engine with the given configuration
    pub fn new(config: EngineConfig) -> Result<Self> {
        Self::new_inner(config, None, None)
    }

    /// Create a new engine wired to the concrete shared library manager.
    pub fn new_with_library(
        config: EngineConfig,
        library: Arc<Mutex<LibraryManager>>,
    ) -> Result<Self> {
        Self::new_inner(config, Some(library), None)
    }

    /// Create an engine with library manager + library cmd bus (performance persistence).
    pub fn new_with_library_bus(
        config: EngineConfig,
        library: Arc<Mutex<LibraryManager>>,
        library_cmd: LibraryBus,
    ) -> Result<Self> {
        Self::new_inner(config, Some(library), Some(library_cmd))
    }

    fn new_inner(
        config: EngineConfig,
        library: Option<Arc<Mutex<LibraryManager>>>,
        library_cmd: Option<LibraryBus>,
    ) -> Result<Self> {
        config.validate()?;
        let backend = create_backend(&config.backend)?;

        Ok(Self {
            config,
            dsp_engine: None,
            backend,
            streams: Vec::new(),
            producer_thread: None,
            running: Arc::new(Mutex::new(false)),
            transport_events: Arc::new(Mutex::new(Vec::new())),
            library,
            library_cmd,
            decode_cache: HashMap::new(),
            master_deck: 0,
            deck_control: (0..NUM_DECKS)
                .map(|_| DeckControlState {
                    quantize: true,
                    ..Default::default()
                })
                .collect(),
            soft_takeover: crate::soft_takeover::SoftTakeoverState::default(),
        })
    }

    /// Load a library-indexed track into a deck through the shared library manager.
    pub fn load_track_from_library(&mut self, deck_id: usize, track_id: &TrackId) -> Result<()> {
        let library = self
            .library
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine has no attached LibraryManager"))?;
        let prepared = LibraryManager::prepare_track_for_playback(library.as_ref(), track_id)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        self.load_prepared_track(deck_id, prepared)
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

        if let Err(e) = start_device_streams(
            &mut streams,
            self.config.buffer_size,
            pacing_callback_frames_atomic,
        ) {
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
            self.config.sampler_strip_route(),
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
        self.decode_cache.clear();

        log::info!("Engine stopped");
        Ok(())
    }

    /// Drain transport events posted by the producer thread (track ended, etc.).
    pub fn drain_transport_events(&mut self) -> Vec<TransportEvent> {
        std::mem::take(&mut *self.transport_events.lock().unwrap())
    }

    /// Snapshot playback positions for all decks that currently have loaded audio.
    pub fn deck_playback_snapshot(&self) -> Vec<(usize, i32, i32)> {
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
            let Some(duration_secs) = deck.duration_seconds() else {
                continue;
            };
            let position = deck.position_ms().unwrap_or(0);
            snapshot.push((deck_id, position, secs_to_ms(duration_secs)));
        }
        snapshot
    }

    /// Snapshot pre-fader stereo peaks for all decks (read from their mixer channels).
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
            let Some(channel) = dsp.mixer().channel(deck_id) else {
                continue;
            };
            let peaks = channel.level_peaks();
            snapshot.push((deck_id, peaks.peak_l, peaks.peak_r));
        }
        snapshot
    }

    /// Load a track from an [`AudioSource`] into a deck.
    ///
    /// Decodes via [`LoadableAudio`] using an engine-owned PCM cache keyed by track id.
    /// Channel auto gain is derived from source metadata (ReplayGain preferred, else
    /// `loudness_lufs`) together with [`Self::set_normalizer_target`].
    pub fn load_track(&mut self, deck_id: usize, source: AudioSource) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();

        if dsp.deck(deck_id).is_none() || dsp.mixer().channel(deck_id).is_none() {
            return Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id));
        }

        let track_id = source.id().clone();
        let loudness_lufs = loudness_from_metadata(source.metadata());
        let bpm = source.metadata().bpm;
        let audio = if let Some(cached) = self.decode_cache.get(&track_id) {
            Arc::clone(cached)
        } else {
            let audio = Arc::new(source.load()?);
            self.decode_cache
                .insert(track_id.clone(), Arc::clone(&audio));
            audio
        };

        {
            let deck = dsp.deck_mut(deck_id).expect("validated above");
            deck.load(audio)?;
            deck.set_track_bpm(bpm);
        }
        dsp.mixer_mut()
            .channel_mut(deck_id)
            .expect("validated above")
            .set_loudness_lufs(loudness_lufs);
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.reset_for_load(bpm);
            control.track_id = Some(track_id);
        }
        drop(dsp);
        self.resync_followers_after_load(deck_id)?;
        log::info!("Track loaded into deck {}", deck_id);
        Ok(())
    }

    /// Load a track prepared by [`library::LibraryManager`] into a deck.
    ///
    /// This consumes library-owned decode/cache output directly instead of filling the engine's
    /// legacy decode cache.
    pub fn load_prepared_track(
        &mut self,
        deck_id: usize,
        prepared: PreparedTrackPlayback,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();

        if dsp.deck(deck_id).is_none() || dsp.mixer().channel(deck_id).is_none() {
            return Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id));
        }

        let bpm = prepared.source.metadata().bpm;
        let track_id = prepared.track_id.clone();
        let loudness_lufs =
            loudness_from_metadata(prepared.source.metadata()).or(prepared.loudness_lufs);
        let audio = prepared.audio;

        {
            let deck = dsp.deck_mut(deck_id).expect("validated above");
            deck.load(audio)?;
            deck.set_track_bpm(bpm);
        }
        dsp.mixer_mut()
            .channel_mut(deck_id)
            .expect("validated above")
            .set_loudness_lufs(loudness_lufs);
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.reset_for_load(bpm);
            control.track_id = Some(track_id);
        }
        drop(dsp);
        self.resync_followers_after_load(deck_id)?;
        log::info!("Library-prepared track loaded into deck {}", deck_id);
        Ok(())
    }

    /// When the master deck's BPM changes on load, refresh synced slaves' ratios.
    fn resync_followers_after_load(&mut self, loaded_deck_id: usize) -> Result<()> {
        if loaded_deck_id != self.master_deck {
            return Ok(());
        }
        for slave_id in 0..self.deck_control.len() {
            if slave_id == loaded_deck_id {
                continue;
            }
            if self.deck_control[slave_id].sync_mode == SyncMode::Off {
                continue;
            }
            // BPM may be missing on a follower; skip rather than fail the load.
            if self.apply_tempo_sync(slave_id, loaded_deck_id).is_err() {
                continue;
            }
        }
        Ok(())
    }

    /// Set loudness-normalization target for all mixer channels (`None` = off).
    pub fn set_normalizer_target(&mut self, target_lufs: Option<f32>) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_normalizer_target(target_lufs);
        Ok(())
    }

    /// Cached auto-gain (dB) for a deck's mixer channel, when the engine is running.
    pub fn deck_auto_gain_db(&self, deck_id: usize) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().channel(deck_id)?.auto_gain_db())
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

    /// Set a deck's volume (0.0..=1.0), routed through its mixer channel.
    pub fn set_deck_volume(&mut self, deck_id: usize, volume: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_volume(volume)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set whether a deck is routed to the headphone cue bus, via its mixer channel.
    pub fn set_deck_headphone_cue(&mut self, deck_id: usize, enabled: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let channel = dsp
            .mixer_mut()
            .channel_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        channel.set_headphone_cue(enabled);
        Ok(())
    }

    /// Set a deck's three-band EQ gains in decibels, via its mixer channel.
    pub fn set_deck_eq(&mut self, deck_id: usize, gains: DeckEqGains) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_eq_gains(gains)?;
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

    /// Set tempo fader position for a deck (`0..1`, center = track BPM).
    ///
    /// When the deck is master, synced slaves follow tempo (not beat phase).
    /// Returns every deck whose speed changed (master first).
    pub fn set_deck_speed(&mut self, deck_id: usize, speed: f32) -> Result<Vec<usize>> {
        if !(0.0..=1.0).contains(&speed) {
            return Err(anyhow::anyhow!("Speed must be between 0 and 1."));
        }
        self.set_deck_speed_raw(deck_id, speed)?;
        let mut updated = vec![deck_id];
        if deck_id == self.master_deck {
            for slave_id in 0..self.deck_control.len() {
                if slave_id == deck_id {
                    continue;
                }
                if self.deck_control[slave_id].sync_mode == SyncMode::Off {
                    continue;
                }
                self.apply_tempo_sync(slave_id, deck_id)?;
                updated.push(slave_id);
            }
        }
        Ok(updated)
    }

    fn set_deck_speed_raw(&mut self, deck_id: usize, speed: f32) -> Result<()> {
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

    fn set_deck_playback_ratio_raw(&mut self, deck_id: usize, ratio: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_playback_ratio(ratio)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    pub fn set_deck_jog_touch(&mut self, deck_id: usize, touching: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.set_jog_touch(touching);
        Ok(())
    }

    pub fn deck_jog_turn(&mut self, deck_id: usize, delta: i32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.jog_turn(delta);
        Ok(())
    }

    pub fn set_deck_jog_mode(
        &mut self,
        deck_id: usize,
        top: engine_api::JogMode,
        outer: engine_api::JogMode,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let deck = dsp
            .deck_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        deck.set_jog_mode(map_jog_mode(top), map_jog_mode(outer));
        Ok(())
    }

    fn apply_tempo_sync(&mut self, slave_id: usize, master_id: usize) -> Result<()> {
        if slave_id == master_id {
            return Err(anyhow::anyhow!("Cannot sync a deck to itself."));
        }
        let slave_bpm = self
            .deck_control
            .get(slave_id)
            .and_then(|d| d.bpm)
            .ok_or_else(|| anyhow::anyhow!("Slave deck BPM is required for sync."))?;
        let master_bpm = self
            .deck_control
            .get(master_id)
            .and_then(|d| d.bpm)
            .ok_or_else(|| anyhow::anyhow!("Master deck BPM is required for sync."))?;
        let master_ratio = {
            let dsp = self
                .dsp_engine
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?
                .lock()
                .unwrap();
            dsp.deck(master_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid master deck ID: {master_id}"))?
                .playback_ratio()
        };
        let target = target_sync_speed(master_bpm, master_ratio, slave_bpm);
        self.set_deck_playback_ratio_raw(slave_id, target)
    }

    fn align_beat_phase(&mut self, slave_id: usize, master_id: usize) -> Result<()> {
        let master_bpm = self
            .deck_control
            .get(master_id)
            .and_then(|d| d.bpm)
            .ok_or_else(|| anyhow::anyhow!("Master deck BPM is required for beat sync."))?;
        let (slave_bpm, quantize) = {
            let control = self
                .deck_control
                .get(slave_id)
                .ok_or_else(|| anyhow::anyhow!("Invalid slave deck ID: {slave_id}"))?;
            let bpm = control
                .bpm
                .ok_or_else(|| anyhow::anyhow!("Slave deck BPM is required for beat sync."))?;
            (bpm, control.quantize)
        };
        let (master_pos, _) = self.deck_playback_ms(master_id).unwrap_or((0, 0));
        let (slave_pos, duration) = self.deck_playback_ms(slave_id).unwrap_or((0, 0));
        let target = beat_align_target(
            master_pos, slave_pos, duration, master_bpm, slave_bpm, quantize,
        );
        self.seek_deck(slave_id, target)
    }

    /// Toggle sync for a slave deck (`beat_sync` chooses beat vs tempo when enabling).
    pub fn toggle_deck_sync(&mut self, deck_id: usize, beat_sync: bool) -> Result<Vec<usize>> {
        if deck_id >= self.deck_control.len() {
            return Err(anyhow::anyhow!("Invalid deck ID: {deck_id}"));
        }
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!("Load a track before enabling sync."));
        }
        let master_id = self.master_deck;
        if deck_id == master_id {
            return Err(anyhow::anyhow!(
                "Master deck cannot sync to itself. Choose the other deck."
            ));
        }

        let next_mode = if self.deck_control[deck_id].sync_mode == SyncMode::Off {
            if beat_sync {
                SyncMode::Beat
            } else {
                SyncMode::Tempo
            }
        } else {
            SyncMode::Off
        };
        self.deck_control[deck_id].sync_mode = next_mode;

        let updated = vec![deck_id];
        if next_mode != SyncMode::Off {
            self.apply_tempo_sync(deck_id, master_id)?;
            if next_mode == SyncMode::Beat {
                self.align_beat_phase(deck_id, master_id)?;
            }
        }
        Ok(updated)
    }

    /// Designate the master deck and re-apply sync to all active slaves.
    pub fn set_master_deck(&mut self, deck_id: usize) -> Result<Vec<usize>> {
        if deck_id >= self.deck_control.len() {
            return Err(anyhow::anyhow!("Invalid deck ID: {deck_id}"));
        }
        self.master_deck = deck_id;
        let mut updated = Vec::with_capacity(self.deck_control.len());
        for id in 0..self.deck_control.len() {
            updated.push(id);
        }
        for slave_id in 0..self.deck_control.len() {
            if slave_id == deck_id {
                continue;
            }
            let mode = self.deck_control[slave_id].sync_mode;
            if mode == SyncMode::Off {
                continue;
            }
            self.apply_tempo_sync(slave_id, deck_id)?;
            if mode == SyncMode::Beat {
                self.align_beat_phase(slave_id, deck_id)?;
            }
        }
        Ok(updated)
    }

    pub fn master_deck(&self) -> usize {
        self.master_deck
    }

    pub fn deck_sync_mode(&self, deck_id: usize) -> Option<SyncMode> {
        self.deck_control.get(deck_id).map(|d| d.sync_mode)
    }

    /// Set DJ filter position for a deck (negative = LP, positive = HP), via its mixer channel.
    pub fn set_deck_filter_db(&mut self, deck_id: usize, filter_db: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_filter_db(filter_db)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Set pre-fader gain trim for a deck in decibels, via its mixer channel.
    pub fn set_deck_gain_trim_db(&mut self, deck_id: usize, gain_db: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(channel) = dsp.mixer_mut().channel_mut(deck_id) {
            channel.set_gain_trim_db(gain_db)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Seek a deck to a position in milliseconds.
    pub fn seek_deck(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.seek_ms(position_ms)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id))
        }
    }

    /// Unload the track from a deck. Clears channel loudness (auto gain → 0) while
    /// preserving manual mixer controls (volume, EQ, filter, trim, headphone cue).
    pub fn unload_deck(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();

        if dsp.deck(deck_id).is_none() || dsp.mixer().channel(deck_id).is_none() {
            return Err(anyhow::anyhow!("Invalid deck ID: {}", deck_id));
        }

        dsp.deck_mut(deck_id).expect("validated above").unload()?;
        dsp.mixer_mut()
            .channel_mut(deck_id)
            .expect("validated above")
            .set_loudness_lufs(None);
        if let Some(control) = self.deck_control.get_mut(deck_id) {
            control.reset_for_load(None);
        }
        Ok(())
    }

    /// Mirror UI quantize into engine control state (beat-sync snap).
    pub fn set_deck_quantize(&mut self, deck_id: usize, enabled: bool) -> Result<()> {
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?;
        control.quantize = enabled;
        Ok(())
    }

    pub fn deck_quantize(&self, deck_id: usize) -> Option<bool> {
        self.deck_control.get(deck_id).map(|c| c.quantize)
    }

    fn decode_source(&mut self, source: &AudioSource) -> Result<Arc<LoadedAudio>> {
        let track_id = source.id().clone();
        if let Some(cached) = self.decode_cache.get(&track_id) {
            return Ok(Arc::clone(cached));
        }
        let audio = Arc::new(source.load()?);
        self.decode_cache.insert(track_id, Arc::clone(&audio));
        Ok(audio)
    }

    /// Assign a decoded sample to a sampler pad slot on a deck.
    pub fn assign_sampler_slot(
        &mut self,
        deck_id: usize,
        slot: usize,
        source: AudioSource,
        label: String,
        loudness_lufs: Option<f64>,
    ) -> Result<()> {
        let audio = self.decode_source(&source)?;
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        let sample_rate = dsp.sample_rate();
        let quality = dsp.mixer().resampler_quality().to_string();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .assign_slot(slot, audio, label, sample_rate, &quality, loudness_lufs)
    }

    /// Clear a sampler pad slot on a deck.
    pub fn clear_sampler_slot(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .clear_slot(slot)
    }

    /// Clear all sampler slots on a deck (e.g. before loading a bank).
    pub fn clear_all_sampler_slots(&mut self, deck_id: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .clear_all_slots();
        Ok(())
    }

    /// Set effective sampler play mode for a deck.
    pub fn set_sampler_play_mode(
        &mut self,
        deck_id: usize,
        mode: engine_dsp::SamplerPlayMode,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .set_play_mode(mode);
        Ok(())
    }

    /// Trigger a sample from a pad slot on a deck.
    pub fn trigger_sampler(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .trigger(slot)
    }

    /// End hold/loop for a sampler pad slot on a deck.
    pub fn end_sampler(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .end(slot)
    }

    /// Whether a sampler slot has assigned audio on a deck.
    pub fn sampler_slot_assigned(&self, deck_id: usize, slot: usize) -> bool {
        let Some(dsp_engine) = self.dsp_engine.as_ref() else {
            return false;
        };
        let Ok(dsp) = dsp_engine.lock() else {
            return false;
        };
        dsp.sampler(deck_id).is_some_and(|s| s.slot_assigned(slot))
    }

    /// Recompute auto-gain for a loaded sampler slot after normalizer settings change.
    pub fn set_sampler_slot_auto_gain(
        &mut self,
        deck_id: usize,
        slot: usize,
        auto_gain_db: f32,
    ) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.sampler_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?
            .set_slot_auto_gain_db(slot, auto_gain_db)
    }

    /// Set the temporary cue point in milliseconds.
    pub fn set_deck_cue_point(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_cue_point_ms(position_ms)?;
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
    pub fn set_deck_loop_region(&mut self, deck_id: usize, in_ms: i32, out_ms: i32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        if let Some(deck) = dsp.deck_mut(deck_id) {
            deck.set_loop_region_ms(in_ms, out_ms)?;
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

    /// Trigger a sampler pad; requires deck pad mode `sampler`.
    pub fn trigger_deck_sampler(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        let mode = self
            .deck_control
            .get(deck_id)
            .map(|c| c.pad_mode)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?;
        if mode != PadMode::Sampler {
            return Err(anyhow::anyhow!("Deck is not in Sampler pad mode."));
        }
        self.trigger_sampler(deck_id, slot)
    }

    /// End hold/loop for a sampler pad slot.
    pub fn end_deck_sampler(&mut self, deck_id: usize, slot: usize) -> Result<()> {
        self.end_sampler(deck_id, slot)
    }

    /// Trigger hot cue: snap position, seek, play.
    pub fn trigger_deck_hot_cue(&mut self, deck_id: usize, position_ms: i32) -> Result<()> {
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!("Load a track before triggering a hot cue."));
        }
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let target = snap_ms(position_ms, bpm, quantize);
        self.seek_deck(deck_id, target)?;
        self.play(deck_id)
    }

    /// Snap playhead and persist a hot cue via the library cmd bus.
    pub fn save_deck_hot_cue(&mut self, deck_id: usize, slot: u8) -> Result<()> {
        if slot > 7 {
            return Err(anyhow::anyhow!("Hot cue slot must be 0..=7."));
        }
        let track_id = self
            .deck_control
            .get(deck_id)
            .and_then(|c| c.track_id.clone())
            .ok_or_else(|| anyhow::anyhow!("Only library tracks can persist hot cues."))?;
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let position_ms = snap_ms(position_ms, bpm, quantize);
        self.publish_library_cmd(
            library_api::Kind::SaveHotCue,
            library_api::CmdBody::SaveHotCue {
                track_id: track_id.as_str().to_string(),
                slot,
                position_ms,
                loop_length_beats: None,
                color: None,
                label: None,
            },
        )
    }

    fn publish_library_cmd(
        &self,
        kind: library_api::Kind,
        body: library_api::CmdBody,
    ) -> Result<()> {
        let bus = self.library_cmd.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Engine has no library cmd bus for deck performance persistence")
        })?;
        let bytes = library_api::encode_cmd_body(&body)
            .map_err(|e| anyhow::anyhow!("encode library cmd: {e}"))?;
        bus.publish(omnibus::Event::new(
            library_api::Origin::Library,
            kind,
            Arc::from(bytes),
        ))
        .map_err(|e| anyhow::anyhow!("library cmd publish: {e}"))?;
        Ok(())
    }

    /// Recall a saved loop region: activate, seek to in, play.
    pub fn recall_deck_saved_loop(
        &mut self,
        deck_id: usize,
        in_ms: i32,
        out_ms: i32,
    ) -> Result<()> {
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!(
                "Load a track before recalling a saved loop."
            ));
        }
        self.set_deck_loop_region(deck_id, in_ms, out_ms)?;
        self.seek_deck(deck_id, in_ms)?;
        self.play(deck_id)
    }

    /// Set controller pad mode for a deck (UI mode; no audio side effects).
    pub fn set_deck_pad_mode(&mut self, deck_id: usize, mode: PadMode) -> Result<()> {
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.pad_mode = mode;
        Ok(())
    }

    /// Begin a temporary loop roll; stashes the prior active loop for restore.
    pub fn begin_deck_loop_roll(&mut self, deck_id: usize, beats: f32) -> Result<()> {
        if !beats.is_finite() || beats <= 0.0 {
            return Err(anyhow::anyhow!(
                "Loop roll requires a positive finite beat length."
            ));
        }
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let bpm = bpm.ok_or_else(|| anyhow::anyhow!("Track BPM is required for loop roll."))?;
        let (position_ms, duration_ms) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let restore = self.active_loop_region(deck_id);
        let control = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?;
        control.loop_roll_restore = restore;

        let beat_len = 60.0 / bpm;
        let in_ms = snap_ms(position_ms, Some(bpm), quantize);
        let in_secs = ms_to_secs(in_ms);
        let duration = ms_to_secs(duration_ms);
        let out_ms = secs_to_ms((in_secs + beat_len * f64::from(beats)).min(duration));
        self.set_deck_loop_region(deck_id, in_ms, out_ms)
    }

    /// End loop roll; restore stashed loop or clear.
    pub fn end_deck_loop_roll(&mut self, deck_id: usize) -> Result<()> {
        let restore = self
            .deck_control
            .get_mut(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {}", deck_id))?
            .loop_roll_restore
            .take();
        if let Some(region) = restore.filter(|region| region.active) {
            self.set_deck_loop_region(deck_id, region.in_ms, region.out_ms)
        } else {
            self.clear_deck_loop(deck_id)
        }
    }

    fn active_loop_region(&self, deck_id: usize) -> Option<LoopRegion> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        deck.loop_region_ms().map(|(in_ms, out_ms)| LoopRegion {
            in_ms,
            out_ms,
            active: true,
        })
    }

    /// Set cue point to the snapped playhead.
    pub fn set_deck_cue_point_at_playhead(&mut self, deck_id: usize) -> Result<()> {
        if !self.deck_has_audio_loaded(deck_id).unwrap_or(false) {
            return Err(anyhow::anyhow!("Load a track before setting cue."));
        }
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let target = snap_ms(position_ms, bpm, quantize);
        self.set_deck_cue_point(deck_id, target)
    }

    /// Auto-loop `beats` from the snapped playhead.
    pub fn set_deck_auto_loop(&mut self, deck_id: usize, beats: f32) -> Result<()> {
        if !beats.is_finite() || beats <= 0.0 {
            return Err(anyhow::anyhow!(
                "Loop length must be a positive finite beat count."
            ));
        }
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let bpm = bpm.ok_or_else(|| anyhow::anyhow!("Track BPM is required for auto loop."))?;
        let (position_ms, duration_ms) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let beat_len = 60.0 / bpm;
        let in_ms = snap_ms(position_ms, Some(bpm), quantize);
        let in_secs = ms_to_secs(in_ms);
        let duration = ms_to_secs(duration_ms);
        let out_ms = secs_to_ms((in_secs + beat_len * f64::from(beats)).min(duration));
        self.set_deck_loop_region(deck_id, in_ms, out_ms)
    }

    /// Move loop-in to the snapped playhead (keeps existing out, or default 4 beats).
    pub fn set_deck_loop_in_at_playhead(&mut self, deck_id: usize) -> Result<()> {
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let in_ms = snap_ms(position_ms, bpm, quantize);
        let default_out = in_ms + secs_to_ms(60.0 / bpm.unwrap_or(120.0) * 4.0);
        let out_ms = self
            .deck_transport_state(deck_id)
            .and_then(|(_, loop_region)| loop_region.map(|(_, out)| out))
            .unwrap_or(default_out);
        self.set_deck_loop_region(deck_id, in_ms, out_ms.max(in_ms + 10))
    }

    /// Move loop-out to the snapped playhead (keeps existing in, or 0).
    pub fn set_deck_loop_out_at_playhead(&mut self, deck_id: usize) -> Result<()> {
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let out_ms = snap_ms(position_ms, bpm, quantize);
        let in_ms = self
            .deck_transport_state(deck_id)
            .and_then(|(_, loop_region)| loop_region.map(|(inn, _)| inn))
            .unwrap_or(0);
        if out_ms <= in_ms {
            return Err(anyhow::anyhow!("Loop out must be after loop in."));
        }
        self.set_deck_loop_region(deck_id, in_ms, out_ms)
    }

    /// Jump playhead by `beats` (negative = backward), optionally snapped.
    pub fn beat_jump_deck(&mut self, deck_id: usize, beats: f32) -> Result<()> {
        if !beats.is_finite() || beats == 0.0 {
            return Err(anyhow::anyhow!(
                "Beat jump requires a non-zero finite beat count."
            ));
        }
        let (bpm, quantize) = self.deck_bpm_quantize(deck_id)?;
        let bpm = bpm.ok_or_else(|| anyhow::anyhow!("Track BPM is required for beat jump."))?;
        let (position_ms, _) = self.deck_playback_ms(deck_id).unwrap_or((0, 0));
        let beat_len = 60.0 / bpm;
        let raw = ms_to_secs(position_ms) + beat_len * f64::from(beats);
        let target = snap_ms(secs_to_ms(raw), Some(bpm), quantize);
        self.seek_deck(deck_id, target)
    }

    fn deck_bpm_quantize(&self, deck_id: usize) -> Result<(Option<f64>, bool)> {
        let control = self
            .deck_control
            .get(deck_id)
            .ok_or_else(|| anyhow::anyhow!("Invalid deck ID: {deck_id}"))?;
        Ok((control.bpm, control.quantize))
    }

    /// Cue point and loop region for status mirroring.
    pub fn deck_transport_state(&self, deck_id: usize) -> Option<DeckTransportState> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        Some((deck.cue_point_ms(), deck.loop_region_ms()))
    }

    /// Soft-takeover latch table (absolute MIDI controls).
    pub fn soft_takeover_mut(&mut self) -> &mut crate::soft_takeover::SoftTakeoverState {
        &mut self.soft_takeover
    }

    /// Channel strip + tempo readbacks as wire `0..1` for soft-takeover compares.
    pub fn deck_strip_norms(&self, deck_id: usize) -> Option<DeckStripNorms> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        let channel = dsp.mixer().channel(deck_id)?;
        let eq = channel.eq_gains();
        Some(DeckStripNorms {
            volume: channel.volume(),
            filter: crate::control_norm::strip_db_to_norm(channel.filter_db()),
            gain_trim: crate::control_norm::strip_db_to_norm(channel.gain_trim_db()),
            eq_low: crate::control_norm::strip_db_to_norm(eq.low_db),
            eq_mid: crate::control_norm::strip_db_to_norm(eq.mid_db),
            eq_high: crate::control_norm::strip_db_to_norm(eq.high_db),
            speed: deck.speed(),
            headphone_cue: channel.headphone_cue(),
        })
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

    pub fn crossfader(&self) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().crossfader())
    }

    /// Set cue blend (0.0 = PFL only, 1.0 = master tap only when `master_cue`).
    pub fn set_cue_mix(&mut self, mix: f32) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_cue_mix(mix)
    }

    /// Current cue blend when the engine is running.
    pub fn cue_mix(&self) -> Option<f32> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().cue_mix())
    }

    /// Enable or disable master tap on the cue bus.
    pub fn set_master_cue(&mut self, enabled: bool) -> Result<()> {
        let dsp_engine = self
            .dsp_engine
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Engine is not running"))?;
        let mut dsp = dsp_engine.lock().unwrap();
        dsp.mixer_mut().set_master_cue(enabled);
        Ok(())
    }

    /// Whether master tap is enabled on the cue bus when the engine is running.
    pub fn master_cue(&self) -> Option<bool> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.mixer().master_cue())
    }

    /// Playback position and duration for a deck (milliseconds), when the engine is running.
    pub fn deck_playback_ms(&self, deck_id: usize) -> Option<(i32, i32)> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let deck = dsp.deck(deck_id)?;
        let duration = secs_to_ms(deck.duration_seconds()?);
        let position = deck.position_ms().unwrap_or(0);
        Some((position, duration))
    }

    /// Whether a deck has loaded audio when the engine is running.
    pub fn deck_has_audio_loaded(&self, deck_id: usize) -> Option<bool> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        Some(dsp.deck(deck_id)?.has_audio_loaded())
    }

    /// Slim deck snapshot for bus `Updated` events.
    pub fn deck_snapshot(&self, deck_id: usize) -> Option<DeckSnapshot> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let control = self.deck_control.get(deck_id)?;
        deck_snapshot_from_dsp(
            &dsp,
            deck_id,
            control.sync_mode,
            control.quantize,
            control.pad_mode,
        )
    }

    /// Full engine snapshot for bus `Status` events.
    pub fn engine_status_snapshot(&self) -> Option<EngineStatus> {
        let dsp_engine = self.dsp_engine.as_ref()?;
        let dsp = dsp_engine.lock().ok()?;
        let mut decks = Vec::with_capacity(dsp.num_decks());
        for deck_id in 0..dsp.num_decks() {
            let (sync_mode, quantize, pad_mode) = self
                .deck_control
                .get(deck_id)
                .map(|d| (d.sync_mode, d.quantize, d.pad_mode))
                .unwrap_or((SyncMode::Off, true, PadMode::HotCue));
            if let Some(snapshot) =
                deck_snapshot_from_dsp(&dsp, deck_id, sync_mode, quantize, pad_mode)
            {
                decks.push(snapshot);
            }
        }
        Some(EngineStatus {
            running: true,
            sample_rate: self.config.sample_rate,
            crossfader: dsp.mixer().crossfader(),
            cue_mix: dsp.mixer().cue_mix(),
            master_cue: dsp.mixer().master_cue(),
            master_deck: self.master_deck as u16,
            decks,
            sampler: SamplerStatus {
                banks: Vec::new(),
                active_bank_id: None,
                active_bank_name: None,
                bank_play_mode: None,
                deck_slots: vec![Vec::new(); NUM_DECKS],
                effective_play_modes: vec![Default::default(); NUM_DECKS],
            },
        })
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
        let mapping = crate::routing::validate_channel_mapping(&new_config.channels)?;
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

    /// Set bus device mapping to a stereo channel pair (1-based).
    pub fn set_bus_device(
        &mut self,
        bus: BusId,
        device: DeviceId,
        channels: [u16; 2],
    ) -> Result<()> {
        let mapping = crate::routing::validate_channel_pair(channels)?;
        self.set_bus_channel_mapping(bus, device, mapping)
    }

    /// Set bus device mapping (stereo pair or mono fold).
    pub fn set_bus_channel_mapping(
        &mut self,
        bus: BusId,
        device: DeviceId,
        channels: ChannelMapping,
    ) -> Result<()> {
        let mapping = crate::routing::validate_channel_mapping(&channels)?;
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
            self.config
                .buses
                .push(BusConfig::new(bus, name, resolved, mapping));
        }
        Ok(())
    }
}

fn deck_snapshot_from_dsp(
    dsp: &DspEngine,
    deck_id: usize,
    sync_mode: SyncMode,
    quantize: bool,
    pad_mode: PadMode,
) -> Option<DeckSnapshot> {
    let deck = dsp.deck(deck_id)?;
    let channel = dsp.mixer().channel(deck_id)?;
    let eq = channel.eq_gains();
    let (position_ms, duration_ms) = match deck.duration_seconds() {
        Some(duration) => (
            Some(deck.position_ms().unwrap_or(0)),
            Some(secs_to_ms(duration)),
        ),
        None => (None, None),
    };
    let active_loop = deck.loop_region_ms().map(|(in_ms, out_ms)| LoopRegion {
        in_ms,
        out_ms,
        active: true,
    });
    Some(DeckSnapshot {
        id: deck_id as u16,
        track: None,
        track_id: None,
        title: None,
        artist: None,
        bpm: None,
        key: None,
        playing: matches!(deck.state(), DeckState::Playing),
        volume: channel.volume(),
        speed: deck.speed(),
        eq: DeckEq {
            low: crate::control_norm::strip_db_to_norm(eq.low_db),
            mid: crate::control_norm::strip_db_to_norm(eq.mid_db),
            high: crate::control_norm::strip_db_to_norm(eq.high_db),
        },
        filter: crate::control_norm::strip_db_to_norm(channel.filter_db()),
        gain_trim: crate::control_norm::strip_db_to_norm(channel.gain_trim_db()),
        headphone_cue: channel.headphone_cue(),
        sync_mode,
        cue_point_ms: deck.cue_point_ms(),
        quantize,
        active_loop,
        pad_mode,
        position_ms,
        duration_ms,
        hot_cues: Vec::new(),
        saved_loops: Vec::new(),
        loudness_lufs: None,
        auto_gain_db: 0.0,
        active_sampler_bank_id: None,
        top_jog_mode: map_jog_mode_to_api(deck.top_jog_mode()),
        outer_jog_mode: map_jog_mode_to_api(deck.outer_jog_mode()),
        jog_touching: deck.jog_touching(),
    })
}

fn map_jog_mode(mode: engine_api::JogMode) -> engine_dsp::JogMode {
    match mode {
        engine_api::JogMode::Vinyl => engine_dsp::JogMode::Vinyl,
        engine_api::JogMode::PitchBend => engine_dsp::JogMode::PitchBend,
        engine_api::JogMode::Ignore => engine_dsp::JogMode::Ignore,
    }
}

fn map_jog_mode_to_api(mode: engine_dsp::JogMode) -> engine_api::JogMode {
    match mode {
        engine_dsp::JogMode::Vinyl => engine_api::JogMode::Vinyl,
        engine_dsp::JogMode::PitchBend => engine_api::JogMode::PitchBend,
        engine_dsp::JogMode::Ignore => engine_api::JogMode::Ignore,
    }
}

fn loudness_from_metadata(metadata: &TrackMetadata) -> Option<f64> {
    if let Some(gain_db) = metadata.replaygain_track_gain_db.filter(|g| g.is_finite()) {
        return Some(analyzer_core::loudness_lufs_from_replaygain_track_gain_db(
            gain_db,
        ));
    }
    metadata.loudness_lufs.filter(|l| l.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_core::{BusId, ChannelMapping, DeviceId, LoadableAudio, LoadedAudio};
    use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
    use std::path::PathBuf;
    use std::sync::Arc;

    fn test_source(loudness_lufs: Option<f64>) -> AudioSource {
        AudioSource::File(FileAudioSource::new(
            TrackId::new("test.wav"),
            PathBuf::from("/no/such/file.wav"),
            TrackMetadata {
                loudness_lufs,
                ..Default::default()
            },
        ))
    }

    /// Bypass file decode by inserting PCM into the engine cache before load.
    fn cache_empty_pcm(engine: &mut Engine, track_id: &str) {
        engine.decode_cache.insert(
            TrackId::new(track_id),
            Arc::new(LoadedAudio {
                samples: vec![],
                sample_rate: 48_000,
                channels: 2,
                source_id: track_id.to_string(),
            }),
        );
    }

    fn started_null_engine() -> Engine {
        let config = EngineConfig {
            backend: "null".to_string(),
            ..Default::default()
        };
        let mut engine = Engine::new(config).unwrap();
        engine.start().unwrap();
        engine
    }

    /// Inspect the mixer channel that backs `deck_id`, bypassing the public API so delegation
    /// tests observe mixer-owned state directly instead of trusting the same setter/getter pair.
    fn with_channel<T>(
        engine: &Engine,
        deck_id: usize,
        f: impl FnOnce(&engine_dsp::MixerChannel) -> T,
    ) -> T {
        let dsp_engine = engine.dsp_engine.as_ref().expect("engine running");
        let dsp = dsp_engine.lock().unwrap();
        let channel = dsp
            .mixer()
            .channel(deck_id)
            .expect("mixer channel must exist for a valid deck id");
        f(channel)
    }

    #[test]
    fn load_track_sets_channel_loudness_from_metadata() {
        let mut engine = started_null_engine();
        engine.set_normalizer_target(Some(-18.0)).unwrap();
        cache_empty_pcm(&mut engine, "test.wav");

        engine.load_track(0, test_source(Some(-24.0))).unwrap();

        assert_eq!(with_channel(&engine, 0, |c| c.loudness_lufs()), Some(-24.0));
        assert_eq!(with_channel(&engine, 0, |c| c.auto_gain_db()), 6.0);
        assert_eq!(engine.deck_auto_gain_db(0), Some(6.0));

        engine.stop().unwrap();
    }

    #[test]
    fn load_track_invalid_deck_id_does_not_partially_mutate_state() {
        let mut engine = started_null_engine();
        engine.set_normalizer_target(Some(-18.0)).unwrap();
        cache_empty_pcm(&mut engine, "test.wav");
        engine.load_track(0, test_source(Some(-21.0))).unwrap();

        let err = engine.load_track(2, test_source(Some(-27.0))).unwrap_err();
        assert!(err.to_string().contains("Invalid deck ID"));

        assert_eq!(with_channel(&engine, 0, |c| c.loudness_lufs()), Some(-21.0));
        assert_eq!(with_channel(&engine, 0, |c| c.auto_gain_db()), 3.0);

        engine.stop().unwrap();
    }

    #[test]
    fn deck_volume_delegates_to_mixer_channel() {
        let mut engine = started_null_engine();

        engine.set_deck_volume(0, 0.25).unwrap();

        assert_eq!(with_channel(&engine, 0, |c| c.volume()), 0.25);

        engine.stop().unwrap();
    }

    #[test]
    fn deck_headphone_cue_delegates_to_mixer_channel() {
        let mut engine = started_null_engine();

        engine.set_deck_headphone_cue(0, true).unwrap();

        assert!(with_channel(&engine, 0, |c| c.headphone_cue()));

        engine.stop().unwrap();
    }

    #[test]
    fn deck_eq_delegates_to_mixer_channel() {
        let mut engine = started_null_engine();

        engine.set_deck_eq_bands(0, 4.0, -2.0, 1.0).unwrap();

        assert_eq!(
            with_channel(&engine, 0, |c| c.eq_gains()),
            DeckEqGains::clamped(4.0, -2.0, 1.0)
        );

        engine.stop().unwrap();
    }

    #[test]
    fn deck_filter_delegates_to_mixer_channel() {
        let mut engine = started_null_engine();

        engine.set_deck_filter_db(0, 5.0).unwrap();

        assert_eq!(with_channel(&engine, 0, |c| c.filter_db()), 5.0);

        engine.stop().unwrap();
    }

    #[test]
    fn deck_gain_trim_delegates_to_mixer_channel() {
        let mut engine = started_null_engine();

        engine.set_deck_gain_trim_db(0, 1.5).unwrap();

        assert_eq!(with_channel(&engine, 0, |c| c.gain_trim_db()), 1.5);

        engine.stop().unwrap();
    }

    #[test]
    fn deck_level_snapshot_reads_peaks_from_mixer_channel_not_deck() {
        let mut engine = started_null_engine();

        engine.decode_cache.insert(
            TrackId::new("tone.wav"),
            Arc::new(LoadedAudio {
                samples: vec![0.5f32; 48_000 * 2],
                sample_rate: 48_000,
                channels: 2,
                source_id: "tone.wav".to_string(),
            }),
        );
        engine
            .load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    TrackId::new("tone.wav"),
                    PathBuf::from("/no/such/tone.wav"),
                    TrackMetadata::default(),
                )),
            )
            .unwrap();
        engine.play(0).unwrap();

        // The null backend never drains the pre-filled ring buffer, so the background
        // producer thread never calls `DspEngine::process`; drive one render cycle directly
        // through the same lock the producer thread uses.
        {
            let dsp_engine = engine.dsp_engine.as_ref().unwrap();
            let mut dsp = dsp_engine.lock().unwrap();
            let mut output_buses = std::collections::HashMap::new();
            output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
            dsp.process(512, &mut output_buses).unwrap();
        }

        let snapshot_peak = engine
            .deck_level_snapshot()
            .into_iter()
            .find(|(deck_id, _, _)| *deck_id == 0)
            .map(|(_, peak_l, _)| peak_l);

        assert!(
            snapshot_peak.unwrap_or(0.0) > 0.0,
            "level snapshot should report a non-zero pre-fader peak once a channel processes audio"
        );

        // A constant-amplitude tone yields the same peak on every render cycle, so this
        // comparison is robust to how many cycles the producer thread ran in the background.
        let channel_peak = with_channel(&engine, 0, |c| c.level_peaks().peak_l);
        assert_eq!(
            snapshot_peak,
            Some(channel_peak),
            "deck_level_snapshot must mirror mixer channel peaks, not deck-owned state"
        );

        engine.stop().unwrap();
    }

    #[test]
    fn unload_deck_clears_loudness_but_preserves_manual_controls() {
        let mut engine = started_null_engine();
        engine.set_normalizer_target(Some(-18.0)).unwrap();
        cache_empty_pcm(&mut engine, "test.wav");
        engine.load_track(0, test_source(Some(-24.0))).unwrap();
        engine.set_deck_volume(0, 0.4).unwrap();
        engine.set_deck_eq_bands(0, 2.0, -1.0, 3.0).unwrap();
        engine.set_deck_filter_db(0, 5.0).unwrap();
        engine.set_deck_gain_trim_db(0, 1.5).unwrap();
        engine.set_deck_headphone_cue(0, true).unwrap();

        engine.unload_deck(0).unwrap();

        assert_eq!(
            with_channel(&engine, 0, |c| c.loudness_lufs()),
            None,
            "loudness must clear on unload"
        );
        assert_eq!(
            with_channel(&engine, 0, |c| c.auto_gain_db()),
            0.0,
            "auto gain must reset on unload"
        );
        assert_eq!(with_channel(&engine, 0, |c| c.volume()), 0.4);
        assert_eq!(with_channel(&engine, 0, |c| c.filter_db()), 5.0);
        assert_eq!(with_channel(&engine, 0, |c| c.gain_trim_db()), 1.5);
        assert!(with_channel(&engine, 0, |c| c.headphone_cue()));
        assert_eq!(
            with_channel(&engine, 0, |c| c.eq_gains()),
            DeckEqGains::clamped(2.0, -1.0, 3.0)
        );

        engine.stop().unwrap();
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
        assert!(engine.load_track(0, test_source(None)).is_err());

        engine.start().unwrap();

        assert!(engine.play(0).is_ok());
        assert!(engine.pause(0).is_ok());
        let missing = FileAudioSource::from_path("test.mp3").load();
        assert!(missing.is_err());
        assert!(missing.unwrap_err().to_string().contains("not found"));

        assert!(engine.play(2).is_err());
        assert!(engine.load_track(2, test_source(None)).is_err());

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
        let config = EngineConfig {
            backend: "null".into(),
            ..Default::default()
        };
        let mut engine = Engine::new(config).unwrap();
        engine
            .set_bus_device(BusId::new("master"), DeviceId::new("null-device"), [3, 4])
            .unwrap();
        let bus = engine.get_bus_config(&BusId::new("master")).unwrap();
        assert_eq!(bus.channels.left, 3);
        assert_eq!(bus.channels.right, 4);
        assert_eq!(bus.device.as_str(), "null-device");
    }

    #[test]
    fn set_bus_device_rejects_overlap_on_same_device() {
        let config = EngineConfig {
            backend: "null".into(),
            buses: vec![
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
            ],
            ..Default::default()
        };
        let mut engine = Engine::new(config).unwrap();
        let err = engine
            .set_bus_device(BusId::new("cue"), DeviceId::new("null-device"), [2, 3])
            .unwrap_err();
        assert!(err.to_string().contains("overlaps"));
    }

    #[test]
    fn set_bus_channel_mapping_accepts_mono() {
        let config = EngineConfig {
            backend: "null".into(),
            ..Default::default()
        };
        let mut engine = Engine::new(config).unwrap();
        engine
            .set_bus_channel_mapping(
                BusId::new("master"),
                DeviceId::new("null-device"),
                ChannelMapping::mono(1),
            )
            .unwrap();
        engine
            .set_bus_channel_mapping(
                BusId::new("cue"),
                DeviceId::new("null-device"),
                ChannelMapping::mono(2),
            )
            .unwrap();
        let master = engine.get_bus_config(&BusId::new("master")).unwrap();
        assert!(master.channels.is_mono());
        assert_eq!(master.channels.left, 1);
        let cue = engine.get_bus_config(&BusId::new("cue")).unwrap();
        assert!(cue.channels.is_mono());
        assert_eq!(cue.channels.left, 2);
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
