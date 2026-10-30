//! FRB audio APIs (settings device discovery) + `EngineTransport` (session + bus).

use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bus_api::BeatJumpPadPress;
use bus_api::BeatJumpPadRelease;
use bus_api::BeginCueHold;
pub use bus_api::CrossfaderAssign;
use bus_api::DeckSnapshot;
use bus_api::DeckUpdated;
use bus_api::EndCueHold;
use bus_api::EngineSnapshot;
pub use bus_api::EqBand;
use bus_api::Event;
use bus_api::ExitLoop;
pub use bus_api::FxKind;
use bus_api::HotCuePadPress;
use bus_api::HotCuePadRelease;
use bus_api::JogTouch;
use bus_api::JogTurn;
use bus_api::KeyShiftPadPress;
use bus_api::KeyShiftPadRelease;
use bus_api::KeyboardPadPress;
use bus_api::KeyboardPadRelease;
use bus_api::LoopIn;
use bus_api::LoopOut;
use bus_api::LoopRollPadPress;
use bus_api::LoopRollPadRelease;
pub use bus_api::PadMode;
use bus_api::PadPress;
use bus_api::PadRelease;
use bus_api::Pause;
use bus_api::Play;
use bus_api::RecallSavedLoop;
use bus_api::SamplerPadPress;
use bus_api::SamplerPadRelease;
use bus_api::SamplerSlotInfo;
use bus_api::Seek;
use bus_api::SetAutoLoop;
use bus_api::SetCrossfader;
use bus_api::SetCrossfaderAssign;
use bus_api::SetCueMix;
use bus_api::SetCuePoint;
use bus_api::SetCueVolume;
use bus_api::SetDeckFxRouted;
use bus_api::SetEqBand;
use bus_api::SetFilter;
use bus_api::SetFxUnitParam;
use bus_api::SetFxUnitSlot;
use bus_api::SetFxUnitSlotEnabled;
use bus_api::SetFxUnitWet;
use bus_api::SetGainTrim;
use bus_api::SetHeadphoneCue;
use bus_api::SetJogMode;
use bus_api::SetKeyLock;
use bus_api::SetKeyShift;
use bus_api::SetKeyShiftPage;
use bus_api::SetKeyboardPage;
use bus_api::SetKeyboardRoot;
use bus_api::SetMasterCue;
use bus_api::SetMasterDeck;
use bus_api::SetMasterVolume;
use bus_api::SetPadMode;
use bus_api::SetQuantize;
use bus_api::SetSlip;
use bus_api::SetSpeed;
use bus_api::SetTempoRange;
use bus_api::SetVolume;
use bus_api::ToggleSync;
use bus_api::Unload;
use engine_core::{
    spawn_engine_worker, AudioBackend, Engine, EngineConfig, EngineWorker, MAX_DECKS,
};
use event_bus::EventBus;
use library::{LibraryManager, PreparedTrackPlayback, SamplerSlotRecord};
use library_core::{AudioSource, TrackId};

use crate::api::bus::BusTransport;
use crate::api::bus::DeckChrome;
use crate::api::history_worker::HistoryWorker;
use crate::api::library::LibraryTransport;
use crate::api::settings::{
    settings_engine_config, settings_host_runtime, settings_realtime_stems_enabled, JogModeSetting,
};
use crate::bridge::BridgeResult;

const SAMPLER_SLOT_COUNT: usize = library::SAMPLER_BANK_SIZE;

/// Inclusive sample-rate span a device can open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleRateRange {
    pub min: u32,
    pub max: u32,
}

/// Output device summary for the Flutter settings / smoke UI.
#[derive(Clone, Debug)]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub max_channels: u16,
    pub default_sample_rates: Vec<u32>,
    /// Rate the system mixes this device at; `None` when the backend cannot report
    /// it. Settings → Audio shows it as the "Device default" rate.
    pub mix_sample_rate: Option<u32>,
    /// Spans of rates the device can open, so the UI can tell whether a rate it
    /// remembers from another device is still valid here.
    pub sample_rate_ranges: Vec<SampleRateRange>,
}

/// Output devices of the compiled-in real backend (Settings → Audio).
///
/// `"auto"` resolves through the engine registry to the shipped device
/// backend; it never falls back to the headless `null` backend.
pub fn list_output_devices() -> Result<Vec<OutputDevice>, String> {
    let backend = AudioBackend::new("auto").bridge()?;
    let devices = backend.list_output_devices().bridge()?;
    Ok(devices.into_iter().map(output_device).collect())
}

