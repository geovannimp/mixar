//! FRB `AudioBackendTransport` (settings discovery) + `EngineTransport` (session + bus).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{
    deck_snapshot_to_evt, spawn_engine_worker, AudioBackend, AudioBackendTrait, Engine,
    EngineBuses, EngineConfig, EngineWorker, Evt,
};
use library::{LibraryManager, PreparedTrackPlayback};
use library_core::TrackId;

use crate::api::library::LibraryTransport;
use crate::frb_generated::StreamSink;

/// Output device summary for the Flutter settings / smoke UI.
#[derive(Clone, Debug)]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub max_channels: u16,
    pub default_sample_rates: Vec<u32>,
}

/// Config passed to [`EngineTransport::start`] — maps onto [`EngineConfig`].
#[derive(Clone, Debug)]
pub struct EngineStartConfig {
    pub backend: String,
    pub sample_rate: Option<u32>,
    pub buffer_size: Option<u32>,
}

impl EngineStartConfig {
    fn to_engine_config(&self) -> EngineConfig {
        let mut config = EngineConfig {
            backend: self.backend.clone(),
            ..EngineConfig::default()
        };
        if let Some(sr) = self.sample_rate {
            config.sample_rate = sr;
        }
        if let Some(bs) = self.buffer_size {
            config.buffer_size = bs;
        }
        config
    }
}

/// Settings-only backend discovery (`AudioBackend::new` + `list_output_devices`).
#[flutter_rust_bridge::frb(opaque)]
pub struct AudioBackendTransport {
    backend: Box<dyn AudioBackendTrait>,
}

impl AudioBackendTransport {
    /// Compiled-in backend names, with `"auto"` first (config default; not from `list_names`).
    #[flutter_rust_bridge::frb(sync)]
    pub fn list_names() -> Vec<String> {
        let mut names = AudioBackend::list_names();
        if !names.iter().any(|n| n == "auto") {
            names.insert(0, "auto".into());
        }
        names
    }

    /// Open a backend by name (`AudioBackend::new`).
    pub fn open(name: String) -> Result<Self, String> {
        let backend = AudioBackend::new(&name).map_err(|e| e.to_string())?;
        Ok(Self { backend })
    }

    /// List output devices for this backend instance.
    pub fn list_output_devices(&self) -> Result<Vec<OutputDevice>, String> {
        let devices = self
            .backend
            .list_output_devices()
            .map_err(|e| e.to_string())?;
        Ok(devices
            .into_iter()
            .map(|d| OutputDevice {
                id: d.id.as_str().to_string(),
                name: d.name,
                is_default: d.is_default,
                max_channels: d.max_channels,
                default_sample_rates: d.default_sample_rates,
            })
            .collect())
    }
}

/// Discriminator for thin engine egress (unit enum — no freezed on Dart).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineEvtKind {
    Status,
    Updated,
    Position,
    Levels,
    Error,
    Notice,
}

/// Thin typed engine egress for Dart (no MessagePack on the Flutter side).
#[derive(Clone, Debug)]
pub struct EngineEvt {
    pub kind: EngineEvtKind,
    pub deck_id: Option<u16>,
    pub running: Option<bool>,
    pub playing: Option<bool>,
    pub track: Option<String>,
    pub track_id: Option<String>,
    pub position_ms: Option<i32>,
    pub peak_l: Option<f32>,
    pub peak_r: Option<f32>,
    pub message: Option<String>,
}

struct EngineEvtForwarder {
    shutdown: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

fn is_coalescible(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Position | Kind::Levels | Kind::Updated | Kind::Status
    )
}

/// Host-owned engine handle exposed to Dart via FRB methods.
#[flutter_rust_bridge::frb(opaque)]
pub struct EngineTransport {
    /// Declared first so Drop joins the worker before `engine` is destroyed.
    #[allow(dead_code)]
    worker: EngineWorker,
    engine: Arc<Mutex<Option<Engine>>>,
    buses: EngineBuses,
    library: Arc<Mutex<LibraryManager>>,
    evt_forwarder: Mutex<Option<EngineEvtForwarder>>,
}

