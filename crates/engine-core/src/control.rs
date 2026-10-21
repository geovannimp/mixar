//! Control thread: cmd dispatch and high-rate evt egress.

use crate::bus::EngineBus;
use crate::engine::{Engine, NUM_DECKS};
use crate::soft_takeover;
use crate::transport::TransportEvent;
use anyhow::{anyhow, Result};
use engine_api::{
    decode_cmd_body, encode_evt_body, CmdBody, DeckSnapshot, EqBand, EvtBody, Kind, Origin,
};
use library::{LibraryManager, PreparedTrackPlayback};
use library_api::{
    decode_evt_body as decode_library_evt_body, EvtBody as LibraryEvtBody, Kind as LibraryKind,
};
use library_core::TrackId;
use omnibus::Event;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const PEAK_HOLD_DECAY_PER_TICK: f32 = 0.04;
const LEVEL_IDLE_EPSILON: f32 = 1e-5;

enum CmdOutcome {
    DeckUpdated(usize),
    DecksUpdated(Vec<usize>),
    EngineStatus,
    /// Publish a Position evt for `deck_id` (playhead moved without a full DeckUpdated).
    Position(usize),
    Silent,
}

struct PeakHoldState {
    hold_l: Vec<f32>,
    hold_r: Vec<f32>,
    /// Decks that last published non-idle levels and still need a zero flush for the UI.
    needs_zero_flush: Vec<bool>,
}

impl PeakHoldState {
    fn new() -> Self {
        // Deck slots 0..NUM_DECKS-1 plus master at NUM_DECKS.
        let slots = NUM_DECKS + 1;
        Self {
            hold_l: vec![0.0; slots],
            hold_r: vec![0.0; slots],
            needs_zero_flush: vec![false; slots],
        }
    }

    fn ensure_capacity(&mut self, deck_id: usize) {
        let need = deck_id + 1;
        if self.hold_l.len() < need {
            self.hold_l.resize(need, 0.0);
            self.hold_r.resize(need, 0.0);
            self.needs_zero_flush.resize(need, false);
        }
    }

    fn update(&mut self, deck_id: usize, peak_l: f32, peak_r: f32) -> (f32, f32) {
        self.ensure_capacity(deck_id);
        Self::ballistics(&mut self.hold_l[deck_id], peak_l);
        Self::ballistics(&mut self.hold_r[deck_id], peak_r);
        (self.hold_l[deck_id], self.hold_r[deck_id])
    }

    fn ballistics(hold: &mut f32, peak: f32) {
        if peak >= *hold {
            *hold = peak;
        } else {
            *hold = (*hold - PEAK_HOLD_DECAY_PER_TICK).max(0.0);
        }
    }

    fn mark_published(&mut self, deck_id: usize, idle: bool) {
        self.ensure_capacity(deck_id);
        self.needs_zero_flush[deck_id] = !idle;
    }

    fn take_zero_flush(&mut self, deck_id: usize) -> bool {
        self.ensure_capacity(deck_id);
        if self.needs_zero_flush[deck_id] {
            self.needs_zero_flush[deck_id] = false;
            true
        } else {
            false
        }
    }
}

/// Host-owned engine cmd worker (`JoinHandle` must not live inside `Mutex<Engine>`).
pub struct EngineWorker {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for EngineWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn the cmd-draining worker using buses attached to `engine`.
///
/// Errors if [`Engine::set_buses`] was not called, or if the thread fails to subscribe.
pub fn spawn_engine_worker(engine: Arc<Mutex<Option<Engine>>>) -> Result<EngineWorker> {
    let buses = {
        let guard = engine.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().and_then(Engine::buses).ok_or_else(|| {
            anyhow!("engine buses not attached; call set_buses before spawn_engine_worker")
        })?
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    let cmd = buses.cmd_bus();
    let evt = buses.evt_bus();
    let revision = buses.revision_arc();
    let engine_handle = Arc::clone(&engine);
    let shutdown_flag = Arc::clone(&shutdown);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
    let handle = std::thread::spawn(move || {
        control_thread_loop(cmd, evt, engine_handle, revision, shutdown_flag, ready_tx);
    });
    ready_rx
        .recv()
        .map_err(|_| anyhow!("engine worker failed to start"))??;

    Ok(EngineWorker {
        shutdown,
        handle: Some(handle),
    })
}

fn control_thread_loop(
    cmd_bus: EngineBus,
    evt_bus: EngineBus,
    engine: Arc<Mutex<Option<Engine>>>,
    revision: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    ready: std::sync::mpsc::Sender<Result<()>>,
) {
    let rx = match cmd_bus.subscribe(omnibus::Filter::Any, omnibus::Filter::Any) {
        Ok(rx) => rx,
        Err(e) => {
            let _ = ready.send(Err(e.into()));
            return;
        }
    };
    // Best-effort at control-thread start: subscribe to library HotCuesChanged so
    // deck_control.hot_cues commits only after DB ack. Hosts attach buses before
    // spawn; no retry if subscribe fails or library has no buses.
    let library_evt_rx = library_evt_receiver(&engine);
    let _ = ready.send(Ok(()));
    let mut peak_hold = PeakHoldState::new();

    while !shutdown.load(Ordering::Relaxed) {
        match rx.recv_timeout(TICK_INTERVAL) {
            Ok(Some(event)) => handle_cmd_event(&event, &engine, &evt_bus, &revision),
            Ok(None) => {}
            Err(_) => break,
        }
        if let Some(lib_rx) = library_evt_rx.as_ref() {
            drain_library_evt(lib_rx, &engine, &evt_bus, &revision);
        }
        tick(&engine, &evt_bus, &revision, &mut peak_hold);
    }
}

fn library_evt_receiver(engine: &Arc<Mutex<Option<Engine>>>) -> Option<library::EvtReceiver> {
    let guard = engine.lock().ok()?;
    let eng = guard.as_ref()?;
    let library = eng.library_manager()?;
    let buses = {
        let lib = match library.lock() {
            Ok(lib) => lib,
            Err(_) => {
                tracing::warn!("library lock poisoned; hot cue library evt sync disabled");
                return None;
            }
        };
        // No buses → engine has no performance-persist path; stay quiet.
        lib.buses()?
    };
    match buses.subscribe_evt_all() {
        Ok(rx) => Some(rx),
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to subscribe to library evt; hot cue ack sync disabled"
            );
            None
        }
    }
}

