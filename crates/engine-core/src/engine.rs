use crate::backend::create_backend;
use crate::bus::{EngineBuses, EvtReceiver};
use crate::callback::ConsumerCallback;
use crate::config::EngineConfig;
use crate::producer::{
    create_device_ring_buffer, producer_thread_loop, start_device_streams, DeviceStreamSetup,
};
use crate::routing::DeviceStreamPlan;
use crate::sync::DeckControlState;
use crate::transport::TransportEvent;
use anyhow::Result;
use audio_core::{AudioStream, Sample, StreamParams};
use engine_api::{EvtBody, Kind, Origin};
use engine_dsp::DspEngine;
use library::{LibraryBus, LibraryManager};
use rtrb::Producer;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub(crate) const NUM_DECKS: usize = 2;

/// Cue point (ms) and optional loop region `(start, end)` for status mirroring.
pub(crate) type DeckTransportState = (Option<i32>, Option<(i32, i32)>);

/// Main engine struct
pub struct Engine {
    pub(crate) config: EngineConfig,
    pub(crate) dsp_engine: Option<Arc<Mutex<DspEngine>>>,
    pub(crate) backend: Box<dyn audio_core::AudioBackend>,
    /// One opened stream per resolved device plan (spec §5.3: buses sharing a device share a stream).
    pub(crate) streams: Vec<Box<dyn AudioStream>>,
    pub(crate) producer_thread: Option<JoinHandle<()>>,
    pub(crate) running: Arc<Mutex<bool>>,
    pub(crate) transport_events: Arc<Mutex<Vec<TransportEvent>>>,
    pub(crate) library: Option<Arc<Mutex<LibraryManager>>>,
    /// Optional library cmd bus for deck performance persistence (hot cues / loops).
    pub(crate) library_cmd: Option<LibraryBus>,
    pub(crate) engine_buses: Option<EngineBuses>,
    pub(crate) master_deck: usize,
    pub(crate) deck_control: Vec<DeckControlState>,
    pub(crate) soft_takeover: crate::soft_takeover::SoftTakeoverState,
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
            engine_buses: None,
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

    /// Attach host-owned omnibus buses (clones stored on the engine).
    pub fn set_buses(&mut self, buses: EngineBuses) {
        self.engine_buses = Some(buses);
    }

    /// Clone of attached buses, if any.
    pub fn buses(&self) -> Option<EngineBuses> {
        self.engine_buses.clone()
    }

    /// Shared library manager handle (clone of the engine-held Arc).
    pub fn library_manager(&self) -> Option<Arc<Mutex<LibraryManager>>> {
        self.library.clone()
    }

    fn require_buses(&self) -> Result<&EngineBuses> {
        self.engine_buses
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("engine buses not attached; call set_buses first"))
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.require_buses()?.publish_cmd(origin, kind, body)
    }

    /// Encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        self.require_buses()?.publish_evt(origin, kind, body)
    }

    /// Subscribe to all egress events (host bridge / MIDI).
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        self.require_buses()?.subscribe_evt_all()
    }

    /// Whether [`Self::start`] has opened streams (producer running).
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Start the engine
    pub fn start(&mut self) -> Result<()> {
        if !self.streams.is_empty() {
            return Err(anyhow::anyhow!("Engine is already running"));
        }

        tracing::info!("Starting engine with backend: {}", self.backend.name());
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

        tracing::info!(
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

            tracing::info!(
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
        {
            let default_range = self.config.default_tempo_range();
            let default_key_lock = self.config.default_key_lock();
            let vinyl_brake = self.config.vinyl_brake_pause();
            let vinyl_brake_stop = self.config.vinyl_brake_stop_secs();
            let mut dsp = dsp_engine.lock().unwrap();
            for deck_id in 0..dsp.num_decks() {
                if let Some(deck) = dsp.deck_mut(deck_id) {
                    deck.set_tempo_range(default_range);
                    let _ = deck.set_key_lock(default_key_lock);
                    deck.set_vinyl_brake(vinyl_brake, vinyl_brake_stop);
                }
            }
        }
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

        // No warm-up sleep: `create_device_ring_buffer` already pre-fills the ring
        // with silence synchronously, and the producer is parked on `filled >=
        // target_fill` until a device callback drains it. A fixed sleep here only
        // added startup latency (200 ms per start, ~46 starts in the test suite).

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
        tracing::info!("Stopping engine");

        *self.running.lock().unwrap() = false;
        if let Some(thread) = self.producer_thread.take() {
            thread
                .join()
                .map_err(|_| anyhow::anyhow!("Failed to join producer thread"))?;
        }

        let stream_count = self.streams.len();
        self.streams.clear();
        if stream_count > 0 {
            tracing::info!("Audio stream(s) stopped ({})", stream_count);
        }

        self.dsp_engine = None;
        self.transport_events.lock().unwrap().clear();

        tracing::info!("Engine stopped");
        Ok(())
    }

    /// Drain transport events posted by the producer thread (track ended, etc.).
    pub fn drain_transport_events(&mut self) -> Vec<TransportEvent> {
        std::mem::take(&mut *self.transport_events.lock().unwrap())
    }

    /// Soft-takeover latch table (absolute MIDI controls).
    pub fn soft_takeover_mut(&mut self) -> &mut crate::soft_takeover::SoftTakeoverState {
        &mut self.soft_takeover
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests;