impl Drop for EngineTransport {
    fn drop(&mut self) {
        let mut slot = self.evt_forwarder.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(fwd) = slot.take() {
            fwd.shutdown.store(true, Ordering::Relaxed);
            let _ = fwd.handle.join();
        }
        if let Ok(mut guard) = self.engine.lock() {
            if let Some(engine) = guard.as_mut() {
                let _ = engine.stop();
            }
        }
    }
}

impl EngineTransport {
    /// Start the engine from `config` (`Engine::new`), sharing `library` for load-to-deck.
    pub fn start(
        library_transport: &LibraryTransport,
        config: EngineStartConfig,
    ) -> Result<Self, String> {
        let library_arc = library_transport.library_arc();
        let mut engine = Engine::new_with_library_bus(
            config.to_engine_config(),
            Arc::clone(&library_arc),
            library_transport.cmd_bus(),
        )
        .map_err(|e| e.to_string())?;
        let buses = EngineBuses::new();
        engine.set_buses(buses.clone());
        engine.start().map_err(|e| e.to_string())?;
        let engine = Arc::new(Mutex::new(Some(engine)));
        let worker = match spawn_engine_worker(Arc::clone(&engine)) {
            Ok(worker) => worker,
            Err(e) => {
                if let Ok(mut guard) = engine.lock() {
                    if let Some(eng) = guard.as_mut() {
                        let _ = eng.stop();
                    }
                }
                return Err(e.to_string());
            }
        };
        Ok(Self {
            worker,
            engine,
            buses,
            library: library_arc,
            evt_forwarder: Mutex::new(None),
        })
    }

    /// Stop audio streams; the transport still owns the worker until Drop.
    pub fn stop(&self) -> Result<(), String> {
        let mut guard = self
            .engine
            .lock()
            .map_err(|_| "engine lock poisoned".to_string())?;
        let Some(engine) = guard.as_mut() else {
            return Ok(());
        };
        engine.stop().map_err(|e| e.to_string())
    }