fn drain_library_evt(
    rx: &library::EvtReceiver,
    engine: &Arc<Mutex<Option<Engine>>>,
    evt_bus: &EngineBus,
    revision: &AtomicU64,
) {
    // omnibus `recv` is non-blocking (`try_recv`); drains only queued events.
    while let Ok(Some(event)) = rx.recv() {
        handle_library_evt(event.as_ref(), engine, evt_bus, revision);
    }
}

fn handle_library_evt(
    event: &omnibus::Event<library_api::Origin, LibraryKind, Arc<[u8]>>,
    engine: &Arc<Mutex<Option<Engine>>>,
    evt_bus: &EngineBus,
    revision: &AtomicU64,
) {
    match *event.kind() {
        LibraryKind::HotCuesChanged => {
            let (track_id, hot_cues) = match decode_library_evt_body(event.payload()) {
                Ok(LibraryEvtBody::HotCuesChanged { track_id, hot_cues }) => (track_id, hot_cues),
                Ok(_) => {
                    tracing::warn!("HotCuesChanged kind with mismatched library evt body");
                    return;
                }
                Err(err) => {
                    tracing::warn!(error = %err, "failed to decode library HotCuesChanged");
                    return;
                }
            };
            if let Err(err) = with_engine_mut(engine, |eng| {
                let changed = eng.apply_track_hot_cues(&track_id, &hot_cues);
                for deck_id in changed {
                    publish_deck_updated(evt_bus, revision, eng, deck_id)?;
                }
                Ok(())
            }) {
                tracing::warn!(error = %err, "failed to apply HotCuesChanged / publish DeckUpdated");
            }
        }
        LibraryKind::BeatGridChanged => {
            let (track_id, beat_grid) = match decode_library_evt_body(event.payload()) {
                Ok(LibraryEvtBody::BeatGridChanged {
                    track_id,
                    beat_grid,
                }) => (track_id, beat_grid),
                Ok(_) => {
                    tracing::warn!("BeatGridChanged kind with mismatched library evt body");
                    return;
                }
                Err(err) => {
                    tracing::warn!(error = %err, "failed to decode library BeatGridChanged");
                    return;
                }
            };
            if let Err(err) = with_engine_mut(engine, |eng| {
                let changed = eng.apply_track_beat_grid(&track_id, &beat_grid);
                for deck_id in changed {
                    publish_deck_updated(evt_bus, revision, eng, deck_id)?;
                }
                Ok(())
            }) {
                tracing::warn!(error = %err, "failed to apply BeatGridChanged / publish DeckUpdated");
            }
        }
        _ => {}
    }
}

fn with_engine_mut<F, R>(engine: &Arc<Mutex<Option<Engine>>>, f: F) -> Result<R>
where
    F: FnOnce(&mut Engine) -> Result<R>,
{
    let mut guard = engine.lock().unwrap();
    let eng = guard
        .as_mut()
        .ok_or_else(|| anyhow!("engine not available"))?;
    f(eng)
}

fn with_engine_ref<F, R>(engine: &Arc<Mutex<Option<Engine>>>, f: F) -> Result<R>
where
    F: FnOnce(&Engine) -> Result<R>,
{
    let guard = engine.lock().unwrap();
    let eng = guard
        .as_ref()
        .ok_or_else(|| anyhow!("engine not available"))?;
    f(eng)
}

fn bump_revision(revision: &AtomicU64) {
    revision.fetch_add(1, Ordering::Relaxed);
}

fn publish_evt(evt_bus: &EngineBus, origin: Origin, kind: Kind, body: EvtBody) {
    if let Ok(bytes) = encode_evt_body(&body) {
        let _ = evt_bus.publish(Event::new(origin, kind, Arc::from(bytes)));
    }
}

fn publish_error(evt_bus: &EngineBus, origin: Origin, message: impl Into<String>) {
    publish_evt(
        evt_bus,
        origin,
        Kind::Error,
        EvtBody::Error {
            message: message.into(),
        },
    );
}

