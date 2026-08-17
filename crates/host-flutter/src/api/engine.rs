//! FRB `AudioBackendTransport` (settings discovery) + `EngineTransport` (session + bus).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, DeckSnapshot, EvtBody, Kind, Origin};
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
    pub peak_hold_l: Option<f32>,
    pub peak_hold_r: Option<f32>,
    pub message: Option<String>,
    pub volume: Option<f32>,
    pub eq_low: Option<f32>,
    pub eq_mid: Option<f32>,
    pub eq_high: Option<f32>,
    pub filter: Option<f32>,
    pub gain_trim: Option<f32>,
    pub headphone_cue: Option<bool>,
    pub crossfader: Option<f32>,
    pub duration_ms: Option<i32>,
    pub speed: Option<f32>,
    pub tempo_range: Option<f32>,
}

impl EngineEvt {
    fn bare(kind: EngineEvtKind) -> Self {
        Self {
            kind,
            deck_id: None,
            running: None,
            playing: None,
            track: None,
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
            headphone_cue: None,
            crossfader: None,
            duration_ms: None,
            speed: None,
            tempo_range: None,
        }
    }
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
        let track_id = prepared.track_id.as_str().to_string();
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
            let mut snap = engine
                .deck_snapshot(deck_id as usize)
                .ok_or_else(|| "deck snapshot unavailable".to_string())?;
            // DSP snapshots omit library identity; the prepared load is the
            // source of truth Flutter needs to fetch L0/L1 peaks.
            snap.track_id = Some(track_id);
            snap
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
                        for mapped in map_engine_evts(ev.as_ref()) {
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
    evt.track = snap.track.clone();
    evt.track_id = snap.track_id.clone();
    evt.position_ms = snap.position_ms;
    evt.volume = Some(snap.volume);
    evt.eq_low = Some(snap.eq.low);
    evt.eq_mid = Some(snap.eq.mid);
    evt.eq_high = Some(snap.eq.high);
    evt.filter = Some(snap.filter);
    evt.gain_trim = Some(snap.gain_trim);
    evt.headphone_cue = Some(snap.headphone_cue);
    evt.duration_ms = snap.duration_ms;
    evt.speed = Some(snap.speed);
    evt.tempo_range = Some(snap.tempo_range);
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
            headphone_cue,
            duration_ms,
            speed,
            tempo_range,
            ..
        } => {
            let mut evt = EngineEvt::bare(EngineEvtKind::Updated);
            evt.deck_id = deck_id.or(Some(id));
            evt.playing = Some(playing);
            evt.track = track;
            evt.track_id = track_id;
            evt.position_ms = position_ms;
            evt.volume = Some(volume);
            evt.eq_low = Some(eq.low);
            evt.eq_mid = Some(eq.mid);
            evt.eq_high = Some(eq.high);
            evt.filter = Some(filter);
            evt.gain_trim = Some(gain_trim);
            evt.headphone_cue = Some(headphone_cue);
            evt.duration_ms = duration_ms;
            evt.speed = Some(speed);
            evt.tempo_range = Some(tempo_range);
            vec![evt]
        }
        EvtBody::Position { position_ms } => {
            let mut evt = EngineEvt::bare(EngineEvtKind::Position);
            evt.deck_id = deck_id;
            evt.position_ms = Some(position_ms);
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
    use engine_api::{DeckEq, EngineStatus, JogMode, PadMode, SamplerStatus, SyncMode};

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
            bpm: None,
            key: None,
            playing: false,
            volume,
            speed: 0.5,
            tempo_range: 0.08,
            eq: DeckEq {
                low: 0.5,
                mid: 0.5,
                high: 0.25,
            },
            filter: 0.5,
            gain_trim: 0.5,
            headphone_cue: true,
            sync_mode: SyncMode::Off,
            cue_point_ms: None,
            quantize: true,
            active_loop: None,
            pad_mode: PadMode::HotCue,
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
    fn map_status_fans_out_crossfader_and_decks() {
        let mapped = recv_mapped(
            Origin::Mixer,
            Kind::Status,
            EvtBody::EngineStatus {
                status: EngineStatus {
                    running: true,
                    sample_rate: 48_000,
                    crossfader: 0.25,
                    cue_mix: 0.0,
                    master_cue: false,
                    master_deck: 0,
                    decks: vec![sample_deck(0, 0.3), sample_deck(1, 0.7)],
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
        assert_eq!(mapped[1].kind, EngineEvtKind::Updated);
        assert_eq!(mapped[1].deck_id, Some(0));
        assert_eq!(mapped[1].volume, Some(0.3));
        assert_eq!(mapped[1].eq_high, Some(0.25));
        assert_eq!(mapped[1].headphone_cue, Some(true));
        assert_eq!(mapped[1].speed, Some(0.5));
        assert_eq!(mapped[1].tempo_range, Some(0.08));
        assert_eq!(mapped[2].deck_id, Some(1));
        assert_eq!(mapped[2].volume, Some(0.7));
    }
}