    /// Whether [`Engine::start`] has opened streams.
    pub fn is_running(&self) -> bool {
        self.engine
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(Engine::is_running))
            .unwrap_or(false)
    }

    /// Play a deck (cmd bus).
    pub fn play(&self, deck_id: u16) -> Result<(), String> {
        self.publish_empty(Origin::Deck(deck_id), Kind::Play)
    }

    /// Pause a deck (cmd bus).
    pub fn pause(&self, deck_id: u16) -> Result<(), String> {
        self.publish_empty(Origin::Deck(deck_id), Kind::Pause)
    }

    /// Load a library track: prepare outside the engine lock, then `load_prepared_track`.
    pub fn load_library_track(&self, deck_id: u16, track_id: String) -> Result<(), String> {
        let prepared = LibraryManager::prepare_track_for_playback(
            self.library.as_ref(),
            &TrackId::new(track_id),
        )
        .map_err(|e| e.to_string())?;
        self.load_prepared(deck_id, prepared)
    }

    /// Load a filesystem path: prepare outside the engine lock, then `load_prepared_track`.
    pub fn load_path(&self, deck_id: u16, path: String) -> Result<(), String> {
        let prepared =
            LibraryManager::prepare_file_path_for_playback(self.library.as_ref(), Path::new(&path))
                .map_err(|e| e.to_string())?;
        self.load_prepared(deck_id, prepared)
    }

    fn load_prepared(&self, deck_id: u16, prepared: PreparedTrackPlayback) -> Result<(), String> {
        let snap = {
            let mut guard = self
                .engine
                .lock()
                .map_err(|_| "engine lock poisoned".to_string())?;
            let engine = guard
                .as_mut()
                .ok_or_else(|| "engine not available".to_string())?;
            engine
                .load_prepared_track(deck_id as usize, prepared)
                .map_err(|e| e.to_string())?;
            engine
                .deck_snapshot(deck_id as usize)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?
        };
        self.buses
            .publish_evt(
                Origin::Deck(deck_id),
                Kind::Updated,
                deck_snapshot_to_evt(snap),
            )
            .map_err(|e| e.to_string())
    }

    fn publish_empty(&self, origin: Origin, kind: Kind) -> Result<(), String> {
        let bytes = encode_cmd_body(&CmdBody::Empty).map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(origin, kind, bytes)
            .map_err(|e| e.to_string())
    }

    /// Forward thin typed engine events to Dart via FRB `StreamSink`.
    ///
    /// Coalesces Position/Levels/Updated/Status (latest wins) like Tauri.
    /// Replaces any previous forwarder so repeated subscribe calls do not leak threads.
    pub fn subscribe_events(&self, sink: StreamSink<EngineEvt>) -> Result<(), String> {
        let rx = self.buses.subscribe_evt_all().map_err(|e| e.to_string())?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let handle = std::thread::Builder::new()
            .name("engine-evt-forwarder".into())
            .spawn(move || {
                while !shutdown_flag.load(Ordering::Relaxed) {
                    let first = match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(Some(ev)) => ev,
                        Ok(None) => continue,
                        Err(_) => break,
                    };
                    let mut discrete: Vec<Arc<Evt>> = Vec::new();
                    let mut coalesced: HashMap<(Origin, Kind), Arc<Evt>> = HashMap::new();
                    let mut push = |ev: Arc<Evt>| {
                        if is_coalescible(ev.kind()) {
                            coalesced.insert((ev.origin().clone(), ev.kind().clone()), ev);
                        } else {
                            discrete.push(ev);
                        }
                    };
                    push(first);
                    while let Ok(Some(ev)) = rx.recv_timeout(Duration::ZERO) {
                        push(ev);
                    }
                    for ev in discrete.into_iter().chain(coalesced.into_values()) {
                        if let Some(mapped) = map_engine_evt(ev.as_ref()) {
                            if sink.add(mapped).is_err() {
                                return;
                            }
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        let mut slot = self.evt_forwarder.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(prev) = slot.take() {
            prev.shutdown.store(true, Ordering::Relaxed);
            let _ = prev.handle.join();
        }
        *slot = Some(EngineEvtForwarder { shutdown, handle });
        Ok(())
    }

    /// Raw evt subscription for host/tests. Prefer [`Self::subscribe_events`] for Dart.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn subscribe_evt_all(&self) -> Result<engine_core::EvtReceiver, String> {
        self.buses.subscribe_evt_all().map_err(|e| e.to_string())
    }
}

fn deck_id_of(origin: &Origin) -> Option<u16> {
    match origin {
        Origin::Deck(id) => Some(*id),
        Origin::Engine | Origin::Mixer => None,
    }
}

pub(crate) fn map_engine_evt(ev: &Evt) -> Option<EngineEvt> {
    let body = decode_evt_body(ev.payload()).ok()?;
    let deck_id = deck_id_of(ev.origin());
    match body {
        EvtBody::EngineStatus { status } => Some(EngineEvt {
            kind: EngineEvtKind::Status,
            deck_id,
            running: Some(status.running),
            playing: None,
            track: None,
            track_id: None,
            position_ms: None,
            peak_l: None,
            peak_r: None,
            message: None,
        }),
        EvtBody::DeckUpdated {
            playing,
            track,
            track_id,
            position_ms,
            ..
        } => Some(EngineEvt {
            kind: EngineEvtKind::Updated,
            deck_id,
            running: None,
            playing: Some(playing),
            track,
            track_id,
            position_ms,
            peak_l: None,
            peak_r: None,
            message: None,
        }),
        EvtBody::Position { position_ms } => Some(EngineEvt {
            kind: EngineEvtKind::Position,
            deck_id,
            running: None,
            playing: None,
            track: None,
            track_id: None,
            position_ms: Some(position_ms),
            peak_l: None,
            peak_r: None,
            message: None,
        }),
        EvtBody::Levels { peak_l, peak_r, .. } => Some(EngineEvt {
            kind: EngineEvtKind::Levels,
            deck_id,
            running: None,
            playing: None,
            track: None,
            track_id: None,
            position_ms: None,
            peak_l: Some(peak_l),
            peak_r: Some(peak_r),
            message: None,
        }),
        EvtBody::Error { message } => Some(EngineEvt {
            kind: EngineEvtKind::Error,
            deck_id,
            running: None,
            playing: None,
            track: None,
            track_id: None,
            position_ms: None,
            peak_l: None,
            peak_r: None,
            message: Some(message),
        }),
        EvtBody::Notice { message } => Some(EngineEvt {
            kind: EngineEvtKind::Notice,
            deck_id,
            running: None,
            playing: None,
            track: None,
            track_id: None,
            position_ms: None,
            peak_l: None,
            peak_r: None,
            message: Some(message),
        }),
        EvtBody::Empty => None,
    }
}
