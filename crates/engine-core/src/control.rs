//! Control thread: cmd dispatch and high-rate evt egress.

use crate::engine::Engine;
use crate::soft_takeover;
use crate::transport::TransportEvent;
use anyhow::anyhow;
use anyhow::Result;
use bus_api::DeckLevels;
use bus_api::DeckPosition;
use bus_api::DeckUpdated;
use bus_api::EngineSnapshot;
use bus_api::EqBand;
use bus_api::Event;
use event_bus::EventBus;
use event_bus::Subscriber;
use library::{LibraryManager, PreparedTrackPlayback};
use library_core::TrackId;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const PEAK_HOLD_DECAY_PER_TICK: f32 = 0.04;
const LEVEL_IDLE_EPSILON: f32 = 1e-5;

/// Engine handle shared with the host (the engine can be stopped/restarted).
type EngineHandle = Arc<Mutex<Option<Engine>>>;

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
        // Deck slots plus the master slot; growth handles four-deck engines.
        let slots = 3;
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

/// Spawn the cmd-draining worker using the bus attached to `engine`.
///
/// Errors if [`Engine::set_buses`] was not called, or if the thread fails to subscribe.
pub fn spawn_engine_worker(engine: EngineHandle) -> Result<EngineWorker> {
    let bus = {
        let guard = engine.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().and_then(Engine::buses).ok_or_else(|| {
            anyhow!("engine buses not attached; call set_buses before spawn_engine_worker")
        })?
    };

    let shutdown = Arc::new(AtomicBool::new(false));
    let engine_handle = Arc::clone(&engine);
    let shutdown_flag = Arc::clone(&shutdown);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();
    let handle = std::thread::spawn(move || {
        control_thread_loop(bus, engine_handle, shutdown_flag, ready_tx);
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
    bus: EventBus<Event>,
    engine: EngineHandle,
    shutdown: Arc<AtomicBool>,
    ready: std::sync::mpsc::Sender<Result<()>>,
) {
    let mut cmd_rx = match bus.subscribe_cmd() {
        Ok(rx) => rx,
        Err(e) => {
            let _ = ready.send(Err(e.into()));
            return;
        }
    };
    // Best-effort at control-thread start: subscribe to library HotCuesChanged so
    // deck_control.hot_cues commits only after DB ack. Hosts attach buses before
    // spawn; no retry if subscribe fails or library has no buses.
    let mut library_evt_rx = library_evt_receiver(&engine);
    let _ = ready.send(Ok(()));
    let mut peak_hold = PeakHoldState::new();

    while !shutdown.load(Ordering::Relaxed) {
        match cmd_rx.recv_timeout(TICK_INTERVAL) {
            Ok(Some(event)) => handle_cmd_event(event, &engine, &bus),
            Ok(None) => {}
            Err(_) => break,
        }
        if let Some(lib_rx) = library_evt_rx.as_mut() {
            drain_library_evt(lib_rx, &engine, &bus);
        }
        tick(&engine, &bus, &mut peak_hold);
    }
}

fn library_evt_receiver(engine: &EngineHandle) -> Option<Subscriber<Event>> {
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
    match buses.subscribe_evt() {
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

fn drain_library_evt(rx: &mut Subscriber<Event>, engine: &EngineHandle, bus: &EventBus<Event>) {
    // `try_recv` is non-blocking and folds the pending queue; drains only queued events.
    while let Some(event) = rx.try_recv() {
        handle_library_evt(&event, engine, bus);
    }
}

fn handle_library_evt(event: &Event, engine: &EngineHandle, bus: &EventBus<Event>) {
    match event {
        Event::HotCuesChanged(changed) => {
            let track_id = changed.track_id.clone();
            let hot_cues = changed.hot_cues.clone();
            if let Err(err) = with_engine_mut(engine, |eng| {
                let changed = eng.apply_track_hot_cues(&track_id, &hot_cues);
                for deck_id in changed {
                    publish_deck_updated(bus, eng, deck_id)?;
                }
                Ok(())
            }) {
                tracing::warn!(error = %err, "failed to apply HotCuesChanged / publish DeckUpdated");
            }
        }
        Event::BeatGridChanged(changed) => {
            let track_id = changed.track_id.clone();
            let beat_grid = changed.beat_grid.clone();
            if let Err(err) = with_engine_mut(engine, |eng| {
                let changed = eng.apply_track_beat_grid(&track_id, &beat_grid);
                for deck_id in changed {
                    publish_deck_updated(bus, eng, deck_id)?;
                }
                Ok(())
            }) {
                tracing::warn!(error = %err, "failed to apply BeatGridChanged / publish DeckUpdated");
            }
        }
        _ => {}
    }
}

fn with_engine_mut<F, R>(engine: &EngineHandle, f: F) -> Result<R>
where
    F: FnOnce(&mut Engine) -> Result<R>,
{
    let mut guard = engine.lock().expect("engine handle mutex poisoned");
    let eng = guard
        .as_mut()
        .ok_or_else(|| anyhow!("engine not available"))?;
    f(eng)
}

fn with_engine_ref<F, R>(engine: &EngineHandle, f: F) -> Result<R>
where
    F: FnOnce(&Engine) -> Result<R>,
{
    let guard = engine.lock().expect("engine handle mutex poisoned");
    let eng = guard
        .as_ref()
        .ok_or_else(|| anyhow!("engine not available"))?;
    f(eng)
}

/// Fire-and-forget egress publish from the control thread.
fn publish(bus: &EventBus<Event>, event: Event) {
    // A dead bus only means no host is listening; the control loop keeps running.
    let _ = bus.publish_evt(event);
}

fn publish_error(bus: &EventBus<Event>, deck: Option<u16>, message: impl Into<String>) {
    publish(
        bus,
        Event::Error(bus_api::Error {
            message: message.into(),
            track_id: None,
            deck,
        }),
    );
}

fn publish_deck_updated(bus: &EventBus<Event>, engine: &Engine, deck_id: usize) -> Result<()> {
    let snap = engine
        .deck_snapshot(deck_id)
        .ok_or_else(|| anyhow!("deck snapshot unavailable"))?;
    publish(bus, Event::DeckUpdated(DeckUpdated::from(snap)));
    Ok(())
}

fn publish_engine_status(bus: &EventBus<Event>, engine: &Engine) {
    if let Some(status) = engine.engine_status_snapshot() {
        publish(bus, Event::EngineSnapshot(EngineSnapshot { status }));
    }
}

/// Deck an event refers to, for error reporting.
fn deck_of(event: &Event) -> Option<u16> {
    match event {
        Event::Play(e) => Some(e.deck),
        Event::Pause(e) => Some(e.deck),
        Event::TogglePlay(e) => Some(e.deck),
        Event::Seek(e) => Some(e.deck),
        Event::Unload(e) => Some(e.deck),
        Event::SetVolume(e) => Some(e.deck),
        Event::SetEq(e) => Some(e.deck),
        Event::SetEqBand(e) => Some(e.deck),
        Event::SetFilter(e) => Some(e.deck),
        Event::SetGainTrim(e) => Some(e.deck),
        Event::SetHeadphoneCue(e) => Some(e.deck),
        Event::ToggleHeadphoneCue(e) => Some(e.deck),
        Event::SetCrossfaderAssign(e) => Some(e.deck),
        Event::SetSpeed(e) => Some(e.deck),
        Event::SetTempoRange(e) => Some(e.deck),
        Event::SetKeyLock(e) => Some(e.deck),
        Event::SetKeyShift(e) => Some(e.deck),
        Event::SetKeyboardRoot(e) => Some(e.deck),
        Event::SetKeyboardPage(e) => Some(e.deck),
        Event::SetKeyShiftPage(e) => Some(e.deck),
        Event::ToggleSync(e) => Some(e.deck),
        Event::SetMasterDeck(e) => Some(e.deck),
        Event::SetCuePoint(e) => Some(e.deck),
        Event::BeginCueHold(e) => Some(e.deck),
        Event::EndCueHold(e) => Some(e.deck),
        Event::SetQuantize(e) => Some(e.deck),
        Event::ToggleQuantize(e) => Some(e.deck),
        Event::SetSlip(e) => Some(e.deck),
        Event::ToggleSlip(e) => Some(e.deck),
        Event::SetAutoLoop(e) => Some(e.deck),
        Event::LoopIn(e) => Some(e.deck),
        Event::LoopOut(e) => Some(e.deck),
        Event::ExitLoop(e) => Some(e.deck),
        Event::RecallSavedLoop(e) => Some(e.deck),
        Event::BeginLoopRoll(e) => Some(e.deck),
        Event::EndLoopRoll(e) => Some(e.deck),
        Event::BeatJump(e) => Some(e.deck),
        Event::SetPadMode(e) => Some(e.deck),
        Event::PadPress(e) => Some(e.deck),
        Event::PadRelease(e) => Some(e.deck),
        Event::HotCuePadPress(e) => Some(e.deck),
        Event::HotCuePadRelease(e) => Some(e.deck),
        Event::LoopRollPadPress(e) => Some(e.deck),
        Event::LoopRollPadRelease(e) => Some(e.deck),
        Event::BeatJumpPadPress(e) => Some(e.deck),
        Event::BeatJumpPadRelease(e) => Some(e.deck),
        Event::SamplerPadPress(e) => Some(e.deck),
        Event::SamplerPadRelease(e) => Some(e.deck),
        Event::KeyboardPadPress(e) => Some(e.deck),
        Event::KeyboardPadRelease(e) => Some(e.deck),
        Event::KeyShiftPadPress(e) => Some(e.deck),
        Event::KeyShiftPadRelease(e) => Some(e.deck),
        Event::TriggerHotCue(e) => Some(e.deck),
        Event::SaveDeckHotCue(e) => Some(e.deck),
        Event::DeleteDeckHotCue(e) => Some(e.deck),
        Event::AssignSampler(e) => Some(e.deck),
        Event::AssignSamplerTrack(e) => Some(e.deck),
        Event::ClearSampler(e) => Some(e.deck),
        Event::JogTouch(e) => Some(e.deck),
        Event::JogTurn(e) => Some(e.deck),
        Event::SetJogMode(e) => Some(e.deck),
        Event::SetDeckFxRouted(e) => Some(e.deck),
        _ => None,
    }
}

fn handle_cmd_event(event: Event, engine: &EngineHandle, bus: &EventBus<Event>) {
    let deck = deck_of(&event);
    let result = dispatch_cmd(&event, engine);

    match result {
        Ok(CmdOutcome::DeckUpdated(deck_id)) => {
            if let Err(e) = with_engine_ref(engine, |eng| publish_deck_updated(bus, eng, deck_id)) {
                publish_error(bus, deck, format!("{e:#}"));
            }
        }
        Ok(CmdOutcome::DecksUpdated(deck_ids)) => {
            for deck_id in deck_ids {
                if let Err(e) =
                    with_engine_ref(engine, |eng| publish_deck_updated(bus, eng, deck_id))
                {
                    publish_error(bus, deck, format!("{e:#}"));
                }
            }
        }
        Ok(CmdOutcome::EngineStatus) => {
            let _ = with_engine_ref(engine, |eng| {
                publish_engine_status(bus, eng);
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
                    publish(
                        bus,
                        Event::DeckPosition(DeckPosition {
                            deck: deck_id as u16,
                            position_ms,
                            slip_shadow_position_ms,
                        }),
                    );
                }
                Ok(())
            });
        }
        Ok(CmdOutcome::Silent) => {}
        Err(e) => publish_error(bus, deck, format!("{e:#}")),
    }
}

/// Run `f` against the engine with the deck id already converted to the index.
fn deck_cmd(
    deck: u16,
    engine: &EngineHandle,
    f: impl FnOnce(&mut Engine, usize) -> Result<CmdOutcome>,
) -> Result<CmdOutcome> {
    with_engine_mut(engine, |eng| f(eng, usize::from(deck)))
}

fn dispatch_cmd(event: &Event, engine: &EngineHandle) -> Result<CmdOutcome> {
    match event {
        // ── deck transport ───────────────────────────────────────────────────
        Event::Play(bus_api::Play { deck }) => deck_cmd(*deck, engine, |eng, id| {
            if !eng.deck_has_audio_loaded(id).unwrap_or(false) {
                return Err(anyhow!("No track loaded"));
            }
            eng.play(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::Pause(bus_api::Pause { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.pause(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::TogglePlay(bus_api::TogglePlay { deck }) => deck_cmd(*deck, engine, |eng, id| {
            if eng.deck_is_playing(id).unwrap_or(false) {
                eng.pause(id)?;
            } else {
                if !eng.deck_has_audio_loaded(id).unwrap_or(false) {
                    return Err(anyhow!("No track loaded"));
                }
                eng.play(id)?;
            }
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::Seek(bus_api::Seek { deck, position_ms }) => deck_cmd(*deck, engine, |eng, id| {
            eng.seek_deck(id, *position_ms)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::Unload(bus_api::Unload { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.unload_deck(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),

        // ── levels and mixer strip ───────────────────────────────────────────
        Event::SetVolume(bus_api::SetVolume {
            deck,
            volume,
            soft_takeover,
        }) => deck_cmd(*deck, engine, |eng, id| {
            let key = soft_takeover::key_deck(id, "volume");
            let current = eng.deck_strip_norms(id).map(|s| s.volume).unwrap_or(0.0);
            if !gate_mixer_soft_takeover(eng, &key, *soft_takeover, current, *volume) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_volume(id, *volume)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::SetEq(bus_api::SetEq {
            deck,
            low,
            mid,
            high,
        }) => deck_cmd(*deck, engine, |eng, id| {
            eng.set_deck_eq_bands(
                id,
                crate::control_norm::norm_to_strip_db(*low),
                crate::control_norm::norm_to_strip_db(*mid),
                crate::control_norm::norm_to_strip_db(*high),
            )?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::SetEqBand(bus_api::SetEqBand {
            deck,
            band,
            gain,
            soft_takeover,
        }) => deck_cmd(*deck, engine, |eng, id| {
            let strip = eng.deck_strip_norms(id);
            let (ctrl, current, low, mid, high) = match (band, strip) {
                (EqBand::Low, Some(s)) => ("eq_low", s.eq_low, *gain, s.eq_mid, s.eq_high),
                (EqBand::Mid, Some(s)) => ("eq_mid", s.eq_mid, s.eq_low, *gain, s.eq_high),
                (EqBand::High, Some(s)) => ("eq_high", s.eq_high, s.eq_low, s.eq_mid, *gain),
                (EqBand::Low, None) => ("eq_low", 0.5, *gain, 0.5, 0.5),
                (EqBand::Mid, None) => ("eq_mid", 0.5, 0.5, *gain, 0.5),
                (EqBand::High, None) => ("eq_high", 0.5, 0.5, 0.5, *gain),
            };
            let key = soft_takeover::key_deck(id, ctrl);
            if !eng
                .soft_takeover_mut()
                .allow(&key, *soft_takeover, current, *gain)
            {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_eq_bands(
                id,
                crate::control_norm::norm_to_strip_db(low),
                crate::control_norm::norm_to_strip_db(mid),
                crate::control_norm::norm_to_strip_db(high),
            )?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::SetFilter(bus_api::SetFilter {
            deck,
            filter,
            soft_takeover,
        }) => deck_cmd(*deck, engine, |eng, id| {
            let key = soft_takeover::key_deck(id, "filter");
            let current = eng.deck_strip_norms(id).map(|s| s.filter).unwrap_or(0.5);
            if !eng
                .soft_takeover_mut()
                .allow(&key, *soft_takeover, current, *filter)
            {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_filter_db(id, crate::control_norm::norm_to_strip_db(*filter))?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::SetGainTrim(bus_api::SetGainTrim {
            deck,
            gain_trim,
            soft_takeover,
        }) => deck_cmd(*deck, engine, |eng, id| {
            let key = soft_takeover::key_deck(id, "gain");
            let current = eng.deck_strip_norms(id).map(|s| s.gain_trim).unwrap_or(0.5);
            if !eng
                .soft_takeover_mut()
                .allow(&key, *soft_takeover, current, *gain_trim)
            {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_deck_gain_trim_db(id, crate::control_norm::norm_to_strip_db(*gain_trim))?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::SetHeadphoneCue(bus_api::SetHeadphoneCue { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_headphone_cue(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::ToggleHeadphoneCue(bus_api::ToggleHeadphoneCue { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let enabled = !eng
                    .deck_strip_norms(id)
                    .map(|s| s.headphone_cue)
                    .unwrap_or(false);
                eng.set_deck_headphone_cue(id, enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SetCrossfaderAssign(bus_api::SetCrossfaderAssign { deck, assignment }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_crossfader_assign(id, *assignment)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── tempo / pitch ────────────────────────────────────────────────────
        Event::SetSpeed(bus_api::SetSpeed {
            deck,
            speed,
            soft_takeover,
        }) => deck_cmd(*deck, engine, |eng, id| {
            let key = soft_takeover::key_deck(id, "speed");
            let current = eng.deck_strip_norms(id).map(|s| s.speed).unwrap_or(0.5);
            if !eng
                .soft_takeover_mut()
                .allow(&key, *soft_takeover, current, *speed)
            {
                return Ok(CmdOutcome::Silent);
            }
            let updated = eng.set_deck_speed(id, speed.clamp(0.0, 1.0))?;
            Ok(CmdOutcome::DecksUpdated(updated))
        }),
        Event::SetTempoRange(bus_api::SetTempoRange { deck, tempo_range }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let updated = eng.set_deck_tempo_range(id, *tempo_range)?;
                Ok(CmdOutcome::DecksUpdated(updated))
            })
        }
        Event::SetKeyLock(bus_api::SetKeyLock { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_key_lock(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SetKeyShift(bus_api::SetKeyShift { deck, semitones }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_key_shift(id, *semitones)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SetKeyboardRoot(bus_api::SetKeyboardRoot { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_keyboard_root(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SetKeyboardPage(bus_api::SetKeyboardPage { deck, page }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_keyboard_page(id, *page)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SetKeyShiftPage(bus_api::SetKeyShiftPage { deck, page }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_key_shift_page(id, *page)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::ToggleSync(bus_api::ToggleSync { deck, beat_sync }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let updated = eng.toggle_deck_sync(id, *beat_sync)?;
                Ok(CmdOutcome::DecksUpdated(updated))
            })
        }
        Event::SetMasterDeck(bus_api::SetMasterDeck { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let _updated = eng.set_master_deck(id)?;
                // master_deck + slave speeds/sync live on the engine snapshot decks.
                Ok(CmdOutcome::EngineStatus)
            })
        }

        // ── cue / loop ───────────────────────────────────────────────────────
        Event::SetCuePoint(bus_api::SetCuePoint { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.set_deck_cue_point_at_playhead(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::BeginCueHold(bus_api::BeginCueHold { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.begin_deck_cue_hold(id)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::EndCueHold(bus_api::EndCueHold { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.end_deck_cue_hold(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::SetQuantize(bus_api::SetQuantize { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_quantize(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::ToggleQuantize(bus_api::ToggleQuantize { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let enabled = !eng.deck_quantize(id).unwrap_or(true);
                eng.set_deck_quantize(id, enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SetSlip(bus_api::SetSlip { deck, enabled }) => deck_cmd(*deck, engine, |eng, id| {
            eng.set_deck_slip(id, *enabled)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::ToggleSlip(bus_api::ToggleSlip { deck }) => deck_cmd(*deck, engine, |eng, id| {
            let enabled = !eng.deck_slip_enabled(id).unwrap_or(false);
            eng.set_deck_slip(id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::SetAutoLoop(bus_api::SetAutoLoop { deck, beats }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_auto_loop(id, *beats)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::LoopIn(bus_api::LoopIn { deck, position_ms }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_loop_in(id, *position_ms)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::LoopOut(bus_api::LoopOut { deck, position_ms }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_loop_out(id, *position_ms)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::ExitLoop(bus_api::ExitLoop { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.clear_deck_loop(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::RecallSavedLoop(bus_api::RecallSavedLoop {
            deck,
            in_ms,
            out_ms,
        }) => deck_cmd(*deck, engine, |eng, id| {
            eng.recall_deck_saved_loop(id, *in_ms, *out_ms)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::BeginLoopRoll(bus_api::BeginLoopRoll { deck, beats }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.begin_deck_loop_roll(id, *beats)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::EndLoopRoll(bus_api::EndLoopRoll { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.end_deck_loop_roll(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Event::BeatJump(bus_api::BeatJump { deck, beats }) => deck_cmd(*deck, engine, |eng, id| {
            eng.beat_jump_deck(id, *beats)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),

        // ── pads ─────────────────────────────────────────────────────────────
        Event::SetPadMode(bus_api::SetPadMode { deck, mode }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_pad_mode(id, *mode)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::PadPress(bus_api::PadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::PadRelease(bus_api::PadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::HotCuePadPress(bus_api::HotCuePadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.hot_cue_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::HotCuePadRelease(bus_api::HotCuePadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.hot_cue_pad_release(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }
        Event::LoopRollPadPress(bus_api::LoopRollPadPress { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.loop_roll_pad_press(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::LoopRollPadRelease(bus_api::LoopRollPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.loop_roll_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::BeatJumpPadPress(bus_api::BeatJumpPadPress { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.beat_jump_pad_press(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::BeatJumpPadRelease(bus_api::BeatJumpPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.beat_jump_pad_release(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }
        Event::SamplerPadPress(bus_api::SamplerPadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.sampler_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SamplerPadRelease(bus_api::SamplerPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.sampler_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::KeyboardPadPress(bus_api::KeyboardPadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.keyboard_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::KeyboardPadRelease(bus_api::KeyboardPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.keyboard_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::KeyShiftPadPress(bus_api::KeyShiftPadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.key_shift_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::KeyShiftPadRelease(bus_api::KeyShiftPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.key_shift_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::TriggerHotCue(bus_api::TriggerHotCue { deck, position_ms }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.trigger_deck_hot_cue(id, *position_ms)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── hot cues / loops on the deck ─────────────────────────────────────
        Event::SaveDeckHotCue(bus_api::SaveDeckHotCue { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.save_deck_hot_cue(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }
        Event::DeleteDeckHotCue(bus_api::DeleteDeckHotCue { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.delete_deck_hot_cue(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }

        // ── sampler slots ────────────────────────────────────────────────────
        Event::AssignSampler(bus_api::AssignSampler { deck, slot, path }) => {
            dispatch_assign_sampler(*deck, *slot, path, engine)
        }
        Event::AssignSamplerTrack(bus_api::AssignSamplerTrack {
            deck,
            slot,
            track_id,
        }) => dispatch_assign_sampler_track(*deck, *slot, track_id, engine),
        Event::ClearSampler(bus_api::ClearSampler { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.clear_sampler_slot(id, usize::from(*slot))?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── jog ──────────────────────────────────────────────────────────────
        Event::JogTouch(bus_api::JogTouch { deck, touching }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_jog_touch(id, *touching)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::JogTurn(bus_api::JogTurn { deck, delta }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.deck_jog_turn(id, *delta)?;
                // So paused vinyl scrub updates UI playhead/time without waiting for
                // the next 33ms tick (JogTouch release used to be the only cue).
                Ok(CmdOutcome::Position(id))
            })
        }
        Event::SetJogMode(bus_api::SetJogMode { deck, top, outer }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_jog_mode(id, *top, *outer)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Event::SetDeckFxRouted(bus_api::SetDeckFxRouted { deck, routed }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_fx_routed(id, *routed)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── mixer ────────────────────────────────────────────────────────────
        Event::SetCrossfader(bus_api::SetCrossfader {
            position,
            soft_takeover,
        }) => with_engine_mut(engine, |eng| {
            let key = soft_takeover::key_mixer("crossfader");
            let current = eng.crossfader().unwrap_or(0.5);
            if !gate_mixer_soft_takeover(eng, &key, *soft_takeover, current, *position) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_crossfader(*position)?;
            Ok(CmdOutcome::EngineStatus)
        }),
        Event::SetMasterVolume(bus_api::SetMasterVolume {
            volume,
            soft_takeover,
        }) => with_engine_mut(engine, |eng| {
            let key = soft_takeover::key_mixer("master_volume");
            let current = eng.master_volume().unwrap_or(1.0);
            if !gate_mixer_soft_takeover(eng, &key, *soft_takeover, current, *volume) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_master_volume(*volume)?;
            Ok(CmdOutcome::EngineStatus)
        }),
        Event::SetCueVolume(bus_api::SetCueVolume {
            volume,
            soft_takeover,
        }) => with_engine_mut(engine, |eng| {
            let key = soft_takeover::key_mixer("cue_volume");
            let current = eng.cue_volume().unwrap_or(1.0);
            if !gate_mixer_soft_takeover(eng, &key, *soft_takeover, current, *volume) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_cue_volume(*volume)?;
            Ok(CmdOutcome::EngineStatus)
        }),
        Event::SetCueMix(bus_api::SetCueMix { mix, soft_takeover }) => {
            with_engine_mut(engine, |eng| {
                let key = soft_takeover::key_mixer("cue_mix");
                let current = eng.cue_mix().unwrap_or(0.5);
                if !gate_mixer_soft_takeover(eng, &key, *soft_takeover, current, *mix) {
                    return Ok(CmdOutcome::Silent);
                }
                eng.set_cue_mix(*mix)?;
                Ok(CmdOutcome::EngineStatus)
            })
        }
        Event::SetMasterCue(bus_api::SetMasterCue { enabled }) => with_engine_mut(engine, |eng| {
            eng.set_master_cue(*enabled)?;
            Ok(CmdOutcome::EngineStatus)
        }),
        Event::ToggleMasterCue(first) => {
            // `ToggleMasterCue` folds in the subscriber, so a folded pair never
            // reaches the engine and `flipped` is always false here.
            let _ = first;
            with_engine_mut(engine, |eng| {
                let enabled = !eng.master_cue().unwrap_or(false);
                eng.set_master_cue(enabled)?;
                Ok(CmdOutcome::EngineStatus)
            })
        }

        // ── shared FX unit ───────────────────────────────────────────────────
        Event::SetFxUnitSlot(bus_api::SetFxUnitSlot { slot, kind }) => {
            with_engine_mut(engine, |eng| {
                eng.set_fx_unit_slot(*slot, crate::status::map_fx_kind(*kind))?;
                Ok(CmdOutcome::DecksUpdated(
                    (0..eng.config.num_decks).collect(),
                ))
            })
        }
        Event::SetFxUnitSlotEnabled(bus_api::SetFxUnitSlotEnabled { slot, enabled }) => {
            with_engine_mut(engine, |eng| {
                eng.set_fx_unit_slot_enabled(*slot, *enabled)?;
                Ok(CmdOutcome::DecksUpdated(
                    (0..eng.config.num_decks).collect(),
                ))
            })
        }
        Event::SetFxUnitWet(bus_api::SetFxUnitWet {
            slot,
            wet,
            soft_takeover,
        }) => with_engine_mut(engine, |eng| {
            let key = soft_takeover::key_mixer(&format!("fx_wet{slot}"));
            let current = eng.fx_unit_wet(*slot).unwrap_or(0.5);
            if !gate_mixer_soft_takeover(eng, &key, *soft_takeover, current, *wet) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_fx_unit_wet(*slot, *wet)?;
            Ok(CmdOutcome::DecksUpdated(
                (0..eng.config.num_decks).collect(),
            ))
        }),
        Event::SetFxUnitParam(bus_api::SetFxUnitParam { slot, param, value }) => {
            with_engine_mut(engine, |eng| {
                eng.set_fx_unit_param(*slot, *param, *value)?;
                Ok(CmdOutcome::DecksUpdated(
                    (0..eng.config.num_decks).collect(),
                ))
            })
        }

        // ── egress-only events never arrive as commands ───────────────────────
        Event::DeckUpdated(_)
        | Event::DeckPosition(_)
        | Event::DeckLevels(_)
        | Event::EngineSnapshot(_)
        | Event::Error(_)
        | Event::Notice(_) => Err(anyhow!("egress event is not a command")),

        // ── library and UI commands are not the engine's to execute ────────────
        Event::AnalyzeTrack(_)
        | Event::GenerateStems(_)
        | Event::RefreshTrack(_)
        | Event::SaveBeatGrid(_)
        | Event::SaveTrackHotCue(_)
        | Event::DeleteTrackHotCue(_)
        | Event::SaveTrackLoop(_)
        | Event::DeleteTrackLoop(_)
        | Event::TrackAnalyzed(_)
        | Event::TrackUpdated(_)
        | Event::TrackProgress(_)
        | Event::HotCuesChanged(_)
        | Event::LoopsChanged(_)
        | Event::BeatGridChanged(_)
        | Event::HistorySessionUpdated(_)
        | Event::Navigate(_)
        | Event::Focus(_)
        | Event::Load(_) => Err(anyhow!("library event is not an engine command")),
    }
}

fn dispatch_assign_sampler(
    deck: u16,
    slot: u8,
    path: &str,
    engine: &EngineHandle,
) -> Result<CmdOutcome> {
    let deck_id = usize::from(deck);
    let library = with_engine_ref(engine, |eng| {
        eng.library_manager()
            .ok_or_else(|| anyhow!("Engine has no attached LibraryManager"))
    })?;
    let prepared: PreparedTrackPlayback =
        LibraryManager::prepare_file_path_for_playback(library.as_ref(), Path::new(path))
            .map_err(|e| anyhow!(e.to_string()))?;
    with_engine_mut(engine, |eng| {
        eng.assign_prepared_sampler(deck_id, usize::from(slot), prepared)?;
        Ok(CmdOutcome::DeckUpdated(deck_id))
    })
}

fn dispatch_assign_sampler_track(
    deck: u16,
    slot: u8,
    track_id: &str,
    engine: &EngineHandle,
) -> Result<CmdOutcome> {
    let deck_id = usize::from(deck);
    let library = with_engine_ref(engine, |eng| {
        eng.library_manager()
            .ok_or_else(|| anyhow!("Engine has no attached LibraryManager"))
    })?;
    let prepared: PreparedTrackPlayback =
        LibraryManager::prepare_track_for_playback(library.as_ref(), &TrackId::new(track_id))
            .map_err(|e| anyhow!(e.to_string()))?;
    with_engine_mut(engine, |eng| {
        eng.assign_prepared_sampler(deck_id, usize::from(slot), prepared)?;
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

fn tick(engine: &EngineHandle, bus: &EventBus<Event>, peak_hold: &mut PeakHoldState) {
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
        let _ = publish_deck_updated(bus, eng, deck_id);
    }

    // A live stem producer publishes its first window asynchronously; emit one
    // Updated so pads/banner reflect readiness without waiting for another cmd.
    for deck_id in 0..eng.config.num_decks {
        let ready = eng.deck_stems_ready(deck_id).unwrap_or(false);
        if ready != eng.stems_ready_seen[deck_id] {
            eng.stems_ready_seen[deck_id] = ready;
            let _ = publish_deck_updated(bus, eng, deck_id);
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
            publish(
                bus,
                Event::DeckPosition(DeckPosition {
                    deck: deck_id as u16,
                    position_ms: position,
                    slip_shadow_position_ms,
                }),
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
                publish(
                    bus,
                    Event::DeckLevels(DeckLevels {
                        deck: Some(deck_id as u16),
                        peak_l: 0.0,
                        peak_r: 0.0,
                        peak_hold_l: 0.0,
                        peak_hold_r: 0.0,
                    }),
                );
            }
            continue;
        }
        peak_hold.mark_published(deck_id, false);
        publish(
            bus,
            Event::DeckLevels(DeckLevels {
                deck: Some(deck_id as u16),
                peak_l,
                peak_r,
                peak_hold_l,
                peak_hold_r,
            }),
        );
    }

    // Master bus peaks (post fader). Hold slot sits above deck indices.
    let master_hold_slot = eng.config.num_decks;
    if let Some((peak_l, peak_r)) = eng.master_level_snapshot() {
        let any_playing =
            (0..eng.config.num_decks).any(|deck_id| eng.deck_is_playing(deck_id) == Some(true));
        let (peak_hold_l, peak_hold_r) = peak_hold.update(master_hold_slot, peak_l, peak_r);
        if !should_publish_levels(any_playing, peak_l, peak_r, peak_hold_l, peak_hold_r) {
            if peak_hold.take_zero_flush(master_hold_slot) {
                publish(
                    bus,
                    Event::DeckLevels(DeckLevels {
                        deck: None,
                        peak_l: 0.0,
                        peak_r: 0.0,
                        peak_hold_l: 0.0,
                        peak_hold_r: 0.0,
                    }),
                );
            }
        } else {
            peak_hold.mark_published(master_hold_slot, false);
            publish(
                bus,
                Event::DeckLevels(DeckLevels {
                    deck: None,
                    peak_l,
                    peak_r,
                    peak_hold_l,
                    peak_hold_r,
                }),
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