/// Map a deck snapshot to the `DeckUpdated` evt body (host load paths + cmd worker).
pub fn deck_snapshot_to_evt(snap: DeckSnapshot) -> EvtBody {
    EvtBody::DeckUpdated {
        id: snap.id,
        track: snap.track,
        track_id: snap.track_id,
        title: snap.title,
        artist: snap.artist,
        album: snap.album,
        isrc: snap.isrc,
        bpm: snap.bpm,
        key: snap.key,
        playing: snap.playing,
        volume: snap.volume,
        speed: snap.speed,
        tempo_range: snap.tempo_range,
        key_lock: snap.key_lock,
        key_shift: snap.key_shift,
        keyboard_page: snap.keyboard_page,
        key_shift_page: snap.key_shift_page,
        keyboard_root_hot_cue: snap.keyboard_root_hot_cue,
        eq: snap.eq,
        filter: snap.filter,
        gain_trim: snap.gain_trim,
        fx_routed: snap.fx_routed,
        fx: snap.fx,
        headphone_cue: snap.headphone_cue,
        sync_mode: snap.sync_mode,
        cue_point_ms: snap.cue_point_ms,
        quantize: snap.quantize,
        active_loop: snap.active_loop,
        pending_loop_in_ms: snap.pending_loop_in_ms,
        slip_enabled: snap.slip_enabled,
        slip_shadow_position_ms: snap.slip_shadow_position_ms,
        pad_mode: snap.pad_mode,
        stems_ready: snap.stems_ready,
        stem_mute: snap.stem_mute,
        stem_isolate: snap.stem_isolate,
        position_ms: snap.position_ms,
        duration_ms: snap.duration_ms,
        hot_cues: snap.hot_cues,
        saved_loops: snap.saved_loops,
        loudness_lufs: snap.loudness_lufs,
        auto_gain_db: snap.auto_gain_db,
        active_sampler_bank_id: snap.active_sampler_bank_id,
        top_jog_mode: snap.top_jog_mode,
        outer_jog_mode: snap.outer_jog_mode,
        jog_touching: snap.jog_touching,
    }
}

fn publish_deck_updated(
    evt_bus: &EngineBus,
    revision: &AtomicU64,
    engine: &Engine,
    deck_id: usize,
) -> Result<()> {
    let snap = engine
        .deck_snapshot(deck_id)
        .ok_or_else(|| anyhow!("deck snapshot unavailable"))?;
    bump_revision(revision);
    publish_evt(
        evt_bus,
        Origin::Deck(deck_id as u16),
        Kind::Updated,
        deck_snapshot_to_evt(snap),
    );
    Ok(())
}

fn handle_cmd_event(
    event: &Event<Origin, Kind, Arc<[u8]>>,
    engine: &Arc<Mutex<Option<Engine>>>,
    evt_bus: &EngineBus,
    revision: &AtomicU64,
) {
    let origin = event.origin().clone();
    let kind = event.kind().clone();
    let payload = event.payload();

    let result = match origin.clone() {
        Origin::Deck(deck_id) => dispatch_deck_cmd(deck_id as usize, kind, payload, engine),
        Origin::Mixer => dispatch_mixer_cmd(kind, payload, engine),
        Origin::Engine => Err(anyhow!("unsupported origin on cmd bus")),
    };

    match result {
        Ok(CmdOutcome::DeckUpdated(deck_id)) => {
            if let Err(e) = with_engine_ref(engine, |eng| {
                publish_deck_updated(evt_bus, revision, eng, deck_id)
            }) {
                publish_error(evt_bus, origin, format!("{e:#}"));
            }
        }
        Ok(CmdOutcome::DecksUpdated(deck_ids)) => {
            for deck_id in deck_ids {
                if let Err(e) = with_engine_ref(engine, |eng| {
                    publish_deck_updated(evt_bus, revision, eng, deck_id)
                }) {
                    publish_error(evt_bus, origin.clone(), format!("{e:#}"));
                }
            }
        }
        Ok(CmdOutcome::EngineStatus) => {
            let _ = with_engine_ref(engine, |eng| {
                if let Some(status) = eng.engine_status_snapshot() {
                    bump_revision(revision);
                    publish_evt(
                        evt_bus,
                        Origin::Mixer,
                        Kind::Status,
                        EvtBody::EngineStatus { status },
                    );
                }
                Ok(())
            });
        }
        Ok(CmdOutcome::Position(deck_id)) => {
            let _ = with_engine_ref(engine, |eng| {
                if let Some((position_ms, _)) = eng.deck_playback_ms(deck_id) {
                    let slip_shadow_position_ms = eng
                        .deck_slip_enabled(deck_id)
                        .filter(|enabled| *enabled)
                        .and_then(|_| eng.deck_slip_shadow_position_ms(deck_id));
                    publish_evt(
                        evt_bus,
                        Origin::Deck(deck_id as u16),
                        Kind::Position,
                        EvtBody::Position {
                            position_ms,
                            slip_shadow_position_ms,
                        },
                    );
                }
                Ok(())
            });
        }
        Ok(CmdOutcome::Silent) => {}
        Err(e) => publish_error(evt_bus, origin, format!("{e:#}")),
    }
}

