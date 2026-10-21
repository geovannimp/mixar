//! FRB `AudioBackendTransport` (settings discovery) + `EngineTransport` (session + bus).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, DeckSnapshot, EvtBody, Kind, Origin};
use engine_core::{
    deck_snapshot_to_evt, spawn_engine_worker, AudioBackend, AudioBackendTrait, Engine,
    EngineBuses, EngineConfig, EngineWorker, Evt,
};
use library::{LibraryManager, PreparedTrackPlayback, SamplerSlotRecord};
use library_core::{AudioSource, TrackId};

use crate::api::history_worker::HistoryWorker;
use crate::api::library::LibraryTransport;
use crate::api::settings::{
    seed_engine_config_if_unconfigured, settings_engine_config, settings_host_runtime,
    settings_realtime_stems_enabled, JogModeSetting,
};
use crate::frb_generated::StreamSink;

const NUM_DECKS: usize = 2;
const SAMPLER_SLOT_COUNT: usize = library::SAMPLER_BANK_SIZE;

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
        seed_engine_config_if_unconfigured(&self.backend, self.sample_rate, self.buffer_size)
            .unwrap_or_else(|_| {
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
            })
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

/// EQ band for [`EngineTransport::set_eq_band`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EqBand {
    Low,
    Mid,
    High,
}

impl From<EqBand> for engine_api::EqBand {
    fn from(band: EqBand) -> Self {
        match band {
            EqBand::Low => Self::Low,
            EqBand::Mid => Self::Mid,
            EqBand::High => Self::High,
        }
    }
}

/// FX unit effect selection (FRB mirror of `engine_api::FxKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxKind {
    Off,
    Echo,
    Reverb,
}

impl From<FxKind> for engine_api::FxKind {
    fn from(kind: FxKind) -> Self {
        match kind {
            FxKind::Off => Self::Off,
            FxKind::Echo => Self::Echo,
            FxKind::Reverb => Self::Reverb,
        }
    }
}

impl From<engine_api::FxKind> for FxKind {
    fn from(kind: engine_api::FxKind) -> Self {
        match kind {
            engine_api::FxKind::Off => Self::Off,
            engine_api::FxKind::Echo => Self::Echo,
            engine_api::FxKind::Reverb => Self::Reverb,
        }
    }
}

/// Pad mode for [`EngineTransport::set_pad_mode`] / [`EngineEvt::pad_mode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadMode {
    HotCue,
    LoopRoll,
    BeatJump,
    Sampler,
    Stems,
    Keyboard,
    KeyShift,
}

impl From<PadMode> for engine_api::PadMode {
    fn from(mode: PadMode) -> Self {
        match mode {
            PadMode::HotCue => Self::HotCue,
            PadMode::LoopRoll => Self::LoopRoll,
            PadMode::BeatJump => Self::BeatJump,
            PadMode::Sampler => Self::Sampler,
            PadMode::Stems => Self::Stems,
            PadMode::Keyboard => Self::Keyboard,
            PadMode::KeyShift => Self::KeyShift,
        }
    }
}

impl From<engine_api::PadMode> for PadMode {
    fn from(mode: engine_api::PadMode) -> Self {
        match mode {
            engine_api::PadMode::HotCue => Self::HotCue,
            engine_api::PadMode::LoopRoll => Self::LoopRoll,
            engine_api::PadMode::BeatJump => Self::BeatJump,
            engine_api::PadMode::Sampler => Self::Sampler,
            engine_api::PadMode::Stems => Self::Stems,
            engine_api::PadMode::Keyboard => Self::Keyboard,
            engine_api::PadMode::KeyShift => Self::KeyShift,
        }
    }
}

pub use engine_api::SyncMode;

/// Deck sync follow mode (slave → master).
#[allow(dead_code)] // FRB codegen-only; `EngineEvt.sync_mode` is `engine_api::SyncMode`.
#[flutter_rust_bridge::frb(mirror(SyncMode))]
pub enum _SyncMode {
    Off,
    Tempo,
    Beat,
}

/// Active loop region for Dart (`engine_api::LoopRegion`).
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveLoopInfo {
    pub in_ms: i32,
    pub out_ms: i32,
    pub active: bool,
}

/// Pad chrome for one sampler slot (Tauri `SamplerSlotInfo` shape).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SamplerSlotChrome {
    pub label: Option<String>,
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub duration_ms: Option<i32>,
}

/// Per-slot FX state surfaced to the UI.
#[derive(Clone, Debug, PartialEq)]
pub struct FxSlotChrome {
    pub kind: FxKind,
    pub enabled: bool,
    /// Wet/dry `0..1` (`0` = dry).
    pub wet: f32,
    /// Normalized knob positions for `kind` (unused entries `0.0`).
    pub params: Vec<f32>,
}

fn fx_slot_chrome(slot: &engine_api::FxSlotState) -> FxSlotChrome {
    FxSlotChrome {
        kind: slot.kind.into(),
        enabled: slot.enabled,
        wet: slot.wet,
        params: slot.params.to_vec(),
    }
}

fn empty_deck_sampler_chrome() -> Vec<SamplerSlotChrome> {
    vec![SamplerSlotChrome::default(); SAMPLER_SLOT_COUNT]
}

fn empty_all_sampler_chrome() -> Vec<Vec<SamplerSlotChrome>> {
    vec![empty_deck_sampler_chrome(); NUM_DECKS]
}

fn source_path(source: &AudioSource) -> Option<String> {
    source
        .file()
        .map(|f| f.path().to_string_lossy().into_owned())
}