fn output_device(device: audio_core::DeviceInfo) -> OutputDevice {
    OutputDevice {
        id: device.id.as_str().to_string(),
        name: device.name,
        is_default: device.is_default,
        max_channels: device.max_channels,
        default_sample_rates: device.default_sample_rates,
        mix_sample_rate: device.mix_sample_rate,
        sample_rate_ranges: device
            .sample_rate_ranges
            .into_iter()
            .map(|range| SampleRateRange {
                min: range.min,
                max: range.max,
            })
            .collect(),
    }
}
#[allow(dead_code)]
struct EngineHistoryWorker(HistoryWorker);

/// Filesystem path of a prepared source, for the sampler pad tooltip.
fn source_path(source: &AudioSource) -> Option<String> {
    source
        .file()
        .map(|f| f.path().to_string_lossy().into_owned())
}

/// Display label for a sampler pad: metadata title, else file stem, else track id.
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

/// Sampler pad chrome for one prepared source.
fn chrome_from_prepared(prepared: &PreparedTrackPlayback) -> SamplerSlotInfo {
    SamplerSlotInfo {
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
) -> SamplerSlotInfo {
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

/// Host-owned engine handle exposed to Dart via FRB methods.
#[flutter_rust_bridge::frb(opaque)]
pub struct EngineTransport {
    /// Declared first so Drop joins the worker before `engine` is destroyed.
    #[allow(dead_code)]
    worker: EngineWorker,
    engine: Arc<Mutex<Option<Engine>>>,
    bus: EventBus<Event>,
    library: Arc<Mutex<LibraryManager>>,
    library_cmd_bus: library::LibraryBus,
    /// Ephemeral sampler pad chrome, stamped onto `DeckUpdated` by [`crate::api::bus`].
    sampler_slots: DeckChrome,
    history_worker: Mutex<Option<EngineHistoryWorker>>,
    /// Bumped at the start of each deck load; stale prepares skip `load_prepared`.
    ///
    /// Shared with a background StemgenRT model download so a track replaced
    /// while the model is downloading never starts a stale producer.
    deck_load_generation: Arc<Vec<AtomicU64>>,
    deck_count: AtomicUsize,
    /// Mixar-owned model root, for opportunistic live separation.
    models_root: std::path::PathBuf,
    /// Live producers per deck; replacing or dropping one stops it.
    live_separators: Arc<Mutex<Vec<Option<analyzer_stems::LiveSeparator>>>>,
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
    /// Start the engine on the shared bus, from the saved settings host config.
    pub fn start(bus: &BusTransport, library_transport: &LibraryTransport) -> Result<Self, String> {
        Self::start_with(
            bus.bus(),
            bus.deck_chrome(),
            library_transport,
            settings_engine_config()?,
        )
    }

    /// Start with an explicit config. Crate-private: only the `test-utils`
    /// `testing::start_engine` seam calls this, so the Dart bridge never
    /// exposes a backend selector.
    pub(crate) fn start_with(
        bus: EventBus<Event>,
        chrome: DeckChrome,
        library_transport: &LibraryTransport,
        config: EngineConfig,
    ) -> Result<Self, String> {
        let library_arc = library_transport.library_arc();
        let live_stems_enabled = config.backend != "null";
        let deck_count = config.num_decks;
        let mut engine =
            Engine::new_with_library_bus(config, Arc::clone(&library_arc), library_transport.bus())
                .bridge()?;
        engine.set_buses(bus.clone());
        // Failure reason reaches `mixar.log` through `BridgeResult`, like every other error.
        engine.start().bridge()?;
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
        let history_worker = match HistoryWorker::start(&bus, library_arc.clone()) {
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
            bus,
            library: library_arc,
            library_cmd_bus: library_transport.bus(),
            sampler_slots: chrome,
            history_worker: Mutex::new(Some(history_worker)),
            deck_load_generation: Arc::new((0..MAX_DECKS).map(|_| AtomicU64::new(0)).collect()),
            deck_count: AtomicUsize::new(deck_count),
            models_root: library_transport.library_buses().models_root(),
            live_separators: Arc::new(Mutex::new((0..MAX_DECKS).map(|_| None).collect())),
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
        for generation in self.deck_load_generation.iter() {
            generation.fetch_add(1, Ordering::AcqRel);
        }
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
            engine.stop().bridge()?;
        }
        let previous_deck_count = self.deck_count.load(Ordering::Acquire);
        let deck_count = config.num_decks;
        match self.build_started_engine(config) {
            Ok(engine) => {
                // A fresh engine is empty, so deck 0 is a neutral unloaded
                // template for retiring decks the new profile dropped.
                let retirement = engine.deck_snapshot(0);
                if let Ok(mut chrome) = self.sampler_slots.lock() {
                    chrome.resize_with(deck_count, Vec::new);
                }
                self.deck_count.store(deck_count, Ordering::Release);
                *guard = Some(engine);
                drop(guard);
                if previous_deck_count > deck_count {
                    if let Some(blank) = retirement {
                        self.publish_retired_decks(previous_deck_count, deck_count, blank);
                    }
                }
                // Hydrate every deck of the new profile, including empty ones.
                self.publish_current_status();
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
        .bridge()?;
        engine.set_buses(self.bus.clone());
        engine.start().bridge()?;
        Ok(engine)
    }

    /// Apply normalizer + jog defaults after start/restart.
    #[flutter_rust_bridge::frb(ignore)]
    pub fn apply_host_settings(
        &self,
        normalizer_target: Option<f32>,
        top_jog: bus_api::JogMode,
        outer_jog: bus_api::JogMode,
    ) -> Result<(), String> {
        let mut guard = self
            .engine
            .lock()
            .map_err(|_| "engine lock poisoned".to_string())?;
        let engine = guard
            .as_mut()
            .ok_or_else(|| "engine not available".to_string())?;
        engine.set_normalizer_target(normalizer_target).bridge()?;
        // Read under the engine lock: restart updates the count while holding it.
        let deck_count = self.deck_count.load(Ordering::Acquire);
        for deck_id in 0..deck_count {
            engine
                .set_deck_jog_mode(deck_id, top_jog, outer_jog)
                .bridge()?;
        }
        Ok(())
    }

    /// Restart using the current settings host config + runtime normalizer/jog defaults.
    pub fn restart_from_settings(&self) -> Result<(), String> {
        let config = settings_engine_config()?;
        self.restart(config)?;
        let applied = settings_host_runtime()
            .and_then(|(target, top, outer)| self.apply_host_settings(target, top, outer));
        // Hydrate every deck of the new profile, including empty ones: a bare
        // profile swap otherwise leaves their routing/state at mirror defaults.
        self.publish_current_status();
        applied
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
        engine.stop().bridge()
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
        self.publish_cmd(Event::Play(Play { deck: deck_id }))
    }

    /// Pause a deck (cmd bus).
    pub fn pause(&self, deck_id: u16) -> Result<(), String> {
        self.publish_cmd(Event::Pause(Pause { deck: deck_id }))
    }

    /// Seek a deck to `position_ms` (cmd bus).
    pub fn seek(&self, deck_id: u16, position_ms: i32) -> Result<(), String> {
        self.publish_cmd(Event::Seek(Seek {
            deck: deck_id,
            position_ms,
        }))
    }

    /// Channel fader `0..1`.
    pub fn set_volume(&self, deck_id: u16, volume: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetVolume(SetVolume {
            deck: deck_id,
            volume,
            soft_takeover: false,
        }))
    }

    /// Single EQ band as `0..1` (center `0.5` = 0 dB).
    pub fn set_eq_band(&self, deck_id: u16, band: EqBand, gain: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetEqBand(SetEqBand {
            deck: deck_id,
            band,
            gain,
            soft_takeover: false,
        }))
    }

    /// Filter knob `0..1`.
    pub fn set_filter(&self, deck_id: u16, filter: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetFilter(SetFilter {
            deck: deck_id,
            filter,
            soft_takeover: false,
        }))
    }

    /// FX unit slot `0..=2` effect selection; `off` clears the slot.
    pub fn set_fx_unit_slot(&self, slot: u8, kind: FxKind) -> Result<(), String> {
        self.publish_cmd(Event::SetFxUnitSlot(SetFxUnitSlot { slot, kind }))
    }

    /// FX unit slot on/off (wet/dry and knobs are kept).
    pub fn set_fx_unit_slot_enabled(&self, slot: u8, enabled: bool) -> Result<(), String> {
        self.publish_cmd(Event::SetFxUnitSlotEnabled(SetFxUnitSlotEnabled {
            slot,
            enabled,
        }))
    }

    /// FX unit slot wet/dry `0..1`.
    pub fn set_fx_unit_wet(&self, slot: u8, wet: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetFxUnitWet(SetFxUnitWet {
            slot,
            wet,
            soft_takeover: false,
        }))
    }

    /// FX unit slot knob `param` `0..=2`, normalized `0..1`.
    pub fn set_fx_unit_param(&self, slot: u8, param: u8, value: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetFxUnitParam(SetFxUnitParam { slot, param, value }))
    }

    /// Route deck `deck_id` through the shared FX unit.
    pub fn set_deck_fx_routed(&self, deck_id: u16, routed: bool) -> Result<(), String> {
        self.publish_cmd(Event::SetDeckFxRouted(SetDeckFxRouted {
            deck: deck_id,
            routed,
        }))
    }

    /// Gain trim knob `0..1`.
    pub fn set_gain_trim(&self, deck_id: u16, gain_trim: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetGainTrim(SetGainTrim {
            deck: deck_id,
            gain_trim,
            soft_takeover: false,
        }))
    }

    /// Per-deck headphone cue (PFL).
    pub fn set_headphone_cue(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_cmd(Event::SetHeadphoneCue(SetHeadphoneCue {
            deck: deck_id,
            enabled,
        }))
    }

    /// Crossfader `0..1` (A … B).
    pub fn set_crossfader(&self, position: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetCrossfader(SetCrossfader {
            position,
            soft_takeover: false,
        }))
    }

    /// Assign a deck to crossfader side A, Thru, or side B.
    pub fn set_crossfader_assign(
        &self,
        deck_id: u16,
        assignment: CrossfaderAssign,
    ) -> Result<(), String> {
        self.publish_cmd(Event::SetCrossfaderAssign(SetCrossfaderAssign {
            deck: deck_id,
            assignment,
        }))
    }

    /// Master bus volume `0..1`.
    pub fn set_master_volume(&self, volume: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetMasterVolume(SetMasterVolume {
            volume,
            soft_takeover: false,
        }))
    }

    /// Cue/headphone bus volume `0..1`.
    pub fn set_cue_volume(&self, volume: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetCueVolume(SetCueVolume {
            volume,
            soft_takeover: false,
        }))
    }

    /// Cue/master headphone mix `0..1`.
    pub fn set_cue_mix(&self, mix: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetCueMix(SetCueMix {
            mix,
            soft_takeover: false,
        }))
    }

    /// Master cue (headphones hear master).
    pub fn set_master_cue(&self, enabled: bool) -> Result<(), String> {
        self.publish_cmd(Event::SetMasterCue(SetMasterCue { enabled }))
    }

    /// Tempo fader position `0..1`.
    pub fn set_speed(&self, deck_id: u16, speed: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetSpeed(SetSpeed {
            deck: deck_id,
            speed,
            soft_takeover: false,
        }))
    }

    /// Tempo fader half-span as pitch fraction (`0.06` = ±6%).
    pub fn set_tempo_range(&self, deck_id: u16, tempo_range: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetTempoRange(SetTempoRange {
            deck: deck_id,
            tempo_range,
        }))
    }

    /// Key lock / master tempo (time-stretch; pitch held).
    pub fn set_key_lock(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_cmd(Event::SetKeyLock(SetKeyLock {
            deck: deck_id,
            enabled,
        }))
    }

    /// Session key-shift offset in semitones (`-16..=16`; `0` = bypass).
    pub fn set_key_shift(&self, deck_id: u16, semitones: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetKeyShift(SetKeyShift {
            deck: deck_id,
            semitones,
        }))
    }

    /// Select the hot-cue slot used as the Keyboard pad root (0..=15).
    pub fn set_keyboard_root(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::SetKeyboardRoot(SetKeyboardRoot {
            deck: deck_id,
            slot,
        }))
    }

    /// Select the Keyboard pad semitone page (1..=4).
    pub fn set_keyboard_page(&self, deck_id: u16, page: u8) -> Result<(), String> {
        self.publish_cmd(Event::SetKeyboardPage(SetKeyboardPage {
            deck: deck_id,
            page,
        }))
    }

    /// Select the Key Shift pad semitone page (1..=5).
    pub fn set_key_shift_page(&self, deck_id: u16, page: u8) -> Result<(), String> {
        self.publish_cmd(Event::SetKeyShiftPage(SetKeyShiftPage {
            deck: deck_id,
            page,
        }))
    }

    pub fn jog_touch(&self, deck_id: u16, touching: bool) -> Result<(), String> {
        self.publish_cmd(Event::JogTouch(JogTouch {
            deck: deck_id,
            touching,
        }))
    }

    pub fn jog_turn(&self, deck_id: u16, delta: i32) -> Result<(), String> {
        self.publish_cmd(Event::JogTurn(JogTurn {
            deck: deck_id,
            delta,
        }))
    }

    pub fn set_jog_mode(
        &self,
        deck_id: u16,
        top: JogModeSetting,
        outer: JogModeSetting,
    ) -> Result<(), String> {
        self.publish_cmd(Event::SetJogMode(SetJogMode {
            deck: deck_id,
            top: top.into(),
            outer: outer.into(),
        }))
    }

    pub fn set_cue_point(&self, deck_id: u16) -> Result<(), String> {
        self.publish_cmd(Event::SetCuePoint(SetCuePoint { deck: deck_id }))
    }

    pub fn begin_cue_hold(&self, deck_id: u16) -> Result<(), String> {
        self.publish_cmd(Event::BeginCueHold(BeginCueHold { deck: deck_id }))
    }

    pub fn end_cue_hold(&self, deck_id: u16) -> Result<(), String> {
        self.publish_cmd(Event::EndCueHold(EndCueHold { deck: deck_id }))
    }

    pub fn set_quantize(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_cmd(Event::SetQuantize(SetQuantize {
            deck: deck_id,
            enabled,
        }))
    }

    pub fn set_slip(&self, deck_id: u16, enabled: bool) -> Result<(), String> {
        self.publish_cmd(Event::SetSlip(SetSlip {
            deck: deck_id,
            enabled,
        }))
    }

    pub fn unload(&self, deck_id: u16) -> Result<(), String> {
        if usize::from(deck_id) >= self.deck_count.load(Ordering::Acquire) {
            return Err(format!("Invalid deck ID: {deck_id}"));
        }
        // Drop the producer + its ring before the deck unload closes coverage.
        if let Ok(mut live) = self.live_separators.lock() {
            if let Some(previous) = live.get_mut(usize::from(deck_id)).and_then(Option::take) {
                previous.stop();
            }
        }
        self.publish_cmd(Event::Unload(Unload { deck: deck_id }))
    }

    pub fn set_auto_loop(&self, deck_id: u16, beats: f32) -> Result<(), String> {
        self.publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: deck_id,
            beats,
        }))
    }

    pub fn loop_in(&self, deck_id: u16, position_ms: i32) -> Result<(), String> {
        self.publish_cmd(Event::LoopIn(LoopIn {
            deck: deck_id,
            position_ms,
        }))
    }

    pub fn loop_out(&self, deck_id: u16, position_ms: i32) -> Result<(), String> {
        self.publish_cmd(Event::LoopOut(LoopOut {
            deck: deck_id,
            position_ms,
        }))
    }

    pub fn exit_loop(&self, deck_id: u16) -> Result<(), String> {
        self.publish_cmd(Event::ExitLoop(ExitLoop { deck: deck_id }))
    }

    pub fn recall_saved_loop(&self, deck_id: u16, in_ms: i32, out_ms: i32) -> Result<(), String> {
        self.publish_cmd(Event::RecallSavedLoop(RecallSavedLoop {
            deck: deck_id,
            in_ms,
            out_ms,
        }))
    }

    pub fn toggle_sync(&self, deck_id: u16, beat_sync: bool) -> Result<(), String> {
        self.publish_cmd(Event::ToggleSync(ToggleSync {
            deck: deck_id,
            beat_sync,
        }))
    }

    pub fn set_master_deck(&self, deck_id: u16) -> Result<(), String> {
        self.publish_cmd(Event::SetMasterDeck(SetMasterDeck { deck: deck_id }))
    }

    /// Per-deck pad mode.
    pub fn set_pad_mode(&self, deck_id: u16, mode: PadMode) -> Result<(), String> {
        self.publish_cmd(Event::SetPadMode(SetPadMode {
            deck: deck_id,
            mode,
        }))
    }

    /// Stems / generic pad press (engine dispatches by current `pad_mode`).
    pub fn pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_cmd(Event::PadPress(PadPress {
            deck: deck_id,
            slot,
            shift,
        }))
    }

    pub fn pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::PadRelease(PadRelease {
            deck: deck_id,
            slot,
        }))
    }

    pub fn hot_cue_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_cmd(Event::HotCuePadPress(HotCuePadPress {
            deck: deck_id,
            slot,
            shift,
        }))
    }

    pub fn hot_cue_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::HotCuePadRelease(HotCuePadRelease {
            deck: deck_id,
            slot,
        }))
    }

    pub fn loop_roll_pad_press(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::LoopRollPadPress(LoopRollPadPress {
            deck: deck_id,
            slot,
        }))
    }

    pub fn loop_roll_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::LoopRollPadRelease(LoopRollPadRelease {
            deck: deck_id,
            slot,
        }))
    }

    pub fn beat_jump_pad_press(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::BeatJumpPadPress(BeatJumpPadPress {
            deck: deck_id,
            slot,
        }))
    }

    pub fn beat_jump_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::BeatJumpPadRelease(BeatJumpPadRelease {
            deck: deck_id,
            slot,
        }))
    }

    /// Keyboard pad press; `shift` plays the upper octave / accent.
    pub fn keyboard_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_cmd(Event::KeyboardPadPress(KeyboardPadPress {
            deck: deck_id,
            slot,
            shift,
        }))
    }

    pub fn keyboard_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::KeyboardPadRelease(KeyboardPadRelease {
            deck: deck_id,
            slot,
        }))
    }

    /// Key Shift pad press; `shift` resets the deck to `0.0` semitones.
    pub fn key_shift_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_cmd(Event::KeyShiftPadPress(KeyShiftPadPress {
            deck: deck_id,
            slot,
            shift,
        }))
    }

    pub fn key_shift_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::KeyShiftPadRelease(KeyShiftPadRelease {
            deck: deck_id,
            slot,
        }))
    }

    pub fn sampler_pad_press(&self, deck_id: u16, slot: u8, shift: bool) -> Result<(), String> {
        self.publish_cmd(Event::SamplerPadPress(SamplerPadPress {
            deck: deck_id,
            slot,
            shift,
        }))
    }

    pub fn sampler_pad_release(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        self.publish_cmd(Event::SamplerPadRelease(SamplerPadRelease {
            deck: deck_id,
            slot,
        }))
    }

    pub fn assign_sampler(&self, deck_id: u16, slot: u8, path: String) -> Result<(), String> {
        let prepared =
            LibraryManager::prepare_file_path_for_playback(self.library.as_ref(), Path::new(&path))
                .bridge()?;
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
        .bridge()?;
        self.assign_prepared(deck_id, slot, prepared)
    }

    pub fn clear_sampler(&self, deck_id: u16, slot: u8) -> Result<(), String> {
        let deck = usize::from(deck_id);
        let slot_i = usize::from(slot);
        if deck >= self.deck_count.load(Ordering::Acquire) || slot_i >= SAMPLER_SLOT_COUNT {
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
            engine.clear_sampler_slot(deck, slot_i).bridge()?;
            // Hold the engine lock across the chrome write: `restart` takes the
            // engine lock before resizing the chrome, so this order keeps a
            // shrink from truncating the slot between the lookup and the write.
            let mut chrome = self
                .sampler_slots
                .lock()
                .map_err(|_| "sampler chrome lock poisoned".to_string())?;
            let deck_chrome = chrome
                .get_mut(deck)
                .ok_or_else(|| format!("Invalid sampler deck/slot: {deck_id}/{slot}"))?;
            let snap = engine
                .deck_snapshot(deck)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?;
            deck_chrome[slot_i] = SamplerSlotInfo::default();
            snap
        };
        self.publish_deck_updated(deck_id, snap)
    }

    /// Load a sampler bank's slots onto a deck (prepare outside the engine lock).
    pub fn set_sampler_bank(&self, deck_id: u16, bank_id: String) -> Result<(), String> {
        let deck = usize::from(deck_id);
        if deck >= self.deck_count.load(Ordering::Acquire) {
            return Err(format!("Invalid deck ID: {deck_id}"));
        }
        let slots = {
            let lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            if lib.get_sampler_bank(&bank_id).bridge()?.is_none() {
                return Err(format!("Sampler bank not found: {bank_id}"));
            }
            lib.list_sampler_bank_slots(&bank_id).bridge()?
        };
        let mut prepared: Vec<(u8, PreparedTrackPlayback, SamplerSlotInfo)> = Vec::new();
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
            .bridge()?;
            let chrome = chrome_from_bank_slot(&record, &item);
            prepared.push((record.slot_index, item, chrome));
        }
        let mut deck_chrome = vec![SamplerSlotInfo::default(); SAMPLER_SLOT_COUNT];
        let snap = {
            let mut guard = self
                .engine
                .lock()
                .map_err(|_| "engine lock poisoned".to_string())?;
            let engine = guard
                .as_mut()
                .ok_or_else(|| "engine not available".to_string())?;
            engine.clear_all_sampler_slots(deck).bridge()?;
            for (slot, item, chrome) in prepared {
                engine
                    .assign_prepared_sampler(deck, usize::from(slot), item)
                    .bridge()?;
                deck_chrome[usize::from(slot)] = chrome;
            }
            engine.set_deck_sampler_bank(deck, Some(bank_id)).bridge()?;
            // Same lock order as `restart`: engine, then sampler chrome.
            let mut chrome = self
                .sampler_slots
                .lock()
                .map_err(|_| "sampler chrome lock poisoned".to_string())?;
            let slot_chrome = chrome
                .get_mut(deck)
                .ok_or_else(|| format!("Invalid deck ID: {deck_id}"))?;
            let snap = engine
                .deck_snapshot(deck)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?;
            *slot_chrome = deck_chrome;
            snap
        };
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
        lib.update_sampler_bank(&bank_id, &name, play_mode).bridge()
    }

    /// Load a library track: prepare outside the engine lock, then `load_prepared_track`.
    pub fn load_library_track(&self, deck_id: u16, track_id: String) -> Result<(), String> {
        let generation = self.begin_deck_load(deck_id)?;
        let prepared = LibraryManager::prepare_track_for_playback(
            self.library.as_ref(),
            &TrackId::new(track_id),
        )
        .bridge()?;
        self.load_prepared(deck_id, prepared, generation)
    }

    /// Load a filesystem path: prepare outside the engine lock, then `load_prepared_track`.
    pub fn load_path(&self, deck_id: u16, path: String) -> Result<(), String> {
        let generation = self.begin_deck_load(deck_id)?;
        let prepared =
            LibraryManager::prepare_file_path_for_playback(self.library.as_ref(), Path::new(&path))
                .bridge()?;
        self.load_prepared(deck_id, prepared, generation)
    }

    fn begin_deck_load(&self, deck_id: u16) -> Result<u64, String> {
        let idx = usize::from(deck_id);
        if idx >= self.deck_count.load(Ordering::Acquire) {
            return Err(format!("invalid deck id: {deck_id}"));
        }
        Ok(self.deck_load_generation[idx].fetch_add(1, Ordering::AcqRel) + 1)
    }

    fn is_current_deck_load(&self, deck_id: u16, generation: u64) -> bool {
        let idx = usize::from(deck_id);
        idx < self.deck_count.load(Ordering::Acquire)
            && self.deck_load_generation[idx].load(Ordering::Acquire) == generation
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
                if let Some(previous) = live.get_mut(deck_id as usize).and_then(Option::take) {
                    previous.stop();
                }
            }
            engine
                .load_prepared_track(deck_id as usize, prepared)
                .bridge()?;
            if let Some(stems) = stems {
                engine.attach_deck_stems(deck_id as usize, stems).bridge()?;
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
        if deck >= self.deck_count.load(Ordering::Acquire) || slot_i >= SAMPLER_SLOT_COUNT {
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
                .bridge()?;
            // Same lock order as `restart`: engine, then sampler chrome.
            let mut slots = self
                .sampler_slots
                .lock()
                .map_err(|_| "sampler chrome lock poisoned".to_string())?;
            let deck_chrome = slots
                .get_mut(deck)
                .ok_or_else(|| format!("Invalid sampler deck/slot: {deck_id}/{slot}"))?;
            let snap = engine
                .deck_snapshot(deck)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?;
            deck_chrome[slot_i] = chrome;
            snap
        };
        self.publish_deck_updated(deck_id, snap)
    }

    fn publish_deck_updated(&self, deck_id: u16, snap: DeckSnapshot) -> Result<(), String> {
        let mut update = DeckUpdated::from(snap);
        update.deck = deck_id;
        self.bus.publish_evt(Event::DeckUpdated(update)).bridge()
    }

    /// Publish an empty, stopped snapshot for every deck the new profile dropped.
    ///
    /// History and the UI keep per-deck state by id; without this, decks removed
    /// by shrinking the profile stay "playing" and their set entries never close.
    fn publish_retired_decks(
        &self,
        previous_deck_count: usize,
        deck_count: usize,
        mut blank: DeckSnapshot,
    ) {
        blank.playing = false;
        blank.track = None;
        blank.track_id = None;
        blank.title = None;
        blank.artist = None;
        blank.album = None;
        blank.isrc = None;
        blank.bpm = None;
        blank.key = None;
        blank.position_ms = None;
        blank.duration_ms = None;
        blank.cue_point_ms = None;
        blank.active_loop = None;
        blank.pending_loop_in_ms = None;
        blank.slip_shadow_position_ms = None;
        blank.stems_ready = false;
        blank.stems_live = false;
        blank.loudness_lufs = None;
        blank.active_sampler_bank_id = None;
        blank.hot_cues.clear();
        blank.saved_loops.clear();
        for deck_id in deck_count..previous_deck_count {
            blank.id = deck_id as u16;
            let _ = self.publish_deck_updated(blank.id, blank.clone());
        }
    }

    /// Publish one command on the shared bus.
    fn publish_cmd(&self, event: Event) -> Result<(), String> {
        self.bus.publish_cmd(event).bridge()
    }

    /// Hydrate every deck and the mixer from one snapshot.
    fn publish_current_status(&self) {
        let status = self
            .engine
            .lock()
            .ok()
            .and_then(|g| g.as_ref().and_then(Engine::engine_status_snapshot));
        if let Some(status) = status {
            let _ = self
                .bus
                .publish_evt(Event::EngineSnapshot(EngineSnapshot { status }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bus_api::JogMode;
    use std::time::Duration;

    /// The bus the engine publishes on, plus the library it loads from.
    fn null_transport() -> (
        BusTransport,
        EngineTransport,
        crate::api::library::LibraryTransport,
    ) {
        let bus = BusTransport::open().unwrap();
        let library = crate::api::library::LibraryTransport::open_in_memory(&bus).unwrap();
        let transport =
            EngineTransport::start_with(bus.bus(), bus.deck_chrome(), &library, profile_config(2))
                .unwrap();
        (bus, transport, library)
    }

    fn profile_config(num_decks: usize) -> EngineConfig {
        EngineConfig {
            num_decks,
            backend: "null".into(),
            ..EngineConfig::default()
        }
    }

    fn evt_subscriber(bus: &BusTransport) -> event_bus::Subscriber<Event> {
        bus.bus().subscribe_evt().unwrap()
    }

    fn recv_status(rx: &mut event_bus::Subscriber<Event>) -> bus_api::EngineStatus {
        loop {
            let event = rx
                .recv_timeout(Duration::from_secs(5))
                .expect("evt bus")
                .expect("evt");
            if let Event::EngineSnapshot(snapshot) = event {
                return snapshot.status;
            }
        }
    }

    #[test]
    fn growing_the_profile_hydrates_every_new_deck() {
        let (bus, transport, _library) = null_transport();
        let mut rx = evt_subscriber(&bus);
        transport.restart(profile_config(4)).unwrap();
        let status = recv_status(&mut rx);
        assert_eq!(status.decks.len(), 4);
        let routing: Vec<_> = status
            .decks
            .iter()
            .map(|deck| deck.crossfader_assign)
            .collect();
        assert_eq!(
            routing,
            vec![
                CrossfaderAssign::A,
                CrossfaderAssign::B,
                CrossfaderAssign::A,
                CrossfaderAssign::B,
            ]
        );
    }

    #[test]
    fn shrinking_the_profile_retires_removed_decks() {
        let (bus, transport, _library) = null_transport();
        transport.restart(profile_config(4)).unwrap();
        let mut rx = evt_subscriber(&bus);
        transport.restart(profile_config(2)).unwrap();
        for retired in [2u16, 3] {
            let update = loop {
                let event = rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("evt bus")
                    .expect("evt");
                if let Event::DeckUpdated(update) = event {
                    if update.deck == retired {
                        break update;
                    }
                }
            };
            assert!(!update.playing);
            assert!(update.track.is_none());
            assert!(update.track_id.is_none());
            assert!(update.duration_ms.is_none());
        }
        // The retired slots are gone: sampler commands must fail, not panic.
        assert!(transport.clear_sampler(3, 0).is_err());
        assert!(transport.set_sampler_bank(3, "bank".into()).is_err());
    }

    #[test]
    fn jog_defaults_apply_to_every_enabled_deck() {
        let (_bus, transport, _library) = null_transport();
        transport.restart(profile_config(4)).unwrap();
        transport
            .apply_host_settings(None, JogMode::Ignore, JogMode::Vinyl)
            .unwrap();
        let guard = transport.engine.lock().unwrap();
        let deck = guard
            .as_ref()
            .and_then(|engine| engine.deck_snapshot(3))
            .expect("deck D snapshot");
        assert_eq!(deck.top_jog_mode, JogMode::Ignore);
        assert_eq!(deck.outer_jog_mode, JogMode::Vinyl);
    }
}