fn decode_cmd_body_for(kind: Kind, payload: &[u8]) -> Result<CmdBody> {
    let body = decode_cmd_body(payload).map_err(|e| anyhow!("invalid cmd body: {e}"))?;
    match (&kind, &body) {
        (
            Kind::Play
            | Kind::Pause
            | Kind::TogglePlay
            | Kind::ToggleQuantize
            | Kind::ToggleSlip
            | Kind::ToggleHeadphoneCue
            | Kind::ToggleMasterCue
            | Kind::SetMasterDeck
            | Kind::Unload
            | Kind::SetCuePoint
            | Kind::BeginCueHold
            | Kind::EndCueHold
            | Kind::ExitLoop
            | Kind::EndLoopRoll,
            CmdBody::Empty,
        ) => Ok(body),
        (Kind::Seek, CmdBody::Seek { .. })
        | (Kind::SetVolume, CmdBody::SetVolume { .. })
        | (Kind::SetEq, CmdBody::SetEq { .. })
        | (Kind::SetEqBand, CmdBody::SetEqBand { .. })
        | (Kind::SetSpeed, CmdBody::SetSpeed { .. })
        | (Kind::SetTempoRange, CmdBody::SetTempoRange { .. })
        | (Kind::SetKeyLock, CmdBody::SetKeyLock { .. })
        | (Kind::SetKeyShift, CmdBody::SetKeyShift { .. })
        | (Kind::SetKeyboardRoot, CmdBody::SetKeyboardRoot { .. })
        | (Kind::SetKeyboardPage, CmdBody::SetKeyboardPage { .. })
        | (Kind::SetKeyShiftPage, CmdBody::SetKeyShiftPage { .. })
        | (Kind::SetFilter, CmdBody::SetFilter { .. })
        | (Kind::SetGainTrim, CmdBody::SetGainTrim { .. })
        | (Kind::SetFxUnitSlot, CmdBody::SetFxUnitSlot { .. })
        | (Kind::SetFxUnitSlotEnabled, CmdBody::SetFxUnitSlotEnabled { .. })
        | (Kind::SetFxUnitWet, CmdBody::SetFxUnitWet { .. })
        | (Kind::SetFxUnitParam, CmdBody::SetFxUnitParam { .. })
        | (Kind::SetDeckFxRouted, CmdBody::SetDeckFxRouted { .. })
        | (Kind::SetHeadphoneCue, CmdBody::SetHeadphoneCue { .. })
        | (Kind::ToggleSync, CmdBody::ToggleSync { .. })
        | (Kind::SetQuantize, CmdBody::SetQuantize { .. })
        | (Kind::SetSlip, CmdBody::SetSlip { .. })
        | (Kind::SetAutoLoop, CmdBody::SetAutoLoop { .. })
        | (Kind::LoopIn, CmdBody::LoopIn { .. })
        | (Kind::LoopOut, CmdBody::LoopOut { .. })
        | (Kind::BeatJump, CmdBody::BeatJump { .. })
        | (Kind::SetPadMode, CmdBody::SetPadMode { .. })
        | (Kind::BeginLoopRoll, CmdBody::BeginLoopRoll { .. })
        | (Kind::PadPress, CmdBody::PadPress { .. })
        | (Kind::PadRelease, CmdBody::PadRelease { .. })
        | (Kind::TriggerHotCue, CmdBody::TriggerHotCue { .. })
        | (Kind::RecallSavedLoop, CmdBody::RecallSavedLoop { .. })
        | (Kind::SaveHotCue, CmdBody::SaveHotCue { .. })
        | (Kind::DeleteHotCue, CmdBody::DeleteHotCue { .. })
        | (Kind::HotCuePadPress, CmdBody::HotCuePadPress { .. })
        | (Kind::HotCuePadRelease, CmdBody::HotCuePadRelease { .. })
        | (Kind::LoopRollPadPress, CmdBody::LoopRollPadPress { .. })
        | (Kind::LoopRollPadRelease, CmdBody::LoopRollPadRelease { .. })
        | (Kind::BeatJumpPadPress, CmdBody::BeatJumpPadPress { .. })
        | (Kind::BeatJumpPadRelease, CmdBody::BeatJumpPadRelease { .. })
        | (Kind::SamplerPadPress, CmdBody::SamplerPadPress { .. })
        | (Kind::SamplerPadRelease, CmdBody::SamplerPadRelease { .. })
        | (Kind::KeyboardPadPress, CmdBody::KeyboardPadPress { .. })
        | (Kind::KeyboardPadRelease, CmdBody::KeyboardPadRelease { .. })
        | (Kind::KeyShiftPadPress, CmdBody::KeyShiftPadPress { .. })
        | (Kind::KeyShiftPadRelease, CmdBody::KeyShiftPadRelease { .. })
        | (Kind::AssignSampler, CmdBody::AssignSampler { .. })
        | (Kind::AssignSamplerTrack, CmdBody::AssignSamplerTrack { .. })
        | (Kind::ClearSampler, CmdBody::ClearSampler { .. })
        | (Kind::SetCrossfader, CmdBody::SetCrossfader { .. })
        | (Kind::SetMasterVolume, CmdBody::SetMasterVolume { .. })
        | (Kind::SetCueVolume, CmdBody::SetCueVolume { .. })
        | (Kind::SetCueMix, CmdBody::SetCueMix { .. })
        | (Kind::SetMasterCue, CmdBody::SetMasterCue { .. })
        | (Kind::JogTouch, CmdBody::JogTouch { .. })
        | (Kind::JogTurn, CmdBody::JogTurn { .. })
        | (Kind::SetJogMode, CmdBody::SetJogMode { .. }) => Ok(body),
        _ => Err(anyhow!("cmd body does not match kind {kind:?}")),
    }
}