fn source_label(source: &AudioSource) -> String {
    if let Some(title) = source.metadata().title.as_ref().filter(|t| !t.is_empty()) {
        return title.clone();
    }
    source
        .file()
        .and_then(|f| {
            f.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| source.id().as_str().to_string())
}

fn chrome_from_prepared(prepared: &PreparedTrackPlayback) -> SamplerSlotChrome {
    SamplerSlotChrome {
        label: Some(source_label(&prepared.source)),
        track_id: {
            let id = prepared.track_id.as_str();
            if id.is_empty() {
                None
            } else {
                Some(id.to_string())
            }
        },
        path: source_path(&prepared.source),
        duration_ms: prepared.source.metadata().duration_ms,
    }
}

fn chrome_from_bank_slot(
    record: &SamplerSlotRecord,
    prepared: &PreparedTrackPlayback,
) -> SamplerSlotChrome {
    let mut chrome = chrome_from_prepared(prepared);
    if let Some(label) = record.label.clone().filter(|s| !s.is_empty()) {
        chrome.label = Some(label);
    }
    if record.track_id.is_some() {
        chrome.track_id = record.track_id.clone();
    }
    if record.path.is_some() {
        chrome.path = record.path.clone();
    }
    chrome
}

fn attach_sampler_chrome(evt: &mut EngineEvt, chrome: &[Vec<SamplerSlotChrome>]) {
    if evt.kind != EngineEvtKind::Updated {
        return;
    }
    let Some(deck_id) = evt.deck_id else {
        return;
    };
    let idx = usize::from(deck_id);
    if idx >= chrome.len() {
        return;
    }
    evt.sampler_slots = Some(chrome[idx].clone());
    evt.sampler_slots_known = true;
}

impl From<engine_api::LoopRegion> for ActiveLoopInfo {
    fn from(region: engine_api::LoopRegion) -> Self {
        Self {
            in_ms: region.in_ms,
            out_ms: region.out_ms,
            active: region.active,
        }
    }
}

/// Thin typed engine egress for Dart (no MessagePack on the Flutter side).
#[derive(Clone, Debug)]
pub struct EngineEvt {
    pub kind: EngineEvtKind,
    pub deck_id: Option<u16>,
    pub running: Option<bool>,
    pub playing: Option<bool>,
    /// Filesystem path of the loaded track (`DeckSnapshot.track`); not a display title.
    pub track_path: Option<String>,
    pub track_id: Option<String>,
    pub position_ms: Option<i32>,
    pub peak_l: Option<f32>,
    pub peak_r: Option<f32>,
    pub peak_hold_l: Option<f32>,
    pub peak_hold_r: Option<f32>,
    pub message: Option<String>,
    pub volume: Option<f32>,
    pub eq_low: Option<f32>,
    pub eq_mid: Option<f32>,
    pub eq_high: Option<f32>,
    pub filter: Option<f32>,
    pub gain_trim: Option<f32>,
    /// FX unit slot state when authored on this evt (`FX_SLOT_COUNT` entries).
    pub fx: Option<Vec<FxSlotChrome>>,
    /// True when this deck's pre-fader signal runs through the FX unit.
    pub fx_routed: Option<bool>,
    pub headphone_cue: Option<bool>,
    pub crossfader: Option<f32>,
    pub master_volume: Option<f32>,
    pub cue_volume: Option<f32>,
    pub cue_mix: Option<f32>,
    pub master_cue: Option<bool>,
    pub duration_ms: Option<i32>,
    pub speed: Option<f32>,
    pub tempo_range: Option<f32>,
    pub key_lock: Option<bool>,
    /// Session key-shift offset in semitones (`-16..=16`; `0` = bypass).
    pub key_shift: Option<f32>,
    /// Keyboard pad semitone page (`1..=4`; default 2).
    pub keyboard_page: Option<u8>,
    /// Key Shift pad semitone page (`1..=5`; default 2).
    pub key_shift_page: Option<u8>,
    /// Hot-cue slot used as the Keyboard pad root (default 0).
    pub keyboard_root_hot_cue: Option<u8>,
    pub pad_mode: Option<PadMode>,
    pub sync_mode: Option<SyncMode>,
    pub master_deck: Option<u16>,
    /// Set on every [`EngineEvtKind::Updated`] (including `None` when cleared).
    pub active_loop: Option<ActiveLoopInfo>,
    /// True when [`Self::active_loop`] was authored on this Updated evt (even if `None`).
    #[cfg_attr(frb_expand, flutter_rust_bridge::frb(default = false))]
    pub active_loop_known: bool,
    /// Pending Loop In position (ms) before Loop Out completes the region.
    pub pending_loop_in_ms: Option<i32>,
    /// True when [`Self::pending_loop_in_ms`] was authored on this Updated evt (even if `None`).
    #[cfg_attr(frb_expand, flutter_rust_bridge::frb(default = false))]
    pub pending_loop_in_ms_known: bool,
    /// True when [`Self::duration_ms`] was authored on this Updated evt (even if `None`).
    #[cfg_attr(frb_expand, flutter_rust_bridge::frb(default = false))]
    pub duration_known: bool,
    pub quantize: Option<bool>,
    pub slip_enabled: Option<bool>,
    pub slip_shadow_position_ms: Option<i32>,
    pub jog_touching: Option<bool>,
    pub loudness_lufs: Option<f64>,
    pub auto_gain_db: Option<f32>,
    /// Active sampler bank for this deck (`None` when cleared / unset).
    pub active_sampler_bank_id: Option<String>,
    /// True when [`Self::active_sampler_bank_id`] was authored on this Updated evt.
    #[cfg_attr(frb_expand, flutter_rust_bridge::frb(default = false))]
    pub active_sampler_bank_id_known: bool,
    /// Pad chrome for this deck when [`Self::sampler_slots_known`].
    pub sampler_slots: Option<Vec<SamplerSlotChrome>>,
    /// True when [`Self::sampler_slots`] was authored on this Updated evt.
    #[cfg_attr(frb_expand, flutter_rust_bridge::frb(default = false))]
    pub sampler_slots_known: bool,
    /// True when four stem layers are attached for playback.
    pub stems_ready: Option<bool>,
    /// True when the attached stems come from the realtime StemgenRT ring.
    pub stems_live: Option<bool>,
    /// Per-stem mute flags when [`Self::stems_ready`] is authored (`[drums, bass, other, vocals]`).
    pub stem_mute: Option<Vec<bool>>,
    /// Isolated stem index when set; `None` clears isolate when stems are authored.
    pub stem_isolate: Option<u8>,
}

impl EngineEvt {
    fn bare(kind: EngineEvtKind) -> Self {
        Self {
            kind,
            deck_id: None,
            running: None,
            playing: None,
            track_path: None,
            track_id: None,
            position_ms: None,
            peak_l: None,
            peak_r: None,
            peak_hold_l: None,
            peak_hold_r: None,
            message: None,
            volume: None,
            eq_low: None,
            eq_mid: None,
            eq_high: None,
            filter: None,
            gain_trim: None,
            fx: None,
            fx_routed: None,
            headphone_cue: None,
            crossfader: None,
            master_volume: None,
            cue_volume: None,
            cue_mix: None,
            master_cue: None,
            duration_ms: None,
            speed: None,
            tempo_range: None,
            key_lock: None,
            key_shift: None,
            keyboard_page: None,
            key_shift_page: None,
            keyboard_root_hot_cue: None,
            pad_mode: None,
            sync_mode: None,
            master_deck: None,
            active_loop: None,
            active_loop_known: false,
            pending_loop_in_ms: None,
            pending_loop_in_ms_known: false,
            duration_known: false,
            quantize: None,
            slip_enabled: None,
            slip_shadow_position_ms: None,
            jog_touching: None,
            loudness_lufs: None,
            auto_gain_db: None,
            active_sampler_bank_id: None,
            active_sampler_bank_id_known: false,
            sampler_slots: None,
            sampler_slots_known: false,
            stems_ready: None,
            stems_live: None,
            stem_mute: None,
            stem_isolate: None,
        }
    }
}

struct EngineEvtForwarder {
    shutdown: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

#[allow(dead_code)]
struct EngineHistoryWorker(HistoryWorker);

fn is_coalescible(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Position | Kind::Levels | Kind::Updated | Kind::Status
    )
}

/// Cloneable engine cmd/evt buses for hosts that only publish (controller).
#[derive(Clone)]
#[flutter_rust_bridge::frb(opaque)]
pub struct EngineBusHandle {
    buses: EngineBuses,
}

impl EngineBusHandle {
    /// Wrap an existing bus pair (tests / `EngineTransport::buses`).
    #[flutter_rust_bridge::frb(ignore)]
    pub fn from_buses(buses: EngineBuses) -> Self {
        Self { buses }
    }

    pub(crate) fn buses(&self) -> EngineBuses {
        self.buses.clone()
    }
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
    library_cmd_bus: library::LibraryBus,
    /// Ephemeral pad chrome (Tauri `AppState.sampler_slots`).
    sampler_slots: Arc<Mutex<Vec<Vec<SamplerSlotChrome>>>>,
    evt_forwarder: Mutex<Option<EngineEvtForwarder>>,
    history_worker: Mutex<Option<EngineHistoryWorker>>,
    /// Bumped at the start of each deck load; stale prepares skip `load_prepared`.
    ///
    /// Shared with a background StemgenRT model download so a track replaced
    /// while the model is downloading never starts a stale producer.
    deck_load_generation: Arc<[AtomicU64; NUM_DECKS]>,
    /// Mixar-owned model root, for opportunistic live separation.
    models_root: std::path::PathBuf,
    /// Live producers per deck; replacing or dropping one stops it.
    live_separators: Arc<Mutex<[Option<analyzer_stems::LiveSeparator>; NUM_DECKS]>>,
    /// Serializes first-use StemgenRT downloads so two deck loads cannot race
    /// on the same `.part` file.
    stemgen_download_lock: Arc<Mutex<()>>,
    /// The null backend is used by headless tests and offline tools; keep the
    /// live filter (and its model download) out of that path.
    live_stems_enabled: bool,
}

impl Drop for EngineTransport {
    fn drop(&mut self) {
        let mut history = self
            .history_worker
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        history.take();
        let mut slot = self.evt_forwarder.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(fwd) = slot.take() {
            fwd.shutdown.store(true, Ordering::Relaxed);
            let _ = fwd.handle.join();
        }
        if let Ok(mut live) = self.live_separators.lock() {
            for slot in live.iter_mut() {
                slot.take();
            }
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
        let live_stems_enabled = config.backend != "null";
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
        let engine_worker = match spawn_engine_worker(Arc::clone(&engine)) {
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
        let history_worker = match HistoryWorker::start(&buses, library_arc.clone()) {
            Ok(worker) => EngineHistoryWorker(worker),
            Err(e) => {
                // Tear down the already-running engine worker before failing start.
                drop(engine_worker);
                if let Ok(mut guard) = engine.lock() {
                    if let Some(eng) = guard.as_mut() {
                        let _ = eng.stop();
                    }
                }
                return Err(format!("history worker failed to start: {e}"));
            }
        };
        let transport = Self {
            worker: engine_worker,
            engine,
            buses: buses.clone(),
            library: library_arc,
            library_cmd_bus: library_transport.cmd_bus(),
            sampler_slots: Arc::new(Mutex::new(empty_all_sampler_chrome())),
            evt_forwarder: Mutex::new(None),
            history_worker: Mutex::new(Some(history_worker)),
            deck_load_generation: Arc::new(std::array::from_fn(|_| AtomicU64::new(0))),
            models_root: library_transport.library_buses().models_root(),
            live_separators: Arc::new(Mutex::new(std::array::from_fn(|_| None))),
            stemgen_download_lock: Arc::new(Mutex::new(())),
            live_stems_enabled,
        };
        if let Ok((target, top, outer)) = settings_host_runtime() {
            let _ = transport.apply_host_settings(target, top, outer);
        }

        Ok(transport)
    }

    /// Stop, rebuild, and start the engine with a new config (settings save).
    #[flutter_rust_bridge::frb(ignore)]
    pub fn restart(&self, config: EngineConfig) -> Result<(), String> {
        if let Ok(mut live) = self.live_separators.lock() {
            for slot in live.iter_mut() {
                slot.take();
            }
        }
        let mut guard = self
            .engine
            .lock()
            .map_err(|_| "engine lock poisoned".to_string())?;
        if let Some(engine) = guard.as_mut() {
            engine.stop().map_err(|e| e.to_string())?;
        }
        match self.build_started_engine(config) {
            Ok(engine) => {
                *guard = Some(engine);
                Ok(())
            }
            Err(err) => {
                if let Some(engine) = guard.as_mut() {
                    if let Err(rollback) = engine.start() {
                        return Err(format!("{err}; rollback failed: {rollback}"));
                    }
                }
                Err(err)
            }
        }
    }

    fn build_started_engine(&self, config: EngineConfig) -> Result<Engine, String> {
        let mut engine = Engine::new_with_library_bus(
            config,
            Arc::clone(&self.library),
            self.library_cmd_bus.clone(),
        )
        .map_err(|e| e.to_string())?;
        engine.set_buses(self.buses.clone());
        engine.start().map_err(|e| e.to_string())?;
        Ok(engine)
    }

    /// Apply normalizer + jog defaults after start/restart.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn apply_host_settings(
        &self,
        normalizer_target: Option<f32>,
        top_jog: engine_api::JogMode,
        outer_jog: engine_api::JogMode,
    ) -> Result<(), String> {
        let mut guard = self
            .engine
            .lock()
            .map_err(|_| "engine lock poisoned".to_string())?;
        let engine = guard
            .as_mut()
            .ok_or_else(|| "engine not available".to_string())?;
        engine
            .set_normalizer_target(normalizer_target)
            .map_err(|e| e.to_string())?;
        for deck_id in 0..2 {
            engine
                .set_deck_jog_mode(deck_id, top_jog, outer_jog)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Restart using the current settings host config + runtime normalizer/jog defaults.
    pub fn restart_from_settings(&self) -> Result<(), String> {
        let config = settings_engine_config()?;
        self.restart(config)?;
        let (target, top, outer) = settings_host_runtime()?;
        self.apply_host_settings(target, top, outer)
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

    /// Seek a deck to `position_ms` (cmd bus).
    pub fn seek(&self, deck_id: u16, position_ms: i32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::Seek,
            &CmdBody::Seek { position_ms },
        )
    }

    /// Channel fader `0..1`.
    pub fn set_volume(&self, deck_id: u16, volume: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetVolume,
            &CmdBody::SetVolume {
                volume,
                soft_takeover: false,
            },
        )
    }

    /// Single EQ band as `0..1` (center `0.5` = 0 dB).
    pub fn set_eq_band(&self, deck_id: u16, band: EqBand, gain: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetEqBand,
            &CmdBody::SetEqBand {
                band: band.into(),
                gain,
                soft_takeover: false,
            },
        )
    }

    /// Filter knob `0..1`.
    pub fn set_filter(&self, deck_id: u16, filter: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetFilter,
            &CmdBody::SetFilter {
                filter,
                soft_takeover: false,
            },
        )
    }

    /// FX unit slot `0..=2` effect selection; `off` clears the slot.
    pub fn set_fx_unit_slot(&self, slot: u8, kind: FxKind) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetFxUnitSlot,
            &CmdBody::SetFxUnitSlot {
                slot,
                kind: kind.into(),
            },
        )
    }

    /// FX unit slot on/off (wet/dry and knobs are kept).
    pub fn set_fx_unit_slot_enabled(&self, slot: u8, enabled: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetFxUnitSlotEnabled,
            &CmdBody::SetFxUnitSlotEnabled { slot, enabled },
        )
    }

    /// FX unit slot wet/dry `0..1`.
    pub fn set_fx_unit_wet(&self, slot: u8, wet: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetFxUnitWet,
            &CmdBody::SetFxUnitWet {
                slot,
                wet,
                soft_takeover: false,
            },
        )
    }

    /// FX unit slot knob `param` `0..=2`, normalized `0..1`.
    pub fn set_fx_unit_param(&self, slot: u8, param: u8, value: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetFxUnitParam,
            &CmdBody::SetFxUnitParam { slot, param, value },
        )
    }

    /// Route deck `deck_id` through the shared FX unit.
    pub fn set_deck_fx_routed(&self, deck_id: u16, routed: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetDeckFxRouted,
            &CmdBody::SetDeckFxRouted { routed },
        )
    }

    /// Gain trim knob `0..1`.
    pub fn set_gain_trim(&self, deck_id: u16, gain_trim: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetGainTrim,
            &CmdBody::SetGainTrim {
                gain_trim,
                soft_takeover: false,
            },
        )
    }

    /// Per-deck headphone cue (PFL).
    pub fn set_headphone_cue(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetHeadphoneCue,
            &CmdBody::SetHeadphoneCue { enabled },
        )
    }

    /// Crossfader `0..1` (A … B).
    pub fn set_crossfader(&self, position: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetCrossfader,
            &CmdBody::SetCrossfader {
                position,
                soft_takeover: false,
            },
        )
    }

    /// Master bus volume `0..1`.
    pub fn set_master_volume(&self, volume: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetMasterVolume,
            &CmdBody::SetMasterVolume {
                volume,
                soft_takeover: false,
            },
        )
    }

    /// Cue/headphone bus volume `0..1`.
    pub fn set_cue_volume(&self, volume: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetCueVolume,
            &CmdBody::SetCueVolume {
                volume,
                soft_takeover: false,
            },
        )
    }

    /// Cue/master headphone mix `0..1`.
    pub fn set_cue_mix(&self, mix: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetCueMix,
            &CmdBody::SetCueMix {
                mix,
                soft_takeover: false,
            },
        )
    }

    /// Master cue (headphones hear master).
    pub fn set_master_cue(&self, enabled: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Mixer,
            Kind::SetMasterCue,
            &CmdBody::SetMasterCue { enabled },
        )
    }

    /// Tempo fader position `0..1`.
    pub fn set_speed(&self, deck_id: u16, speed: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetSpeed,
            &CmdBody::SetSpeed {
                speed,
                soft_takeover: false,
            },
        )
    }

    /// Tempo fader half-span as pitch fraction (`0.06` = ±6%).
    pub fn set_tempo_range(&self, deck_id: u16, tempo_range: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetTempoRange,
            &CmdBody::SetTempoRange { tempo_range },
        )
    }

    /// Key lock / master tempo (time-stretch; pitch held).
    pub fn set_key_lock(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetKeyLock,
            &CmdBody::SetKeyLock { enabled },
        )
    }

    /// Session key-shift offset in semitones (`-16..=16`; `0` = bypass).
    pub fn set_key_shift(&self, deck_id: u16, semitones: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetKeyShift,
            &CmdBody::SetKeyShift { semitones },
        )
    }

    /// Select the hot-cue slot used as the Keyboard pad root (0..=15).
    pub fn set_keyboard_root(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetKeyboardRoot,
            &CmdBody::SetKeyboardRoot { slot },
        )
    }

    /// Select the Keyboard pad semitone page (1..=4).
    pub fn set_keyboard_page(&self, deck_id: u16, page: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetKeyboardPage,
            &CmdBody::SetKeyboardPage { page },
        )
    }

    /// Select the Key Shift pad semitone page (1..=5).
    pub fn set_key_shift_page(&self, deck_id: u16, page: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetKeyShiftPage,
            &CmdBody::SetKeyShiftPage { page },
        )
    }

    pub fn jog_touch(&self, deck_id: u16, touching: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::JogTouch,
            &CmdBody::JogTouch { touching },
        )
    }

    pub fn jog_turn(&self, deck_id: u16, delta: i32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::JogTurn,
            &CmdBody::JogTurn { delta },
        )
    }

    pub fn set_jog_mode(
        &self,
        deck_id: u16,
        top: JogModeSetting,
        outer: JogModeSetting,
    ) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetJogMode,
            &CmdBody::SetJogMode {
                top: top.into(),
                outer: outer.into(),
            },
        )
    }

    pub fn set_cue_point(&self, deck_id: u16) -> Result<(), String> {
        self.publish_empty(Origin::Deck(deck_id), Kind::SetCuePoint)
    }

    pub fn begin_cue_hold(&self, deck_id: u16) -> Result<(), String> {
        self.publish_empty(Origin::Deck(deck_id), Kind::BeginCueHold)
    }

    pub fn end_cue_hold(&self, deck_id: u16) -> Result<(), String> {
        self.publish_empty(Origin::Deck(deck_id), Kind::EndCueHold)
    }

    pub fn set_quantize(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetQuantize,
            &CmdBody::SetQuantize { enabled },
        )
    }

    pub fn set_slip(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetSlip,
            &CmdBody::SetSlip { enabled },
        )
    }

    pub fn unload(&self, deck_id: u16) -> Result<(), String> {
        // Drop the producer + its ring before the deck unload closes coverage.
        if let Ok(mut live) = self.live_separators.lock() {
            if let Some(previous) = live[usize::from(deck_id)].take() {
                previous.stop();
            }
        }
        self.publish_empty(Origin::Deck(deck_id), Kind::Unload)
    }

    pub fn set_auto_loop(&self, deck_id: u16, beats: f32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetAutoLoop,
            &CmdBody::SetAutoLoop { beats },
        )
    }

    pub fn loop_in(&self, deck_id: u16, position_ms: i32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::LoopIn,
            &CmdBody::LoopIn { position_ms },
        )
    }

    pub fn loop_out(&self, deck_id: u16, position_ms: i32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::LoopOut,
            &CmdBody::LoopOut { position_ms },
        )
    }

    pub fn exit_loop(&self, deck_id: u16) -> Result<(), String> {
        self.publish_empty(Origin::Deck(deck_id), Kind::ExitLoop)
    }

    pub fn recall_saved_loop(&self, deck_id: u16, in_ms: i32, out_ms: i32) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::RecallSavedLoop,
            &CmdBody::RecallSavedLoop { in_ms, out_ms },
        )
    }

    pub fn toggle_sync(&self, deck_id: u16, beat_sync: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::ToggleSync,
            &CmdBody::ToggleSync { beat_sync },
        )
    }

    pub fn set_master_deck(&self, deck_id: u16) -> Result<(), String> {
        self.publish_empty(Origin::Deck(deck_id), Kind::SetMasterDeck)
    }

    /// Per-deck pad mode.
    pub fn set_pad_mode(&self, deck_id: u16, mode: PadMode) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SetPadMode,
            &CmdBody::SetPadMode { mode: mode.into() },
        )
    }

    /// Stems / generic pad press (engine dispatches by current `pad_mode`).
    pub fn pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::PadPress,
            &CmdBody::PadPress { slot, shift },
        )
    }

    pub fn pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::PadRelease,
            &CmdBody::PadRelease { slot },
        )
    }

    pub fn hot_cue_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::HotCuePadPress,
            &CmdBody::HotCuePadPress { slot, shift },
        )
    }

    pub fn hot_cue_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::HotCuePadRelease,
            &CmdBody::HotCuePadRelease { slot },
        )
    }

    pub fn loop_roll_pad_press(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::LoopRollPadPress,
            &CmdBody::LoopRollPadPress { slot },
        )
    }

    pub fn loop_roll_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::LoopRollPadRelease,
            &CmdBody::LoopRollPadRelease { slot },
        )
    }

    pub fn beat_jump_pad_press(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::BeatJumpPadPress,
            &CmdBody::BeatJumpPadPress { slot },
        )
    }

    pub fn beat_jump_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::BeatJumpPadRelease,
            &CmdBody::BeatJumpPadRelease { slot },
        )
    }

    /// Keyboard pad press; `shift` plays the upper octave / accent.
    pub fn keyboard_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::KeyboardPadPress,
            &CmdBody::KeyboardPadPress { slot, shift },
        )
    }

    pub fn keyboard_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::KeyboardPadRelease,
            &CmdBody::KeyboardPadRelease { slot },
        )
    }

    /// Key Shift pad press; `shift` resets the deck to `0.0` semitones.
    pub fn key_shift_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::KeyShiftPadPress,
            &CmdBody::KeyShiftPadPress { slot, shift },
        )
    }

    pub fn key_shift_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::KeyShiftPadRelease,
            &CmdBody::KeyShiftPadRelease { slot },
        )
    }

    pub fn sampler_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SamplerPadPress,
            &CmdBody::SamplerPadPress { slot, shift },
        )
    }

    pub fn sampler_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_body(
            Origin::Deck(deck_id),
            Kind::SamplerPadRelease,
            &CmdBody::SamplerPadRelease { slot },
        )
    }

    pub fn assign_sampler(&self, deck_id: u16, slot: u8, path: String) -> Result<(), String> {
        let prepared =
            LibraryManager::prepare_file_path_for_playback(self.library.as_ref(), Path::new(&path))
                .map_err(|e| e.to_string())?;
        self.assign_prepared(deck_id, slot, prepared)
    }

    pub fn assign_sampler_track(
        &self,
        deck_id: u16,
        slot: u8,
        track_id: String,
    ) -> Result<(), String> {
        let prepared = LibraryManager::prepare_track_for_playback(
            self.library.as_ref(),
            &TrackId::new(track_id),
        )
        .map_err(|e| e.to_string())?;
        self.assign_prepared(deck_id, slot, prepared)
    }

    pub fn clear_sampler(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        let deck = usize::from(deck_id);
        let slot_i = usize::from(slot);
        if deck >= NUM_DECKS || slot_i >= SAMPLER_SLOT_COUNT {
            return Err(format!("Invalid sampler deck/slot: {deck_id}/{slot}"));
        }
        let snap = {
            let mut guard = self
                .engine
                .lock()
                .map_err(|_| "engine lock poisoned".to_string())?;
            let engine = guard
                .as_mut()
                .ok_or_else(|| "engine not available".to_string())?;
            engine
                .clear_sampler_slot(deck, slot_i)
                .map_err(|e| e.to_string())?;
            engine
                .deck_snapshot(deck)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?
        };
        {
            let mut chrome = self
                .sampler_slots
                .lock()
                .map_err(|_| "sampler chrome lock poisoned".to_string())?;
            chrome[deck][slot_i] = SamplerSlotChrome::default();
        }
        self.publish_deck_updated(deck_id, snap)
    }

    /// Load a sampler bank's slots onto a deck (prepare outside the engine lock).
    pub fn set_sampler_bank(&self, deck_id: u16, bank_id: String) -> Result<(), String> {
        let deck = usize::from(deck_id);
        if deck >= NUM_DECKS {
            return Err(format!("Invalid deck ID: {deck_id}"));
        }
        let slots = {
            let lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            if lib
                .get_sampler_bank(&bank_id)
                .map_err(|e| e.to_string())?
                .is_none()
            {
                return Err(format!("Sampler bank not found: {bank_id}"));
            }
            lib.list_sampler_bank_slots(&bank_id)
                .map_err(|e| e.to_string())?
        };
        let mut prepared: Vec<(u8, PreparedTrackPlayback, SamplerSlotChrome)> = Vec::new();
        for record in slots {
            if usize::from(record.slot_index) >= SAMPLER_SLOT_COUNT {
                continue;
            }
            let item = if let Some(track_id) = record.track_id.as_ref() {
                LibraryManager::prepare_track_for_playback(
                    self.library.as_ref(),
                    &TrackId::new(track_id.clone()),
                )
            } else if let Some(path) = record.path.as_ref() {
                LibraryManager::prepare_file_path_for_playback(
                    self.library.as_ref(),
                    Path::new(path),
                )
            } else {
                continue;
            }
            .map_err(|e| e.to_string())?;
            let chrome = chrome_from_bank_slot(&record, &item);
            prepared.push((record.slot_index, item, chrome));
        }
        let mut deck_chrome = empty_deck_sampler_chrome();
        let snap = {
            let mut guard = self
                .engine
                .lock()
                .map_err(|_| "engine lock poisoned".to_string())?;
            let engine = guard
                .as_mut()
                .ok_or_else(|| "engine not available".to_string())?;
            engine
                .clear_all_sampler_slots(deck)
                .map_err(|e| e.to_string())?;
            for (slot, item, chrome) in prepared {
                engine
                    .assign_prepared_sampler(deck, usize::from(slot), item)
                    .map_err(|e| e.to_string())?;
                deck_chrome[usize::from(slot)] = chrome;
            }
            engine
                .set_deck_sampler_bank(deck, Some(bank_id))
                .map_err(|e| e.to_string())?;
            engine
                .deck_snapshot(deck)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?
        };
        {
            let mut chrome = self
                .sampler_slots
                .lock()
                .map_err(|_| "sampler chrome lock poisoned".to_string())?;
            chrome[deck] = deck_chrome;
        }
        self.publish_deck_updated(deck_id, snap)
    }

    /// Rename / set play mode for a stored sampler bank.
    pub fn update_sampler_bank(
        &self,
        bank_id: String,
        name: String,
        play_mode: Option<crate::api::library::SamplerPlayMode>,
    ) -> Result<(), String> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("Bank name cannot be empty.".to_string());
        }
        let play_mode = play_mode.map(|m| match m {
            crate::api::library::SamplerPlayMode::Oneshot => library::SamplerPlayMode::Oneshot,
            crate::api::library::SamplerPlayMode::Hold => library::SamplerPlayMode::Hold,
            crate::api::library::SamplerPlayMode::Loop => library::SamplerPlayMode::Loop,
        });
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        lib.update_sampler_bank(&bank_id, &name, play_mode)
            .map_err(|e| e.to_string())
    }

    /// Load a library track: prepare outside the engine lock, then `load_prepared_track`.
    pub fn load_library_track(&self, deck_id: u16, track_id: String) -> Result<(), String> {
        let generation = self.begin_deck_load(deck_id)?;
        let prepared = LibraryManager::prepare_track_for_playback(
            self.library.as_ref(),
            &TrackId::new(track_id),
        )
        .map_err(|e| e.to_string())?;
        self.load_prepared(deck_id, prepared, generation)
    }

    /// Load a filesystem path: prepare outside the engine lock, then `load_prepared_track`.
    pub fn load_path(&self, deck_id: u16, path: String) -> Result<(), String> {
        let generation = self.begin_deck_load(deck_id)?;
        let prepared =
            LibraryManager::prepare_file_path_for_playback(self.library.as_ref(), Path::new(&path))
                .map_err(|e| e.to_string())?;
        self.load_prepared(deck_id, prepared, generation)
    }

    fn begin_deck_load(&self, deck_id: u16) -> Result<u64, String> {
        let idx = usize::from(deck_id);
        if idx >= NUM_DECKS {
            return Err(format!("invalid deck id: {deck_id}"));
        }
        Ok(self.deck_load_generation[idx].fetch_add(1, Ordering::AcqRel) + 1)
    }

    fn is_current_deck_load(&self, deck_id: u16, generation: u64) -> bool {
        let idx = usize::from(deck_id);
        idx < NUM_DECKS && self.deck_load_generation[idx].load(Ordering::Acquire) == generation
    }

    /// Opportunistically separate the loaded track on the fly when no cache
    /// exists. The StemgenRT graph is a causal 128-sample-hop filter: the deck
    /// mixes its four stems before the channel gain/EQ strip, and keeps playing
    /// the dry mix until the producer publishes coverage.
    ///
    /// The model is downloaded in the background on first use. The deck is
    /// attached to an empty ring immediately, so this call never blocks on the
    /// network; `stems_ready` flips when the producer publishes its first hop.
    fn start_live_stems(
        &self,
        deck: usize,
        audio: Arc<audio_core::LoadedAudio>,
        engine: &mut Engine,
        generation: u64,
    ) {
        if !self.live_stems_enabled
            || !settings_realtime_stems_enabled()
            || audio.channels != 2
            || audio.sample_rate == 0
        {
            return;
        }
        let ring = Arc::new(audio_core::LiveStemRing::new(
            analyzer_stems::STEM_SAMPLE_RATE,
            audio_core::DEFAULT_LIVE_STEM_CAPACITY,
        ));
        if let Err(err) = engine.attach_live_deck_stems(deck, Arc::clone(&ring)) {
            tracing::warn!(deck, error = %err, "live stems: attach failed");
            return;
        }

        let cached = analyzer_stems::cached_model(&self.models_root, analyzer_stems::STEMGEN_MODEL)
            .ok()
            .flatten();
        if let Some(handle) = cached {
            match analyzer_stems::start_stemgen_live_separator(handle.local_path, audio, ring) {
                Ok(separator) => {
                    if let Ok(mut live) = self.live_separators.lock() {
                        if let Some(previous) = live[deck].replace(separator) {
                            previous.stop();
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(deck, error = %err, "StemgenRT live: producer failed to start")
                }
            }
            return;
        }

        // First use: fetch the 32 MB model off the audio/load path, then start
        // the producer only if this deck still owns the load generation.
        let models_root = self.models_root.clone();
        let live = Arc::clone(&self.live_separators);
        let generations = Arc::clone(&self.deck_load_generation);
        let download_lock = Arc::clone(&self.stemgen_download_lock);
        let thread_audio = Arc::clone(&audio);
        let thread_ring = Arc::clone(&ring);
        let spawned = std::thread::Builder::new()
            .name("stemgen-model".into())
            .spawn(move || {
                let _download_guard = match download_lock.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                if let Err(err) =
                    analyzer_stems::ensure_model(&models_root, analyzer_stems::STEMGEN_MODEL)
                {
                    tracing::warn!(deck, error = %err, "StemgenRT live: model download failed");
                    return;
                }
                if generations[deck].load(Ordering::Acquire) != generation
                    || !settings_realtime_stems_enabled()
                {
                    return;
                }
                let Ok(Some(handle)) =
                    analyzer_stems::cached_model(&models_root, analyzer_stems::STEMGEN_MODEL)
                else {
                    return;
                };
                match analyzer_stems::start_stemgen_live_separator(
                    handle.local_path,
                    thread_audio,
                    thread_ring,
                ) {
                    Ok(separator) => {
                        // The deck may have been replaced while the producer
                        // thread was starting. Never install a stale epoch.
                        if generations[deck].load(Ordering::Acquire) != generation
                            || !settings_realtime_stems_enabled()
                        {
                            separator.stop();
                            return;
                        }
                        if let Ok(mut live) = live.lock() {
                            if let Some(previous) = live[deck].replace(separator) {
                                previous.stop();
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!(deck, error = %err, "StemgenRT live: producer failed to start")
                    }
                }
            });
        if let Err(err) = spawned {
            tracing::warn!(deck, error = %err, "StemgenRT live: model worker failed to spawn");
        }
    }

    fn load_prepared(
        &self,
        deck_id: u16,
        mut prepared: PreparedTrackPlayback,
        generation: u64,
    ) -> Result<(), String> {
        let stems = prepared.stems.take();
        // No cached stems: attach the live StemgenRT filter. It is independent
        // of offline stem generation and runs on the CPU, so it also works on
        // machines without a GPU EP.
        let live_audio = stems.is_none().then(|| Arc::clone(&prepared.audio));
        let snap = {
            let mut guard = self
                .engine
                .lock()
                .map_err(|_| "engine lock poisoned".to_string())?;
            if !self.is_current_deck_load(deck_id, generation) {
                return Ok(());
            }
            let engine = guard
                .as_mut()
                .ok_or_else(|| "engine not available".to_string())?;
            // A live producer from the previous track must stop before either
            // the cached-stems arm or a new live producer replaces it.
            if let Ok(mut live) = self.live_separators.lock() {
                if let Some(previous) = live[deck_id as usize].take() {
                    previous.stop();
                }
            }
            engine
                .load_prepared_track(deck_id as usize, prepared)
                .map_err(|e| e.to_string())?;
            if let Some(stems) = stems {
                engine
                    .attach_deck_stems(deck_id as usize, stems)
                    .map_err(|e| e.to_string())?;
            } else if let Some(audio) = live_audio {
                self.start_live_stems(deck_id as usize, audio, engine, generation);
            }
            engine
                .deck_snapshot(deck_id as usize)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?
        };
        self.publish_deck_updated(deck_id, snap)
    }

    fn assign_prepared(
        &self,
        deck_id: u16,
        slot: u8,
        prepared: PreparedTrackPlayback,
    ) -> Result<(), String> {
        let deck = usize::from(deck_id);
        let slot_i = usize::from(slot);
        if deck >= NUM_DECKS || slot_i >= SAMPLER_SLOT_COUNT {
            return Err(format!("Invalid sampler deck/slot: {deck_id}/{slot}"));
        }
        let chrome = chrome_from_prepared(&prepared);
        let snap = {
            let mut guard = self
                .engine
                .lock()
                .map_err(|_| "engine lock poisoned".to_string())?;
            let engine = guard
                .as_mut()
                .ok_or_else(|| "engine not available".to_string())?;
            engine
                .assign_prepared_sampler(deck, slot_i, prepared)
                .map_err(|e| e.to_string())?;
            engine
                .deck_snapshot(deck)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?
        };
        {
            let mut slots = self
                .sampler_slots
                .lock()
                .map_err(|_| "sampler chrome lock poisoned".to_string())?;
            slots[deck][slot_i] = chrome;
        }
        self.publish_deck_updated(deck_id, snap)
    }

    fn publish_deck_updated(&self, deck_id: u16, snap: DeckSnapshot) -> Result<(), String> {
        self.buses
            .publish_evt(
                Origin::Deck(deck_id),
                Kind::Updated,
                deck_snapshot_to_evt(snap),
            )
            .map_err(|e| e.to_string())
    }

    fn publish_empty(&self, origin: Origin, kind: Kind) -> Result<(), String> {
        self.publish_body(origin, kind, &CmdBody::Empty)
    }

    fn publish_body(&self, origin: Origin, kind: Kind, body: &CmdBody) -> Result<(), String> {
        let bytes = encode_cmd_body(body).map_err(|e| e.to_string())?;
        self.buses
            .publish_cmd(origin, kind, bytes)
            .map_err(|e| e.to_string())
    }

    fn publish_current_status(&self) {
        let status = self
            .engine
            .lock()
            .ok()
            .and_then(|g| g.as_ref().and_then(Engine::engine_status_snapshot));
        if let Some(status) = status {
            let _ = self.buses.publish_evt(
                Origin::Mixer,
                Kind::Status,
                EvtBody::EngineStatus { status },
            );
        }
    }

    /// Forward thin typed engine events to Dart via FRB `StreamSink`.
    ///
    /// Coalesces Position/Levels/Updated/Status (latest wins) like Tauri.
    /// Replaces any previous forwarder so repeated subscribe calls do not leak threads.
    pub fn subscribe_events(&self, sink: StreamSink<EngineEvt>) -> Result<(), String> {
        let rx = self.buses.subscribe_evt_all().map_err(|e| e.to_string())?;
        let chrome = Arc::clone(&self.sampler_slots);
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
                    let chrome_snap = chrome.lock().map(|g| g.clone()).unwrap_or_default();
                    for ev in discrete.into_iter().chain(coalesced.into_values()) {
                        for mut mapped in map_engine_evts(ev.as_ref()) {
                            attach_sampler_chrome(&mut mapped, &chrome_snap);
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
        self.publish_current_status();
        Ok(())
    }

    /// Raw evt subscription for host/tests. Prefer [`Self::subscribe_events`] for Dart.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn subscribe_evt_all(&self) -> Result<engine_core::EvtReceiver, String> {
        self.buses.subscribe_evt_all().map_err(|e| e.to_string())
    }

    /// Clone of the engine cmd/evt buses for [`crate::api::controller::ControllerTransport`].
    pub fn buses(&self) -> EngineBusHandle {
        EngineBusHandle::from_buses(self.buses.clone())
    }
}

fn deck_id_of(origin: &Origin) -> Option<u16> {
    match origin {
        Origin::Deck(id) => Some(*id),
        Origin::Engine | Origin::Mixer => None,
    }
}

fn updated_from_snapshot(snap: &DeckSnapshot) -> EngineEvt {
    let mut evt = EngineEvt::bare(EngineEvtKind::Updated);
    evt.deck_id = Some(snap.id);
    evt.playing = Some(snap.playing);
    evt.track_path = snap.track.clone();
    evt.track_id = snap.track_id.clone();
    evt.position_ms = snap.position_ms;
    evt.volume = Some(snap.volume);
    evt.eq_low = Some(snap.eq.low);
    evt.eq_mid = Some(snap.eq.mid);
    evt.eq_high = Some(snap.eq.high);
    evt.filter = Some(snap.filter);
    evt.gain_trim = Some(snap.gain_trim);
    evt.fx = Some(snap.fx.iter().map(fx_slot_chrome).collect());
    evt.fx_routed = Some(snap.fx_routed);
    evt.headphone_cue = Some(snap.headphone_cue);
    evt.duration_ms = snap.duration_ms;
    evt.duration_known = true;
    evt.speed = Some(snap.speed);
    evt.tempo_range = Some(snap.tempo_range);
    evt.key_lock = Some(snap.key_lock);
    evt.key_shift = Some(snap.key_shift);
    evt.keyboard_page = Some(snap.keyboard_page);
    evt.key_shift_page = Some(snap.key_shift_page);
    evt.keyboard_root_hot_cue = Some(snap.keyboard_root_hot_cue);
    evt.pad_mode = Some(snap.pad_mode.into());
    evt.sync_mode = Some(snap.sync_mode);
    evt.active_loop = snap.active_loop.clone().map(ActiveLoopInfo::from);
    evt.active_loop_known = true;
    evt.pending_loop_in_ms = snap.pending_loop_in_ms;
    evt.pending_loop_in_ms_known = true;
    evt.quantize = Some(snap.quantize);
    evt.slip_enabled = Some(snap.slip_enabled);
    evt.slip_shadow_position_ms = snap.slip_shadow_position_ms;
    evt.jog_touching = Some(snap.jog_touching);
    evt.loudness_lufs = snap.loudness_lufs;
    evt.auto_gain_db = Some(snap.auto_gain_db);
    evt.active_sampler_bank_id = snap.active_sampler_bank_id.clone();
    evt.active_sampler_bank_id_known = true;
    evt.stems_ready = Some(snap.stems_ready);
    evt.stems_live = Some(snap.stems_live);
    evt.stem_mute = Some(snap.stem_mute.to_vec());
    evt.stem_isolate = snap.stem_isolate;
    evt
}

pub(crate) fn map_engine_evts(ev: &Evt) -> Vec<EngineEvt> {
    let Ok(body) = decode_evt_body(ev.payload()) else {
        return Vec::new();
    };
    let deck_id = deck_id_of(ev.origin());
    match body {
        EvtBody::EngineStatus { status } => {
            let mut status_evt = EngineEvt::bare(EngineEvtKind::Status);
            status_evt.running = Some(status.running);
            status_evt.crossfader = Some(status.crossfader);
            status_evt.master_volume = Some(status.master_volume);
            status_evt.cue_volume = Some(status.cue_volume);
            status_evt.cue_mix = Some(status.cue_mix);
            status_evt.master_cue = Some(status.master_cue);
            status_evt.master_deck = Some(status.master_deck);
            let mut out = Vec::with_capacity(1 + status.decks.len());
            out.push(status_evt);
            out.extend(status.decks.iter().map(updated_from_snapshot));
            out
        }
        EvtBody::DeckUpdated {
            id,
            playing,
            track,
            track_id,
            position_ms,
            volume,
            eq,
            filter,
            gain_trim,
            fx,
            fx_routed,
            headphone_cue,
            duration_ms,
            speed,
            tempo_range,
            key_lock,
            key_shift,
            keyboard_page,
            key_shift_page,
            keyboard_root_hot_cue,
            pad_mode,
            sync_mode,
            active_loop,
            pending_loop_in_ms,
            quantize,
            slip_enabled,
            slip_shadow_position_ms,
            jog_touching,
            loudness_lufs,
            auto_gain_db,
            active_sampler_bank_id,
            stems_ready,
            stems_live,
            stem_mute,
            stem_isolate,
            ..
        } => {
            let mut evt = EngineEvt::bare(EngineEvtKind::Updated);
            evt.deck_id = deck_id.or(Some(id));
            evt.playing = Some(playing);
            evt.track_path = track;
            evt.track_id = track_id;
            evt.position_ms = position_ms;
            evt.volume = Some(volume);
            evt.eq_low = Some(eq.low);
            evt.eq_mid = Some(eq.mid);
            evt.eq_high = Some(eq.high);
            evt.filter = Some(filter);
            evt.gain_trim = Some(gain_trim);
            evt.fx = Some(fx.iter().map(fx_slot_chrome).collect());
            evt.fx_routed = Some(fx_routed);
            evt.headphone_cue = Some(headphone_cue);
            evt.duration_ms = duration_ms;
            evt.duration_known = true;
            evt.speed = Some(speed);
            evt.tempo_range = Some(tempo_range);
            evt.key_lock = Some(key_lock);
            evt.key_shift = Some(key_shift);
            evt.keyboard_page = Some(keyboard_page);
            evt.key_shift_page = Some(key_shift_page);
            evt.keyboard_root_hot_cue = Some(keyboard_root_hot_cue);
            evt.pad_mode = Some(pad_mode.into());
            evt.sync_mode = Some(sync_mode);
            evt.active_loop = active_loop.map(ActiveLoopInfo::from);
            evt.active_loop_known = true;
            evt.pending_loop_in_ms = pending_loop_in_ms;
            evt.pending_loop_in_ms_known = true;
            evt.quantize = Some(quantize);
            evt.slip_enabled = Some(slip_enabled);
            evt.slip_shadow_position_ms = slip_shadow_position_ms;
            evt.jog_touching = Some(jog_touching);
            evt.loudness_lufs = loudness_lufs;
            evt.auto_gain_db = Some(auto_gain_db);
            evt.active_sampler_bank_id = active_sampler_bank_id;
            evt.active_sampler_bank_id_known = true;
            evt.stems_ready = Some(stems_ready);
            evt.stems_live = Some(stems_live);
            evt.stem_mute = Some(stem_mute.to_vec());
            evt.stem_isolate = stem_isolate;
            vec![evt]
        }
        EvtBody::Position {
            position_ms,
            slip_shadow_position_ms,
        } => {
            let mut evt = EngineEvt::bare(EngineEvtKind::Position);
            evt.deck_id = deck_id;
            evt.position_ms = Some(position_ms);
            evt.slip_shadow_position_ms = slip_shadow_position_ms;
            vec![evt]
        }
        EvtBody::Levels {
            peak_l,
            peak_r,
            peak_hold_l,
            peak_hold_r,
        } => {
            let mut evt = EngineEvt::bare(EngineEvtKind::Levels);
            evt.deck_id = deck_id;
            evt.peak_l = Some(peak_l);
            evt.peak_r = Some(peak_r);
            evt.peak_hold_l = Some(peak_hold_l);
            evt.peak_hold_r = Some(peak_hold_r);
            vec![evt]
        }
        EvtBody::Error { message } => {
            let mut evt = EngineEvt::bare(EngineEvtKind::Error);
            evt.deck_id = deck_id;
            evt.message = Some(message);
            vec![evt]
        }
        EvtBody::Notice { message } => {
            let mut evt = EngineEvt::bare(EngineEvtKind::Notice);
            evt.deck_id = deck_id;
            evt.message = Some(message);
            vec![evt]
        }
        EvtBody::Empty => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_api::{DeckEq, EngineStatus, JogMode, PadMode, SamplerStatus};

    fn recv_mapped(origin: Origin, kind: Kind, body: EvtBody) -> Vec<EngineEvt> {
        let buses = EngineBuses::new();
        let rx = buses.subscribe_evt_all().unwrap();
        buses.publish_evt(origin, kind, body).unwrap();
        let ev = loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(Some(ev)) => break ev,
                Ok(None) => continue,
                Err(_) => panic!("timeout waiting for mapped evt"),
            }
        };
        map_engine_evts(ev.as_ref())
    }

    fn sample_deck(id: u16, volume: f32) -> DeckSnapshot {
        DeckSnapshot {
            id,
            track: None,
            track_id: None,
            title: None,
            artist: None,
            album: None,
            isrc: None,
            bpm: None,
            key: None,
            playing: false,
            volume,
            speed: 0.5,
            tempo_range: 0.08,
            key_lock: false,
            key_shift: 0.0,
            keyboard_page: engine_api::default_pitch_page(),
            key_shift_page: engine_api::default_pitch_page(),
            keyboard_root_hot_cue: 0,
            eq: DeckEq {
                low: 0.5,
                mid: 0.5,
                high: 0.25,
            },
            filter: 0.5,
            gain_trim: 0.5,
            fx_routed: false,
            fx: Default::default(),
            headphone_cue: true,
            sync_mode: SyncMode::Off,
            cue_point_ms: None,
            quantize: true,
            active_loop: None,
            pending_loop_in_ms: None,
            slip_enabled: false,
            slip_shadow_position_ms: None,
            pad_mode: PadMode::HotCue,
            stems_ready: false,
            stems_live: false,
            stem_mute: [false; 4],
            stem_isolate: None,
            position_ms: None,
            duration_ms: None,
            hot_cues: Vec::new(),
            saved_loops: Vec::new(),
            loudness_lufs: None,
            auto_gain_db: 0.0,
            active_sampler_bank_id: None,
            top_jog_mode: JogMode::Vinyl,
            outer_jog_mode: JogMode::PitchBend,
            jog_touching: false,
        }
    }

    #[test]
    fn map_updated_forwards_track_path_not_title() {
        let mut deck = sample_deck(0, 1.0);
        deck.track = Some("/music/Palawan.opus".into());
        deck.title = Some("Palawan".into());
        deck.track_id = Some("lib-1".into());
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Status,
            EvtBody::EngineStatus {
                status: EngineStatus {
                    running: true,
                    sample_rate: 48_000,
                    crossfader: 0.5,
                    master_volume: 1.0,
                    cue_volume: 1.0,
                    cue_mix: 0.5,
                    master_cue: false,
                    master_deck: 0,
                    decks: vec![deck],
                    sampler: SamplerStatus {
                        banks: Vec::new(),
                        active_bank_id: None,
                        active_bank_name: None,
                        bank_play_mode: None,
                        deck_slots: Vec::new(),
                        effective_play_modes: Vec::new(),
                    },
                },
            },
        );
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[1].kind, EngineEvtKind::Updated);
        assert_eq!(mapped[1].track_path.as_deref(), Some("/music/Palawan.opus"));
        assert_eq!(mapped[1].track_id.as_deref(), Some("lib-1"));
    }

    #[test]
    fn map_updated_forwards_stem_state() {
        let mut deck = sample_deck(0, 1.0);
        deck.stems_ready = true;
        deck.stems_live = true;
        deck.stem_mute = [true, false, true, false];
        deck.stem_isolate = Some(1);
        let mapped = recv_mapped(Origin::Deck(0), Kind::Updated, deck_snapshot_to_evt(deck));
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].stems_ready, Some(true));
        assert_eq!(mapped[0].stems_live, Some(true));
        assert_eq!(
            mapped[0].stem_mute.as_deref(),
            Some([true, false, true, false].as_slice())
        );
        assert_eq!(mapped[0].stem_isolate, Some(1));
    }

    #[test]
    fn map_updated_forwards_pages_and_keyboard_root() {
        let mut deck = sample_deck(0, 1.0);
        deck.keyboard_page = 5;
        deck.key_shift_page = 3;
        deck.keyboard_root_hot_cue = 3;
        let mapped = recv_mapped(Origin::Deck(0), Kind::Updated, deck_snapshot_to_evt(deck));
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].keyboard_page, Some(5));
        assert_eq!(mapped[0].key_shift_page, Some(3));
        assert_eq!(mapped[0].keyboard_root_hot_cue, Some(3));
    }

    #[test]
    fn map_levels_includes_peak_hold() {
        let mapped = recv_mapped(
            Origin::Deck(0),
            Kind::Levels,
            EvtBody::Levels {
                peak_l: 0.4,
                peak_r: 0.5,
                peak_hold_l: 0.8,
                peak_hold_r: 0.9,
            },
        );
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].kind, EngineEvtKind::Levels);
        assert_eq!(mapped[0].deck_id, Some(0));
        assert_eq!(mapped[0].peak_l, Some(0.4));
        assert_eq!(mapped[0].peak_r, Some(0.5));
        assert_eq!(mapped[0].peak_hold_l, Some(0.8));
        assert_eq!(mapped[0].peak_hold_r, Some(0.9));
    }

    #[test]
    fn map_mixer_levels_have_null_deck_id() {
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Levels,
            EvtBody::Levels {
                peak_l: 0.3,
                peak_r: 0.35,
                peak_hold_l: 0.4,
                peak_hold_r: 0.45,
            },
        );
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].kind, EngineEvtKind::Levels);
        assert_eq!(mapped[0].deck_id, None);
        assert_eq!(mapped[0].peak_l, Some(0.3));
        assert_eq!(mapped[0].peak_r, Some(0.35));
    }

    #[test]
    fn map_status_fans_out_crossfader_and_decks() {
        let mut deck1 = sample_deck(1, 0.7);
        deck1.sync_mode = SyncMode::Tempo;
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Status,
            EvtBody::EngineStatus {
                status: EngineStatus {
                    running: true,
                    sample_rate: 48_000,
                    crossfader: 0.25,
                    master_volume: 1.0,
                    cue_volume: 1.0,
                    cue_mix: 0.4,
                    master_cue: true,
                    master_deck: 1,
                    decks: vec![sample_deck(0, 0.3), deck1],
                    sampler: SamplerStatus {
                        banks: Vec::new(),
                        active_bank_id: None,
                        active_bank_name: None,
                        bank_play_mode: None,
                        deck_slots: Vec::new(),
                        effective_play_modes: Vec::new(),
                    },
                },
            },
        );
        assert_eq!(mapped.len(), 3);
        assert_eq!(mapped[0].kind, EngineEvtKind::Status);
        assert_eq!(mapped[0].running, Some(true));
        assert_eq!(mapped[0].crossfader, Some(0.25));
        assert_eq!(mapped[0].master_volume, Some(1.0));
        assert_eq!(mapped[0].cue_volume, Some(1.0));
        assert_eq!(mapped[0].cue_mix, Some(0.4));
        assert_eq!(mapped[0].master_cue, Some(true));
        assert_eq!(mapped[0].master_deck, Some(1));
        assert_eq!(mapped[1].kind, EngineEvtKind::Updated);
        assert_eq!(mapped[1].deck_id, Some(0));
        assert_eq!(mapped[1].volume, Some(0.3));
        assert_eq!(mapped[1].eq_high, Some(0.25));
        assert_eq!(mapped[1].headphone_cue, Some(true));
        assert_eq!(mapped[1].speed, Some(0.5));
        assert_eq!(mapped[1].tempo_range, Some(0.08));
        assert_eq!(mapped[1].pad_mode, Some(super::PadMode::HotCue));
        assert_eq!(mapped[1].sync_mode, Some(SyncMode::Off));
        assert_eq!(mapped[1].quantize, Some(true));
        assert!(mapped[1].duration_known);
        assert_eq!(mapped[1].jog_touching, Some(false));
        assert_eq!(mapped[1].loudness_lufs, None);
        assert_eq!(mapped[1].auto_gain_db, Some(0.0));
        assert_eq!(mapped[2].deck_id, Some(1));
        assert_eq!(mapped[2].volume, Some(0.7));
        assert_eq!(mapped[2].sync_mode, Some(SyncMode::Tempo));
    }

    #[test]
    fn map_status_forwards_active_loop() {
        let mut deck = sample_deck(0, 1.0);
        deck.active_loop = Some(engine_api::LoopRegion {
            in_ms: 1000,
            out_ms: 5000,
            active: true,
        });
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Status,
            EvtBody::EngineStatus {
                status: EngineStatus {
                    running: true,
                    sample_rate: 48_000,
                    crossfader: 0.5,
                    master_volume: 1.0,
                    cue_volume: 1.0,
                    cue_mix: 0.0,
                    master_cue: false,
                    master_deck: 0,
                    decks: vec![deck],
                    sampler: SamplerStatus {
                        banks: Vec::new(),
                        active_bank_id: None,
                        active_bank_name: None,
                        bank_play_mode: None,
                        deck_slots: Vec::new(),
                        effective_play_modes: Vec::new(),
                    },
                },
            },
        );
        assert_eq!(mapped.len(), 2);
        assert!(mapped[1].active_loop_known);
        let region = mapped[1].active_loop.as_ref().expect("active_loop");
        assert_eq!(region.in_ms, 1000);
        assert_eq!(region.out_ms, 5000);
        assert!(region.active);
    }

    #[test]
    fn map_status_forwards_pending_loop_in() {
        let mut deck = sample_deck(0, 1.0);
        deck.pending_loop_in_ms = Some(1500);
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Status,
            EvtBody::EngineStatus {
                status: EngineStatus {
                    running: true,
                    sample_rate: 48_000,
                    crossfader: 0.5,
                    master_volume: 1.0,
                    cue_volume: 1.0,
                    cue_mix: 0.0,
                    master_cue: false,
                    master_deck: 0,
                    decks: vec![deck],
                    sampler: SamplerStatus {
                        banks: Vec::new(),
                        active_bank_id: None,
                        active_bank_name: None,
                        bank_play_mode: None,
                        deck_slots: Vec::new(),
                        effective_play_modes: Vec::new(),
                    },
                },
            },
        );
        assert_eq!(mapped.len(), 2);
        assert!(mapped[1].pending_loop_in_ms_known);
        assert_eq!(mapped[1].pending_loop_in_ms, Some(1500));
    }

    #[test]
    fn map_status_forwards_quantize_gain_and_jog() {
        let mut deck = sample_deck(0, 1.0);
        deck.quantize = false;
        deck.jog_touching = true;
        deck.loudness_lufs = Some(-14.5);
        deck.auto_gain_db = -3.5;
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Status,
            EvtBody::EngineStatus {
                status: EngineStatus {
                    running: true,
                    sample_rate: 48_000,
                    crossfader: 0.5,
                    master_volume: 1.0,
                    cue_volume: 1.0,
                    cue_mix: 0.0,
                    master_cue: false,
                    master_deck: 0,
                    decks: vec![deck],
                    sampler: SamplerStatus {
                        banks: Vec::new(),
                        active_bank_id: None,
                        active_bank_name: None,
                        bank_play_mode: None,
                        deck_slots: Vec::new(),
                        effective_play_modes: Vec::new(),
                    },
                },
            },
        );
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[1].quantize, Some(false));
        assert_eq!(mapped[1].jog_touching, Some(true));
        assert_eq!(mapped[1].loudness_lufs, Some(-14.5));
        assert_eq!(mapped[1].auto_gain_db, Some(-3.5));
    }

    #[test]
    fn map_status_forwards_active_sampler_bank_id() {
        let mut deck = sample_deck(0, 1.0);
        deck.active_sampler_bank_id = Some("bank-1".into());
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Status,
            EvtBody::EngineStatus {
                status: EngineStatus {
                    running: true,
                    sample_rate: 48_000,
                    crossfader: 0.5,
                    master_volume: 1.0,
                    cue_volume: 1.0,
                    cue_mix: 0.0,
                    master_cue: false,
                    master_deck: 0,
                    decks: vec![deck],
                    sampler: SamplerStatus {
                        banks: Vec::new(),
                        active_bank_id: None,
                        active_bank_name: None,
                        bank_play_mode: None,
                        deck_slots: Vec::new(),
                        effective_play_modes: Vec::new(),
                    },
                },
            },
        );
        assert_eq!(mapped.len(), 2);
        assert!(mapped[1].active_sampler_bank_id_known);
        assert_eq!(mapped[1].active_sampler_bank_id.as_deref(), Some("bank-1"));
    }

    #[test]
    fn attach_sampler_chrome_fills_updated_evt() {
        let mut evt = EngineEvt::bare(EngineEvtKind::Updated);
        evt.deck_id = Some(0);
        let mut chrome = empty_all_sampler_chrome();
        chrome[0][0] = SamplerSlotChrome {
            label: Some("kick".into()),
            track_id: None,
            path: Some("/samples/kick.wav".into()),
            duration_ms: Some(250),
        };
        attach_sampler_chrome(&mut evt, &chrome);
        assert!(evt.sampler_slots_known);
        let slots = evt.sampler_slots.expect("slots");
        assert_eq!(slots.len(), SAMPLER_SLOT_COUNT);
        assert_eq!(slots[0].label.as_deref(), Some("kick"));
        assert_eq!(slots[0].path.as_deref(), Some("/samples/kick.wav"));
        assert_eq!(slots[0].duration_ms, Some(250));
    }
}