fn dispatch_deck_cmd(
    deck_id: usize,
    kind: Kind,
    payload: &[u8],
    engine: &Arc<Mutex<Option<Engine>>>,
) -> Result<CmdOutcome> {
    if kind == Kind::AssignSampler {
        return dispatch_assign_sampler(deck_id, payload, engine);
    }
    if kind == Kind::AssignSamplerTrack {
        return dispatch_assign_sampler_track(deck_id, payload, engine);
    }
    with_engine_mut(engine, |eng| match kind {
        Kind::Play => {
            if !eng.deck_has_audio_loaded(deck_id).unwrap_or(false) {
                return Err(anyhow!("No track loaded"));
            }
            eng.play(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::Pause => {
            eng.pause(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::TogglePlay => {
            let _ = decode_cmd_body_for(kind, payload)?;
            if eng.deck_is_playing(deck_id).unwrap_or(false) {
                eng.pause(deck_id)?;
            } else {
                if !eng.deck_has_audio_loaded(deck_id).unwrap_or(false) {
                    return Err(anyhow!("No track loaded"));
                }
                eng.play(deck_id)?;
            }
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::Seek => {
            let CmdBody::Seek { position_ms } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.seek_deck(deck_id, position_ms)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetVolume => {
            let CmdBody::SetVolume {
                volume,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_deck(deck_id, "volume");
            let current = eng
                .deck_strip_norms(deck_id)
                .map(|s| s.volume)
                .unwrap_or(0.0);
            if !gate_mixer_soft_takeover(eng, &key, soft_takeover, current, volume) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_volume(deck_id, volume)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetEq => {
            let CmdBody::SetEq { low, mid, high } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_eq_bands(
                deck_id,
                crate::control_norm::norm_to_strip_db(low),
                crate::control_norm::norm_to_strip_db(mid),
                crate::control_norm::norm_to_strip_db(high),
            )?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetEqBand => {
            let CmdBody::SetEqBand {
                band,
                gain,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let strip = eng.deck_strip_norms(deck_id);
            let (ctrl, current, low, mid, high) = match (band, strip) {
                (EqBand::Low, Some(s)) => ("eq_low", s.eq_low, gain, s.eq_mid, s.eq_high),
                (EqBand::Mid, Some(s)) => ("eq_mid", s.eq_mid, s.eq_low, gain, s.eq_high),
                (EqBand::High, Some(s)) => ("eq_high", s.eq_high, s.eq_low, s.eq_mid, gain),
                (EqBand::Low, None) => ("eq_low", 0.5, gain, 0.5, 0.5),
                (EqBand::Mid, None) => ("eq_mid", 0.5, 0.5, gain, 0.5),
                (EqBand::High, None) => ("eq_high", 0.5, 0.5, 0.5, gain),
            };
            let key = soft_takeover::key_deck(deck_id, ctrl);
            if !eng
                .soft_takeover_mut()
                .allow(&key, soft_takeover, current, gain)
            {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_eq_bands(
                deck_id,
                crate::control_norm::norm_to_strip_db(low),
                crate::control_norm::norm_to_strip_db(mid),
                crate::control_norm::norm_to_strip_db(high),
            )?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetSpeed => {
            let CmdBody::SetSpeed {
                speed,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_deck(deck_id, "speed");
            let current = eng
                .deck_strip_norms(deck_id)
                .map(|s| s.speed)
                .unwrap_or(0.5);
            if !eng
                .soft_takeover_mut()
                .allow(&key, soft_takeover, current, speed)
            {
                return Ok(CmdOutcome::Silent);
            }
            let updated = eng.set_deck_speed(deck_id, speed.clamp(0.0, 1.0))?;
            Ok(CmdOutcome::DecksUpdated(updated))
        }
        Kind::SetTempoRange => {
            let CmdBody::SetTempoRange { tempo_range } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            let updated = eng.set_deck_tempo_range(deck_id, tempo_range)?;
            Ok(CmdOutcome::DecksUpdated(updated))
        }
        Kind::SetKeyLock => {
            let CmdBody::SetKeyLock { enabled } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_key_lock(deck_id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetKeyShift => {
            let CmdBody::SetKeyShift { semitones } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_key_shift(deck_id, semitones)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetKeyboardRoot => {
            let CmdBody::SetKeyboardRoot { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_keyboard_root(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetKeyboardPage => {
            let CmdBody::SetKeyboardPage { page } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_keyboard_page(deck_id, page)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetKeyShiftPage => {
            let CmdBody::SetKeyShiftPage { page } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_key_shift_page(deck_id, page)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetFilter => {
            let CmdBody::SetFilter {
                filter,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_deck(deck_id, "filter");
            let current = eng
                .deck_strip_norms(deck_id)
                .map(|s| s.filter)
                .unwrap_or(0.5);
            if !eng
                .soft_takeover_mut()
                .allow(&key, soft_takeover, current, filter)
            {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_filter_db(deck_id, crate::control_norm::norm_to_strip_db(filter))?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetGainTrim => {
            let CmdBody::SetGainTrim {
                gain_trim,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_deck(deck_id, "gain");
            let current = eng
                .deck_strip_norms(deck_id)
                .map(|s| s.gain_trim)
                .unwrap_or(0.5);
            if !eng
                .soft_takeover_mut()
                .allow(&key, soft_takeover, current, gain_trim)
            {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_gain_trim_db(deck_id, crate::control_norm::norm_to_strip_db(gain_trim))?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetDeckFxRouted => {
            let CmdBody::SetDeckFxRouted { routed } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_fx_routed(deck_id, routed)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetHeadphoneCue => {
            let CmdBody::SetHeadphoneCue { enabled } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_headphone_cue(deck_id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::ToggleHeadphoneCue => {
            let _ = decode_cmd_body_for(kind, payload)?;
            let enabled = !eng
                .deck_strip_norms(deck_id)
                .map(|s| s.headphone_cue)
                .unwrap_or(false);
            eng.set_deck_headphone_cue(deck_id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::ToggleSync => {
            let CmdBody::ToggleSync { beat_sync } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            let updated = eng.toggle_deck_sync(deck_id, beat_sync)?;
            Ok(CmdOutcome::DecksUpdated(updated))
        }
        Kind::SetMasterDeck => {
            let _ = decode_cmd_body_for(kind, payload)?;
            let _updated = eng.set_master_deck(deck_id)?;
            // master_deck + slave speeds/sync live on EngineStatus decks.
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::Unload => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.unload_deck(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetCuePoint => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.set_deck_cue_point_at_playhead(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::BeginCueHold => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.begin_deck_cue_hold(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::EndCueHold => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.end_deck_cue_hold(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetQuantize => {
            let CmdBody::SetQuantize { enabled } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_quantize(deck_id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::ToggleQuantize => {
            let _ = decode_cmd_body_for(kind, payload)?;
            let enabled = !eng.deck_quantize(deck_id).unwrap_or(true);
            eng.set_deck_quantize(deck_id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetSlip => {
            let CmdBody::SetSlip { enabled } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_slip(deck_id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::ToggleSlip => {
            let _ = decode_cmd_body_for(kind, payload)?;
            let enabled = !eng.deck_slip_enabled(deck_id).unwrap_or(false);
            eng.set_deck_slip(deck_id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetAutoLoop => {
            let CmdBody::SetAutoLoop { beats } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_auto_loop(deck_id, beats)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::LoopIn => {
            let CmdBody::LoopIn { position_ms } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_loop_in(deck_id, position_ms)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::LoopOut => {
            let CmdBody::LoopOut { position_ms } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_loop_out(deck_id, position_ms)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::ExitLoop => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.clear_deck_loop(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::BeatJump => {
            let CmdBody::BeatJump { beats } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.beat_jump_deck(deck_id, beats)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetPadMode => {
            let CmdBody::SetPadMode { mode } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_pad_mode(deck_id, mode)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::BeginLoopRoll => {
            let CmdBody::BeginLoopRoll { beats } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.begin_deck_loop_roll(deck_id, beats)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::EndLoopRoll => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.end_deck_loop_roll(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::PadPress => {
            let CmdBody::PadPress { slot, shift } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.pad_press(deck_id, slot, shift)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::PadRelease => {
            let CmdBody::PadRelease { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.pad_release(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::TriggerHotCue => {
            let CmdBody::TriggerHotCue { position_ms } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.trigger_deck_hot_cue(deck_id, position_ms)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SaveHotCue => {
            let CmdBody::SaveHotCue { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.save_deck_hot_cue(deck_id, slot)?;
            Ok(CmdOutcome::Silent)
        }
        Kind::DeleteHotCue => {
            let CmdBody::DeleteHotCue { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.delete_deck_hot_cue(deck_id, slot)?;
            Ok(CmdOutcome::Silent)
        }
        Kind::HotCuePadPress => {
            let CmdBody::HotCuePadPress { slot, shift } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            eng.hot_cue_pad_press(deck_id, slot, shift)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::HotCuePadRelease => {
            let CmdBody::HotCuePadRelease { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.hot_cue_pad_release(deck_id, slot)?;
            Ok(CmdOutcome::Silent)
        }
        Kind::LoopRollPadPress => {
            let CmdBody::LoopRollPadPress { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.loop_roll_pad_press(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::LoopRollPadRelease => {
            let CmdBody::LoopRollPadRelease { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.loop_roll_pad_release(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::BeatJumpPadPress => {
            let CmdBody::BeatJumpPadPress { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.beat_jump_pad_press(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::BeatJumpPadRelease => {
            let CmdBody::BeatJumpPadRelease { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.beat_jump_pad_release(deck_id, slot)?;
            Ok(CmdOutcome::Silent)
        }
        Kind::SamplerPadPress => {
            let CmdBody::SamplerPadPress { slot, shift } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            eng.sampler_pad_press(deck_id, slot, shift)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SamplerPadRelease => {
            let CmdBody::SamplerPadRelease { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.sampler_pad_release(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::KeyboardPadPress => {
            let CmdBody::KeyboardPadPress { slot, shift } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            eng.keyboard_pad_press(deck_id, slot, shift)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::KeyboardPadRelease => {
            let CmdBody::KeyboardPadRelease { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.keyboard_pad_release(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::KeyShiftPadPress => {
            let CmdBody::KeyShiftPadPress { slot, shift } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            eng.key_shift_pad_press(deck_id, slot, shift)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::KeyShiftPadRelease => {
            let CmdBody::KeyShiftPadRelease { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.key_shift_pad_release(deck_id, slot)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::ClearSampler => {
            let CmdBody::ClearSampler { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.clear_sampler_slot(deck_id, slot as usize)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::RecallSavedLoop => {
            let CmdBody::RecallSavedLoop { in_ms, out_ms } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            eng.recall_deck_saved_loop(deck_id, in_ms, out_ms)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::JogTouch => {
            let CmdBody::JogTouch { touching } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_jog_touch(deck_id, touching)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::JogTurn => {
            let CmdBody::JogTurn { delta } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.deck_jog_turn(deck_id, delta)?;
            // So paused vinyl scrub updates UI playhead/time without waiting for
            // the next 33ms tick (JogTouch release used to be the only cue).
            Ok(CmdOutcome::Position(deck_id))
        }
        Kind::SetJogMode => {
            let CmdBody::SetJogMode { top, outer } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_jog_mode(deck_id, top, outer)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        _ => Err(anyhow!("unsupported kind on cmd bus")),
    })
}

fn dispatch_assign_sampler(
    deck_id: usize,
    payload: &[u8],
    engine: &Arc<Mutex<Option<Engine>>>,
) -> Result<CmdOutcome> {
    let kind = Kind::AssignSampler;
    let CmdBody::AssignSampler { slot, path } = decode_cmd_body_for(kind, payload)? else {
        unreachable!()
    };
    let library = with_engine_ref(engine, |eng| {
        eng.library_manager()
            .ok_or_else(|| anyhow!("Engine has no attached LibraryManager"))
    })?;
    let prepared: PreparedTrackPlayback =
        LibraryManager::prepare_file_path_for_playback(library.as_ref(), Path::new(&path))
            .map_err(|e| anyhow!(e.to_string()))?;
    with_engine_mut(engine, |eng| {
        eng.assign_prepared_sampler(deck_id, slot as usize, prepared)?;
        Ok(CmdOutcome::DeckUpdated(deck_id))
    })
}

fn dispatch_assign_sampler_track(
    deck_id: usize,
    payload: &[u8],
    engine: &Arc<Mutex<Option<Engine>>>,
) -> Result<CmdOutcome> {
    let kind = Kind::AssignSamplerTrack;
    let CmdBody::AssignSamplerTrack { slot, track_id } = decode_cmd_body_for(kind, payload)? else {
        unreachable!()
    };
    let library = with_engine_ref(engine, |eng| {
        eng.library_manager()
            .ok_or_else(|| anyhow!("Engine has no attached LibraryManager"))
    })?;
    let prepared: PreparedTrackPlayback =
        LibraryManager::prepare_track_for_playback(library.as_ref(), &TrackId::new(track_id))
            .map_err(|e| anyhow!(e.to_string()))?;
    with_engine_mut(engine, |eng| {
        eng.assign_prepared_sampler(deck_id, slot as usize, prepared)?;
        Ok(CmdOutcome::DeckUpdated(deck_id))
    })
}

/// Shared soft-takeover gate for mixer-origin absolute controls.
///
/// Adopt-first when unseen, then block until the control catches up; keeping
/// the policy in one place stops a new mixer knob from drifting from the rest.
fn gate_mixer_soft_takeover(
    eng: &mut Engine,
    key: &str,
    soft_takeover: bool,
    current: f32,
    requested: f32,
) -> bool {
    eng.soft_takeover_mut()
        .allow(key, soft_takeover, current, requested)
}

fn dispatch_mixer_cmd(
    kind: Kind,
    payload: &[u8],
    engine: &Arc<Mutex<Option<Engine>>>,
) -> Result<CmdOutcome> {
    with_engine_mut(engine, |eng| match kind {
        Kind::SetCrossfader => {
            let CmdBody::SetCrossfader {
                position,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_mixer("crossfader");
            let current = eng.crossfader().unwrap_or(0.5);
            if !gate_mixer_soft_takeover(eng, &key, soft_takeover, current, position) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_crossfader(position)?;
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::SetMasterVolume => {
            let CmdBody::SetMasterVolume {
                volume,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_mixer("master_volume");
            let current = eng.master_volume().unwrap_or(1.0);
            if !gate_mixer_soft_takeover(eng, &key, soft_takeover, current, volume) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_master_volume(volume)?;
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::SetCueVolume => {
            let CmdBody::SetCueVolume {
                volume,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_mixer("cue_volume");
            let current = eng.cue_volume().unwrap_or(1.0);
            if !gate_mixer_soft_takeover(eng, &key, soft_takeover, current, volume) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_cue_volume(volume)?;
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::SetCueMix => {
            let CmdBody::SetCueMix { mix, soft_takeover } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_mixer("cue_mix");
            let current = eng.cue_mix().unwrap_or(0.5);
            if !gate_mixer_soft_takeover(eng, &key, soft_takeover, current, mix) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_cue_mix(mix)?;
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::SetMasterCue => {
            let CmdBody::SetMasterCue { enabled } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_master_cue(enabled)?;
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::ToggleMasterCue => {
            let _ = decode_cmd_body_for(kind, payload)?;
            let enabled = !eng.master_cue().unwrap_or(false);
            eng.set_master_cue(enabled)?;
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::SetFxUnitSlot => {
            let CmdBody::SetFxUnitSlot { slot, kind } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_fx_unit_slot(slot, crate::status::map_fx_kind(kind))?;
            Ok(CmdOutcome::DecksUpdated((0..NUM_DECKS).collect()))
        }
        Kind::SetFxUnitSlotEnabled => {
            let CmdBody::SetFxUnitSlotEnabled { slot, enabled } =
                decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            eng.set_fx_unit_slot_enabled(slot, enabled)?;
            Ok(CmdOutcome::DecksUpdated((0..NUM_DECKS).collect()))
        }
        Kind::SetFxUnitWet => {
            let CmdBody::SetFxUnitWet {
                slot,
                wet,
                soft_takeover,
            } = decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            let key = soft_takeover::key_mixer(&format!("fx_wet{slot}"));
            let current = eng.fx_unit_wet(slot).unwrap_or(0.5);
            if !gate_mixer_soft_takeover(eng, &key, soft_takeover, current, wet) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_fx_unit_wet(slot, wet)?;
            Ok(CmdOutcome::DecksUpdated((0..NUM_DECKS).collect()))
        }
        Kind::SetFxUnitParam => {
            let CmdBody::SetFxUnitParam { slot, param, value } =
                decode_cmd_body_for(kind, payload)?
            else {
                unreachable!()
            };
            eng.set_fx_unit_param(slot, param, value)?;
            Ok(CmdOutcome::DecksUpdated((0..NUM_DECKS).collect()))
        }
        _ => Err(anyhow!("unsupported kind on cmd bus")),
    })
}

fn tick(
    engine: &Arc<Mutex<Option<Engine>>>,
    evt_bus: &EngineBus,
    revision: &AtomicU64,
    peak_hold: &mut PeakHoldState,
) {
    let mut guard = match engine.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let eng = match guard.as_mut() {
        Some(engine) => engine,
        None => return,
    };

    let transport_events = eng.drain_transport_events();
    for TransportEvent::TrackEnded { deck_id } in transport_events {
        let _ = publish_deck_updated(evt_bus, revision, eng, deck_id);
    }

    // A live stem producer publishes its first window asynchronously; emit one
    // Updated so pads/banner reflect readiness without waiting for another cmd.
    for deck_id in 0..NUM_DECKS {
        let ready = eng.deck_stems_ready(deck_id).unwrap_or(false);
        if ready != eng.stems_ready_seen[deck_id] {
            eng.stems_ready_seen[deck_id] = ready;
            let _ = publish_deck_updated(evt_bus, revision, eng, deck_id);
        }
    }

    let playback = eng.deck_playback_snapshot();
    for (deck_id, position, _duration) in playback {
        // Vinyl scratch while paused still advances the playhead; UI time/waveform
        // need Position ticks, not only DeckUpdated on jog-touch release.
        let playing = eng.deck_is_playing(deck_id) == Some(true);
        let jog_driving = eng.deck_jog_driving_audio(deck_id) == Some(true);
        if playing || jog_driving {
            let slip_shadow_position_ms = eng
                .deck_slip_enabled(deck_id)
                .filter(|enabled| *enabled)
                .and_then(|_| eng.deck_slip_shadow_position_ms(deck_id));
            publish_evt(
                evt_bus,
                Origin::Deck(deck_id as u16),
                Kind::Position,
                EvtBody::Position {
                    position_ms: position,
                    slip_shadow_position_ms,
                },
            );
        }
    }

    for (deck_id, peak_l, peak_r) in eng.deck_level_snapshot() {
        let playing = eng.deck_is_playing(deck_id) == Some(true);
        // Always run ballistics first so pause can decay hold; only stop publishing
        // once peaks and hold are fully idle (otherwise UI meters stick).
        let (peak_hold_l, peak_hold_r) = peak_hold.update(deck_id, peak_l, peak_r);
        if !should_publish_levels(playing, peak_l, peak_r, peak_hold_l, peak_hold_r) {
            if peak_hold.take_zero_flush(deck_id) {
                publish_evt(
                    evt_bus,
                    Origin::Deck(deck_id as u16),
                    Kind::Levels,
                    EvtBody::Levels {
                        peak_l: 0.0,
                        peak_r: 0.0,
                        peak_hold_l: 0.0,
                        peak_hold_r: 0.0,
                    },
                );
            }
            continue;
        }
        peak_hold.mark_published(deck_id, false);
        publish_evt(
            evt_bus,
            Origin::Deck(deck_id as u16),
            Kind::Levels,
            EvtBody::Levels {
                peak_l,
                peak_r,
                peak_hold_l,
                peak_hold_r,
            },
        );
    }

    // Master bus peaks (post fader). Hold slot sits above deck indices.
    const MASTER_HOLD_SLOT: usize = NUM_DECKS;
    if let Some((peak_l, peak_r)) = eng.master_level_snapshot() {
        let any_playing = (0..NUM_DECKS).any(|deck_id| eng.deck_is_playing(deck_id) == Some(true));
        let (peak_hold_l, peak_hold_r) = peak_hold.update(MASTER_HOLD_SLOT, peak_l, peak_r);
        if !should_publish_levels(any_playing, peak_l, peak_r, peak_hold_l, peak_hold_r) {
            if peak_hold.take_zero_flush(MASTER_HOLD_SLOT) {
                publish_evt(
                    evt_bus,
                    Origin::Mixer,
                    Kind::Levels,
                    EvtBody::Levels {
                        peak_l: 0.0,
                        peak_r: 0.0,
                        peak_hold_l: 0.0,
                        peak_hold_r: 0.0,
                    },
                );
            }
        } else {
            peak_hold.mark_published(MASTER_HOLD_SLOT, false);
            publish_evt(
                evt_bus,
                Origin::Mixer,
                Kind::Levels,
                EvtBody::Levels {
                    peak_l,
                    peak_r,
                    peak_hold_l,
                    peak_hold_r,
                },
            );
        }
    }
}

fn should_publish_levels(
    playing: bool,
    peak_l: f32,
    peak_r: f32,
    hold_l: f32,
    hold_r: f32,
) -> bool {
    playing
        || peak_l.abs() >= LEVEL_IDLE_EPSILON
        || peak_r.abs() >= LEVEL_IDLE_EPSILON
        || hold_l >= LEVEL_IDLE_EPSILON
        || hold_r >= LEVEL_IDLE_EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_silent_peaks_keep_publishing_until_hold_decays() {
        let mut hold = PeakHoldState::new();
        let (hl, hr) = hold.update(0, 0.8, 0.6);
        assert!(hl >= 0.8 && hr >= 0.6);

        let mut published = 0u32;
        for _ in 0..100 {
            let (hl, hr) = hold.update(0, 0.0, 0.0);
            if !should_publish_levels(false, 0.0, 0.0, hl, hr) {
                break;
            }
            published += 1;
        }
        assert!(
            published > 5,
            "hold should take multiple ticks to decay, got {published} publishes"
        );
        let (hl, hr) = hold.update(0, 0.0, 0.0);
        assert!(
            !should_publish_levels(false, 0.0, 0.0, hl, hr),
            "fully decayed hold should stop publishing"
        );
    }

    #[test]
    fn playing_always_publishes_even_when_silent() {
        assert!(should_publish_levels(true, 0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn idle_deck_flushes_zeros_once() {
        let mut hold = PeakHoldState::new();
        let _ = hold.update(0, 0.5, 0.5);
        hold.mark_published(0, false);
        assert!(hold.take_zero_flush(0));
        assert!(!hold.take_zero_flush(0));
    }
}
