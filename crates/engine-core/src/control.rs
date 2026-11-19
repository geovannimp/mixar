//! Control thread: cmd dispatch and high-rate evt egress.

use crate::engine::Engine;
use crate::fader_start::{FaderStartAction, FaderStartConfig, OnNonZeroAction, OnZeroAction};
use crate::soft_takeover;
use crate::transport::TransportEvent;
use anyhow::anyhow;
use anyhow::Result;
use bus_api::Command;
use bus_api::CommandTarget;
use bus_api::DeckLevels;
use bus_api::DeckPosition;
use bus_api::DeckUpdated;
use bus_api::EngineSnapshot;
use bus_api::EqBand;
use bus_api::Event;
use bus_api::EventTopic;
use event_bus::EventBus;
use event_bus::Subscriber;
use library::{LibraryManager, PreparedTrackPlayback};
use library_core::TrackId;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;

const TICK_INTERVAL: Duration = Duration::from_millis(33);
/// Commands one pass may handle: the rest stay queued for the next pass.
const MAX_COMMANDS_PER_PASS: usize = 64;
/// Library acknowledgements one pass may handle, bounded the same way.
const MAX_LIBRARY_EVENTS_PER_PASS: usize = 64;
/// Wall-clock budget for one batch. Each lane gets its own, so one burst of
/// commands cannot consume the library batch's time.
const BATCH_BUDGET: Duration = Duration::from_millis(2);
const PEAK_HOLD_DECAY_PER_TICK: f32 = 0.04;
const LEVEL_IDLE_EPSILON: f32 = 1e-5;

/// Engine handle shared with the host (the engine can be stopped/restarted).
type EngineHandle = Arc<Mutex<Option<Engine>>>;

enum CmdOutcome {
    DeckUpdated(usize),
    /// Publish a notice, then the updated deck (a coerced value was used).
    DeckUpdatedWithNotice(usize, String),
    DecksUpdated(Vec<usize>),
    EngineStatus,
    /// Publish a notice, then the engine status (a coerced value was used).
    EngineStatusWithNotice(String),
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
    bus: EventBus<Command, Event>,
    engine: EngineHandle,
    shutdown: Arc<AtomicBool>,
    ready: std::sync::mpsc::Sender<Result<()>>,
) {
    let cmd_rx = match bus.subscribe_cmd(CommandTarget::Engine, "engine") {
        Ok(rx) => rx,
        Err(e) => {
            let _ = ready.send(Err(e.into()));
            return;
        }
    };
    // Best-effort at control-thread start: the engine only needs two topics from
    // the library lane, so it filters instead of inspecting every egress event.
    // A missing subscription disables hot-cue/beat-grid acknowledgement sync; it
    // must not stop the engine worker.
    let library_rx = library_evt_receiver(&engine);
    let _ = ready.send(Ok(()));

    let mut control = ControlLoop {
        engine: &engine,
        bus: &bus,
        shutdown: &shutdown,
        cmd_rx,
        library_rx,
        peak_hold: PeakHoldState::new(),
        pending_command: None,
        next_tick: Instant::now() + TICK_INTERVAL,
        command_lane_closed: false,
    };
    control.run();
}

/// What one control pass did.
#[derive(Debug, Default, PartialEq, Eq)]
struct PassSummary {
    commands: usize,
    library_events: usize,
    ticked: bool,
}

impl PassSummary {
    /// Whether the pass handled any message at all.
    fn worked(&self) -> bool {
        self.commands > 0 || self.library_events > 0
    }
}

/// Queues, tick deadline and carried command of the control thread.
///
/// A pass is bounded work, not "everything queued": commands and library
/// acknowledgements are handled in FIFO batches of at most
/// [`MAX_COMMANDS_PER_PASS`] / [`MAX_LIBRARY_EVENTS_PER_PASS`] messages or
/// [`BATCH_BUDGET`] each, and the tick runs once its 33 ms deadline is due.
/// Work left over stays queued for the next pass, so a sustained producer can
/// no longer amplify telemetry or delay shutdown by keeping a queue non-empty.
struct ControlLoop<'a> {
    engine: &'a EngineHandle,
    bus: &'a EventBus<Command, Event>,
    shutdown: &'a AtomicBool,
    cmd_rx: Subscriber<Command>,
    /// `None` once the subscription is gone or disconnected.
    library_rx: Option<Subscriber<Arc<Event>>>,
    peak_hold: PeakHoldState,
    /// A command read while idling until the tick deadline.
    pending_command: Option<Command>,
    next_tick: Instant,
    /// The command lane disconnected: no command can arrive any more.
    command_lane_closed: bool,
}

impl ControlLoop<'_> {
    /// One bounded pass: commands, then library acknowledgements, then the tick
    /// if its deadline is due.
    ///
    /// `clock` is a parameter so tests can drive deadlines with explicit
    /// instants instead of sleeping.
    fn pass(&mut self, clock: impl Fn() -> Instant) -> PassSummary {
        let commands = self.command_batch(&clock);
        let library_events = self.library_batch(&clock);
        let now = clock();
        let ticked = now >= self.next_tick;
        if ticked {
            tick(self.engine, self.bus, &mut self.peak_hold);
            // No catch-up burst: one late tick is one tick.
            self.next_tick = now + TICK_INTERVAL;
        }
        PassSummary {
            commands,
            library_events,
            ticked,
        }
    }

    /// Handle the carried command, then read up to the batch budget.
    fn command_batch(&mut self, clock: &impl Fn() -> Instant) -> usize {
        let deadline = clock() + BATCH_BUDGET;
        let mut handled = 0;
        if let Some(command) = self.pending_command.take() {
            handle_command(command, self.engine, self.bus);
            handled += 1;
        }
        while handled < MAX_COMMANDS_PER_PASS {
            if self.shutdown.load(Ordering::Relaxed) || clock() >= deadline {
                break;
            }
            match self.cmd_rx.try_recv() {
                Ok(Some(command)) => {
                    handle_command(command, self.engine, self.bus);
                    handled += 1;
                }
                Ok(None) => break,
                Err(err) => {
                    // The lane is gone: stop polling it, and stop the loop.
                    tracing::debug!(error = %err, "engine command lane disconnected");
                    self.command_lane_closed = true;
                    break;
                }
            }
        }
        handled
    }

    /// Handle up to the batch budget of library acknowledgements.
    fn library_batch(&mut self, clock: &impl Fn() -> Instant) -> usize {
        let deadline = clock() + BATCH_BUDGET;
        let mut handled = 0;
        let mut disconnected = false;
        if let Some(rx) = self.library_rx.as_mut() {
            while handled < MAX_LIBRARY_EVENTS_PER_PASS {
                if self.shutdown.load(Ordering::Relaxed) || clock() >= deadline {
                    break;
                }
                match rx.try_recv() {
                    Ok(Some(event)) => {
                        handle_library_evt(event.as_ref(), self.engine, self.bus);
                        handled += 1;
                    }
                    Ok(None) => break,
                    Err(err) => {
                        tracing::debug!(error = %err, "library evt lane disconnected");
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if disconnected {
            self.library_rx = None;
        }
        handled
    }

    /// Run passes until shutdown, blocking only when both lanes are empty.
    fn run(&mut self) {
        while !self.shutdown.load(Ordering::Relaxed) && !self.command_lane_closed {
            let summary = self.pass(Instant::now);
            if summary.worked() {
                continue;
            }
            // Both lanes idle: wait for the next command until the tick deadline.
            // A library acknowledgement arriving now is serviced by that tick.
            let timeout = self.next_tick.saturating_duration_since(Instant::now());
            match self.cmd_rx.recv_timeout(timeout) {
                Ok(Some(command)) => self.pending_command = Some(command),
                Ok(None) => {}
                Err(err) => {
                    tracing::debug!(error = %err, "engine command lane disconnected");
                    self.command_lane_closed = true;
                }
            }
        }
    }
}

fn library_evt_receiver(engine: &EngineHandle) -> Option<Subscriber<Arc<Event>>> {
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
    match buses.subscribe_evt_topics(
        "engine-library",
        &[EventTopic::HotCuesChanged, EventTopic::BeatGridChanged],
    ) {
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

fn handle_library_evt(event: &Event, engine: &EngineHandle, bus: &EventBus<Command, Event>) {
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
        // The subscription is topic-filtered to the two events above, so the
        // rest can never arrive; listing them keeps a new event from slipping
        // into a catch-all.
        Event::DeckUpdated(_)
        | Event::DeckPosition(_)
        | Event::DeckLevels(_)
        | Event::EngineSnapshot(_)
        | Event::Error(_)
        | Event::Notice(_)
        | Event::TrackAnalyzed(_)
        | Event::TrackUpdated(_)
        | Event::TrackProgress(_)
        | Event::LoopsChanged(_)
        | Event::HistorySessionUpdated(_)
        | Event::Navigate(_)
        | Event::Focus(_)
        | Event::Load(_)
        | Event::MappingOffer(_)
        | Event::MappingAttached(_)
        | Event::MappingDetached(_) => {}
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

/// Egress publish from the control thread.
///
/// A closed bus means no host is listening, which must not stop the control
/// loop; a poisoned lane is a bug worth a log line, not a panic in the worker.
fn publish(bus: &EventBus<Command, Event>, event: Event) {
    if let Err(err) = bus.publish_evt(event) {
        tracing::error!(error = %err, "event bus publication failed");
    }
}

fn publish_error(bus: &EventBus<Command, Event>, deck: Option<u16>, message: impl Into<String>) {
    publish(
        bus,
        Event::Error(bus_api::Error {
            message: message.into(),
            track_id: None,
            deck,
        }),
    );
}

/// Non-fatal notice: a command was accepted with a coerced value.
fn publish_notice(bus: &EventBus<Command, Event>, message: impl Into<String>) {
    publish(
        bus,
        Event::Notice(bus_api::Notice {
            message: message.into(),
        }),
    );
}

fn publish_deck_updated(
    bus: &EventBus<Command, Event>,
    engine: &Engine,
    deck_id: usize,
) -> Result<()> {
    let snap = engine
        .deck_snapshot(deck_id)
        .ok_or_else(|| anyhow!("deck snapshot unavailable"))?;
    publish(bus, Event::DeckUpdated(DeckUpdated::from(snap)));
    Ok(())
}

fn publish_engine_status(bus: &EventBus<Command, Event>, engine: &Engine) {
    if let Some(status) = engine.engine_status_snapshot() {
        publish(bus, Event::EngineSnapshot(EngineSnapshot { status }));
    }
}

/// Deck a command refers to, for error reporting.
fn deck_of(command: &Command) -> Option<u16> {
    match command {
        Command::Play(e) => Some(e.deck),
        Command::Pause(e) => Some(e.deck),
        Command::TogglePlay(e) => Some(e.deck),
        Command::Seek(e) => Some(e.deck),
        Command::Unload(e) => Some(e.deck),
        Command::SetVolume(e) => Some(e.deck),
        Command::SetEq(e) => Some(e.deck),
        Command::SetEqBand(e) => Some(e.deck),
        Command::SetStemEqBand(e) => Some(e.deck),
        Command::SetStemEqEnabled(e) => Some(e.deck),
        Command::ToggleStemEq(e) => Some(e.deck),
        Command::SetFilter(e) => Some(e.deck),
        Command::SetGainTrim(e) => Some(e.deck),
        Command::SetHeadphoneCue(e) => Some(e.deck),
        Command::ToggleHeadphoneCue(e) => Some(e.deck),
        Command::SetCrossfaderAssign(e) => Some(e.deck),
        Command::SetSpeed(e) => Some(e.deck),
        Command::SetTempoRange(e) => Some(e.deck),
        Command::SetTempoCurve(e) => Some(e.deck),
        Command::SetKeyLock(e) => Some(e.deck),
        Command::SetKeyShift(e) => Some(e.deck),
        Command::SetKeyboardRoot(e) => Some(e.deck),
        Command::SetKeyboardPage(e) => Some(e.deck),
        Command::SetKeyShiftPage(e) => Some(e.deck),
        Command::ToggleSync(e) => Some(e.deck),
        Command::SetMasterDeck(e) => Some(e.deck),
        Command::SetCuePoint(e) => Some(e.deck),
        Command::BeginCueHold(e) => Some(e.deck),
        Command::EndCueHold(e) => Some(e.deck),
        Command::FaderStart(e) => Some(e.deck),
        Command::FaderStop(e) => Some(e.deck),
        Command::SetFaderStartNative(e) => Some(e.deck),
        Command::SetQuantize(e) => Some(e.deck),
        Command::ToggleQuantize(e) => Some(e.deck),
        Command::SetSlip(e) => Some(e.deck),
        Command::ToggleSlip(e) => Some(e.deck),
        Command::SetAutoLoop(e) => Some(e.deck),
        Command::LoopIn(e) => Some(e.deck),
        Command::LoopOut(e) => Some(e.deck),
        Command::ExitLoop(e) => Some(e.deck),
        Command::RecallSavedLoop(e) => Some(e.deck),
        Command::BeginLoopRoll(e) => Some(e.deck),
        Command::EndLoopRoll(e) => Some(e.deck),
        Command::BeatJump(e) => Some(e.deck),
        Command::SetPadMode(e) => Some(e.deck),
        Command::PadPress(e) => Some(e.deck),
        Command::PadRelease(e) => Some(e.deck),
        Command::HotCuePadPress(e) => Some(e.deck),
        Command::HotCuePadRelease(e) => Some(e.deck),
        Command::LoopRollPadPress(e) => Some(e.deck),
        Command::LoopRollPadRelease(e) => Some(e.deck),
        Command::BeatJumpPadPress(e) => Some(e.deck),
        Command::BeatJumpPadRelease(e) => Some(e.deck),
        Command::SamplerPadPress(e) => Some(e.deck),
        Command::SamplerPadRelease(e) => Some(e.deck),
        Command::KeyboardPadPress(e) => Some(e.deck),
        Command::KeyboardPadRelease(e) => Some(e.deck),
        Command::KeyShiftPadPress(e) => Some(e.deck),
        Command::KeyShiftPadRelease(e) => Some(e.deck),
        Command::TriggerHotCue(e) => Some(e.deck),
        Command::SaveDeckHotCue(e) => Some(e.deck),
        Command::DeleteDeckHotCue(e) => Some(e.deck),
        Command::AssignSampler(e) => Some(e.deck),
        Command::AssignSamplerTrack(e) => Some(e.deck),
        Command::ClearSampler(e) => Some(e.deck),
        Command::JogTouch(e) => Some(e.deck),
        Command::JogTurn(e) => Some(e.deck),
        Command::SetJogMode(e) => Some(e.deck),
        Command::SetDeckFxRouted(e) => Some(e.deck),
        _ => None,
    }
}

fn handle_command(command: Command, engine: &EngineHandle, bus: &EventBus<Command, Event>) {
    let deck = deck_of(&command);
    let result = dispatch_cmd(&command, engine);

    match result {
        Ok(CmdOutcome::DeckUpdated(deck_id)) => {
            if let Err(e) = with_engine_ref(engine, |eng| publish_deck_updated(bus, eng, deck_id)) {
                publish_error(bus, deck, format!("{e:#}"));
            }
        }
        Ok(CmdOutcome::DeckUpdatedWithNotice(deck_id, message)) => {
            publish_notice(bus, message);
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
        Ok(CmdOutcome::EngineStatusWithNotice(message)) => {
            publish_notice(bus, message);
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

/// Handle either native fader edge with identical enabled/no-op semantics.
/// Actions come from the global config; `deck_cmd` supplies the deck index for
/// the resulting `DeckUpdated` notification.
fn native_fader_cmd(
    deck: u16,
    engine: &EngineHandle,
    select_action: impl FnOnce(&FaderStartConfig) -> Option<FaderStartAction>,
) -> Result<CmdOutcome> {
    deck_cmd(deck, engine, |eng, id| {
        if !eng.fader_start_cfg.enabled {
            return Ok(CmdOutcome::Silent);
        }
        let Some(action) = select_action(&eng.fader_start_cfg) else {
            return Ok(CmdOutcome::Silent);
        };
        apply_fader_start_action(eng, deck, action)?;
        Ok(CmdOutcome::DeckUpdated(id))
    })
}

/// Apply a fader-start transport action. No-ops are deliberate: an unloaded
/// deck, a disabled feature, or a `Nothing` action must never surface an error.
fn apply_fader_start_action(eng: &mut Engine, deck: u16, action: FaderStartAction) -> Result<()> {
    let id = deck as usize;
    match action {
        FaderStartAction::Play => {
            if !eng.deck_has_audio_loaded(id).unwrap_or(false) {
                return Ok(());
            }
            eng.play(id)?;
        }
        FaderStartAction::TogglePlay => {
            if eng.deck_is_playing(id).unwrap_or(false) {
                eng.pause(id)?;
            } else if eng.deck_has_audio_loaded(id).unwrap_or(false) {
                eng.play(id)?;
            }
        }
        FaderStartAction::Pause => {
            eng.pause(id)?;
        }
        FaderStartAction::PauseAndCue => {
            eng.pause(id)?;
            if let Some(ms) = eng.deck_snapshot(id).and_then(|s| s.cue_point_ms) {
                eng.seek_deck(id, ms)?;
            }
        }
    }
    Ok(())
}

/// Apply one stem-EQ band with soft-takeover gating; `Ok(false)` = gated out.
fn apply_stem_eq_band(
    eng: &mut Engine,
    deck_id: usize,
    band: EqBand,
    gain: f32,
    soft_takeover: bool,
) -> Result<bool> {
    let current = eng.deck_stem_eq(deck_id).unwrap_or([0.5; 3]);
    let (ctrl, current) = match band {
        EqBand::Low => ("stem_eq_low", current[0]),
        EqBand::Mid => ("stem_eq_mid", current[1]),
        EqBand::High => ("stem_eq_high", current[2]),
    };
    let key = soft_takeover::key_deck(deck_id, ctrl);
    if !eng
        .soft_takeover_mut()
        .allow(&key, soft_takeover, current, gain.clamp(0.0, 1.0))
    {
        return Ok(false);
    }
    eng.set_deck_stem_eq_band(deck_id, band, gain)?;
    Ok(true)
}

fn dispatch_cmd(command: &Command, engine: &EngineHandle) -> Result<CmdOutcome> {
    match command {
        // ── deck transport ───────────────────────────────────────────────────
        Command::Play(bus_api::Play { deck }) => deck_cmd(*deck, engine, |eng, id| {
            if !eng.deck_has_audio_loaded(id).unwrap_or(false) {
                return Err(anyhow!("No track loaded"));
            }
            eng.play(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::Pause(bus_api::Pause { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.pause(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::TogglePlay(bus_api::TogglePlay { deck }) => deck_cmd(*deck, engine, |eng, id| {
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
        Command::Seek(bus_api::Seek { deck, position_ms }) => deck_cmd(*deck, engine, |eng, id| {
            eng.seek_deck(id, *position_ms)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::Unload(bus_api::Unload { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.unload_deck(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),

        // ── levels and mixer strip ───────────────────────────────────────────
        Command::SetVolume(bus_api::SetVolume {
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
            if let Some(action) = eng.fader_start_action(*deck, *volume) {
                apply_fader_start_action(eng, *deck, action)?;
            }
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::FaderStart(bus_api::FaderStart { deck }) => {
            native_fader_cmd(*deck, engine, |cfg| match cfg.on_non_zero {
                OnNonZeroAction::Nothing => None,
                OnNonZeroAction::Play => Some(FaderStartAction::Play),
                OnNonZeroAction::TogglePlay => Some(FaderStartAction::TogglePlay),
            })
        }
        Command::FaderStop(bus_api::FaderStop { deck }) => {
            native_fader_cmd(*deck, engine, |cfg| match cfg.on_zero {
                OnZeroAction::Nothing => None,
                OnZeroAction::Pause => Some(FaderStartAction::Pause),
                OnZeroAction::PauseAndCue => Some(FaderStartAction::PauseAndCue),
            })
        }
        Command::SetFaderStartNative(bus_api::SetFaderStartNative { deck, native }) => {
            deck_cmd(*deck, engine, |eng, _id| {
                // Track ownership even while disabled, so enabling the feature
                // later respects controllers that are already attached.
                eng.set_fader_start_native(*deck, *native);
                Ok(CmdOutcome::Silent)
            })
        }
        Command::SetEq(bus_api::SetEq {
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
        Command::SetEqBand(bus_api::SetEqBand {
            deck,
            band,
            gain,
            soft_takeover,
        }) => deck_cmd(*deck, engine, |eng, id| {
            // Audible, not merely attached: while live separation is behind, the
            // deck plays its dry mix and stem gains would do nothing — the knobs
            // must drive the channel EQ instead of a silent path.
            if eng.deck_stem_eq_enabled(id).unwrap_or(false)
                && eng.deck_stems_audible(id).unwrap_or(false)
            {
                return if apply_stem_eq_band(eng, id, *band, *gain, *soft_takeover)? {
                    Ok(CmdOutcome::DeckUpdated(id))
                } else {
                    Ok(CmdOutcome::Silent)
                };
            }
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
        Command::SetStemEqBand(bus_api::SetStemEqBand {
            deck,
            band,
            gain,
            soft_takeover,
        }) => deck_cmd(*deck, engine, |eng, id| {
            if !apply_stem_eq_band(eng, id, *band, *gain, *soft_takeover)? {
                return Ok(CmdOutcome::Silent);
            }
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::SetStemEqEnabled(bus_api::SetStemEqEnabled { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_stem_eq_enabled(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::ToggleStemEq(bus_api::ToggleStemEq { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let enabled = !eng.deck_stem_eq_enabled(id).unwrap_or(false);
                eng.set_deck_stem_eq_enabled(id, enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetFilter(bus_api::SetFilter {
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
        Command::SetGainTrim(bus_api::SetGainTrim {
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
        Command::SetHeadphoneCue(bus_api::SetHeadphoneCue { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_headphone_cue(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::ToggleHeadphoneCue(bus_api::ToggleHeadphoneCue { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let enabled = !eng
                    .deck_strip_norms(id)
                    .map(|s| s.headphone_cue)
                    .unwrap_or(false);
                eng.set_deck_headphone_cue(id, enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetCrossfaderAssign(bus_api::SetCrossfaderAssign { deck, assignment }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_crossfader_assign(id, *assignment)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── tempo / pitch ────────────────────────────────────────────────────
        Command::SetSpeed(bus_api::SetSpeed {
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
        Command::SetTempoRange(bus_api::SetTempoRange { deck, tempo_range }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let updated = eng.set_deck_tempo_range(id, *tempo_range)?;
                Ok(CmdOutcome::DecksUpdated(updated))
            })
        }
        Command::SetTempoCurve(bus_api::SetTempoCurve { deck, id }) => {
            deck_cmd(*deck, engine, |eng, deck_id| {
                Ok(match eng.select_tempo_curve(deck_id, id)? {
                    Some(message) => CmdOutcome::DeckUpdatedWithNotice(deck_id, message),
                    None => CmdOutcome::DeckUpdated(deck_id),
                })
            })
        }
        Command::SetKeyLock(bus_api::SetKeyLock { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_key_lock(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetKeyShift(bus_api::SetKeyShift { deck, semitones }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_key_shift(id, *semitones)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetKeyboardRoot(bus_api::SetKeyboardRoot { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_keyboard_root(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetKeyboardPage(bus_api::SetKeyboardPage { deck, page }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_keyboard_page(id, *page)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetKeyShiftPage(bus_api::SetKeyShiftPage { deck, page }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_key_shift_page(id, *page)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::ToggleSync(bus_api::ToggleSync { deck, beat_sync }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let updated = eng.toggle_deck_sync(id, *beat_sync)?;
                Ok(CmdOutcome::DecksUpdated(updated))
            })
        }
        Command::SetMasterDeck(bus_api::SetMasterDeck { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let _updated = eng.set_master_deck(id)?;
                // master_deck + slave speeds/sync live on the engine snapshot decks.
                Ok(CmdOutcome::EngineStatus)
            })
        }

        // ── cue / loop ───────────────────────────────────────────────────────
        Command::SetCuePoint(bus_api::SetCuePoint { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_cue_point_at_playhead(id)?;
                // Persist the snapped cue as the track's manual cue (best effort, like
                // auto-saved loops: a deck action never fails on the library bus).
                if let Err(err) = eng.save_deck_track_cue(id) {
                    tracing::warn!(error = %err, "failed to persist track cue");
                }
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::BeginCueHold(bus_api::BeginCueHold { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.begin_deck_cue_hold(id)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::EndCueHold(bus_api::EndCueHold { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.end_deck_cue_hold(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::SetQuantize(bus_api::SetQuantize { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_quantize(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::ToggleQuantize(bus_api::ToggleQuantize { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                let enabled = !eng.deck_quantize(id).unwrap_or(true);
                eng.set_deck_quantize(id, enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetSlip(bus_api::SetSlip { deck, enabled }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_slip(id, *enabled)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::ToggleSlip(bus_api::ToggleSlip { deck }) => deck_cmd(*deck, engine, |eng, id| {
            let enabled = !eng.deck_slip_enabled(id).unwrap_or(false);
            eng.set_deck_slip(id, enabled)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::SetAutoLoop(bus_api::SetAutoLoop { deck, beats }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_auto_loop(id, *beats)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::LoopIn(bus_api::LoopIn { deck, position_ms }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_loop_in(id, *position_ms)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::LoopOut(bus_api::LoopOut { deck, position_ms }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_loop_out(id, *position_ms)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::ExitLoop(bus_api::ExitLoop { deck }) => deck_cmd(*deck, engine, |eng, id| {
            eng.clear_deck_loop(id)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::RecallSavedLoop(bus_api::RecallSavedLoop {
            deck,
            in_ms,
            out_ms,
        }) => deck_cmd(*deck, engine, |eng, id| {
            eng.recall_deck_saved_loop(id, *in_ms, *out_ms)?;
            Ok(CmdOutcome::DeckUpdated(id))
        }),
        Command::BeginLoopRoll(bus_api::BeginLoopRoll { deck, beats }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.begin_deck_loop_roll(id, *beats)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::EndLoopRoll(bus_api::EndLoopRoll { deck }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.end_deck_loop_roll(id)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::BeatJump(bus_api::BeatJump { deck, beats }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.beat_jump_deck(id, *beats)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── pads ─────────────────────────────────────────────────────────────
        Command::SetPadMode(bus_api::SetPadMode { deck, mode }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_pad_mode(id, *mode)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::PadPress(bus_api::PadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::PadRelease(bus_api::PadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::HotCuePadPress(bus_api::HotCuePadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.hot_cue_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::HotCuePadRelease(bus_api::HotCuePadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.hot_cue_pad_release(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }
        Command::LoopRollPadPress(bus_api::LoopRollPadPress { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.loop_roll_pad_press(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::LoopRollPadRelease(bus_api::LoopRollPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.loop_roll_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::BeatJumpPadPress(bus_api::BeatJumpPadPress { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.beat_jump_pad_press(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::BeatJumpPadRelease(bus_api::BeatJumpPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.beat_jump_pad_release(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }
        Command::SamplerPadPress(bus_api::SamplerPadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.sampler_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SamplerPadRelease(bus_api::SamplerPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.sampler_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::KeyboardPadPress(bus_api::KeyboardPadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.keyboard_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::KeyboardPadRelease(bus_api::KeyboardPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.keyboard_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::KeyShiftPadPress(bus_api::KeyShiftPadPress { deck, slot, shift }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.key_shift_pad_press(id, *slot, *shift)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::KeyShiftPadRelease(bus_api::KeyShiftPadRelease { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.key_shift_pad_release(id, *slot)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::TriggerHotCue(bus_api::TriggerHotCue { deck, position_ms }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.trigger_deck_hot_cue(id, *position_ms)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── hot cues / loops on the deck ─────────────────────────────────────
        Command::SaveDeckHotCue(bus_api::SaveDeckHotCue { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.save_deck_hot_cue(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }
        Command::DeleteDeckHotCue(bus_api::DeleteDeckHotCue { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.delete_deck_hot_cue(id, *slot)?;
                Ok(CmdOutcome::Silent)
            })
        }

        // ── sampler slots ────────────────────────────────────────────────────
        Command::AssignSampler(bus_api::AssignSampler { deck, slot, path }) => {
            dispatch_assign_sampler(*deck, *slot, path, engine)
        }
        Command::AssignSamplerTrack(bus_api::AssignSamplerTrack {
            deck,
            slot,
            track_id,
        }) => dispatch_assign_sampler_track(*deck, *slot, track_id, engine),
        Command::ClearSampler(bus_api::ClearSampler { deck, slot }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.clear_sampler_slot(id, usize::from(*slot))?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── jog ──────────────────────────────────────────────────────────────
        Command::JogTouch(bus_api::JogTouch { deck, touching }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_jog_touch(id, *touching)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::JogTurn(bus_api::JogTurn { deck, delta }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.deck_jog_turn(id, *delta)?;
                // So paused vinyl scrub updates UI playhead/time without waiting for
                // the next 33ms tick (JogTouch release used to be the only cue).
                Ok(CmdOutcome::Position(id))
            })
        }
        Command::SetJogMode(bus_api::SetJogMode { deck, top, outer }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_jog_mode(id, *top, *outer)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }
        Command::SetDeckFxRouted(bus_api::SetDeckFxRouted { deck, routed }) => {
            deck_cmd(*deck, engine, |eng, id| {
                eng.set_deck_fx_routed(id, *routed)?;
                Ok(CmdOutcome::DeckUpdated(id))
            })
        }

        // ── mixer ────────────────────────────────────────────────────────────
        Command::SetCrossfader(bus_api::SetCrossfader {
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
        Command::SetMasterVolume(bus_api::SetMasterVolume {
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
        Command::SetCueVolume(bus_api::SetCueVolume {
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
        Command::SetCueMix(bus_api::SetCueMix { mix, soft_takeover }) => {
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
        Command::SetMasterCue(bus_api::SetMasterCue { enabled }) => {
            with_engine_mut(engine, |eng| {
                eng.set_master_cue(*enabled)?;
                Ok(CmdOutcome::EngineStatus)
            })
        }
        Command::ToggleMasterCue(first) => {
            // `ToggleMasterCue` folds in the subscriber, so a folded pair never
            // reaches the engine and `flipped` is always false here.
            let _ = first;
            with_engine_mut(engine, |eng| {
                let enabled = !eng.master_cue().unwrap_or(false);
                eng.set_master_cue(enabled)?;
                Ok(CmdOutcome::EngineStatus)
            })
        }
        Command::SetCurveLibrary(bus_api::SetCurveLibrary { library }) => {
            with_engine_mut(engine, |eng| {
                eng.set_curve_library(library)?;
                Ok(CmdOutcome::EngineStatus)
            })
        }
        Command::SetCrossfaderCurve(bus_api::SetCrossfaderCurve { id }) => {
            with_engine_mut(engine, |eng| {
                Ok(match eng.select_crossfader_curve(id)? {
                    Some(message) => CmdOutcome::EngineStatusWithNotice(message),
                    None => CmdOutcome::EngineStatus,
                })
            })
        }
        Command::SetCrossfaderCurveAmount(bus_api::SetCrossfaderCurveAmount {
            set,
            amount,
            soft_takeover,
        }) => with_engine_mut(engine, |eng| {
            Ok(
                match eng.select_crossfader_curve_set(set, *amount, *soft_takeover)? {
                    Some(message) => CmdOutcome::EngineStatusWithNotice(message),
                    None => CmdOutcome::EngineStatus,
                },
            )
        }),
        Command::SetCrossfaderBehavior(bus_api::SetCrossfaderBehavior { hamster, disabled }) => {
            with_engine_mut(engine, |eng| {
                eng.apply_crossfader_behavior(bus_api::CrossfaderBehaviorSetting {
                    hamster: *hamster,
                    disabled: *disabled,
                })?;
                Ok(CmdOutcome::EngineStatus)
            })
        }
        Command::SetChannelFaderCurve(bus_api::SetChannelFaderCurve { id }) => {
            with_engine_mut(engine, |eng| {
                Ok(match eng.select_channel_fader_curve(id)? {
                    Some(message) => CmdOutcome::EngineStatusWithNotice(message),
                    None => CmdOutcome::EngineStatus,
                })
            })
        }

        // ── shared FX unit ───────────────────────────────────────────────────
        Command::SetFxUnitSlot(bus_api::SetFxUnitSlot { slot, kind }) => {
            with_engine_mut(engine, |eng| {
                eng.set_fx_unit_slot(*slot, crate::status::map_fx_kind(*kind))?;
                Ok(CmdOutcome::DecksUpdated(
                    (0..eng.config.num_decks).collect(),
                ))
            })
        }
        Command::SetFxUnitSlotEnabled(bus_api::SetFxUnitSlotEnabled { slot, enabled }) => {
            with_engine_mut(engine, |eng| {
                eng.set_fx_unit_slot_enabled(*slot, *enabled)?;
                Ok(CmdOutcome::DecksUpdated(
                    (0..eng.config.num_decks).collect(),
                ))
            })
        }
        Command::SetFxUnitWet(bus_api::SetFxUnitWet {
            slot,
            wet,
            soft_takeover,
        }) => with_engine_mut(engine, |eng| {
            let key = soft_takeover::key_mixer(&format!("fx_wet{slot}"));
            // Compare in the pre-curve control domain: the knob position the
            // controller reports and the level the slot stores share `0..1`.
            let current = eng.fx_unit_level(*slot).unwrap_or(0.5);
            if !gate_mixer_soft_takeover(eng, &key, *soft_takeover, current, *wet) {
                return Ok(CmdOutcome::Silent);
            }
            eng.set_fx_unit_wet(*slot, *wet)?;
            Ok(CmdOutcome::DecksUpdated(
                (0..eng.config.num_decks).collect(),
            ))
        }),
        Command::SetFxLevelCurve(bus_api::SetFxLevelCurve { id }) => {
            with_engine_mut(engine, |eng| {
                Ok(match eng.select_fx_level_curve(id)? {
                    Some(message) => CmdOutcome::EngineStatusWithNotice(message),
                    None => CmdOutcome::EngineStatus,
                })
            })
        }
        Command::SetFxUnitParam(bus_api::SetFxUnitParam { slot, param, value }) => {
            with_engine_mut(engine, |eng| {
                eng.set_fx_unit_param(*slot, *param, *value)?;
                Ok(CmdOutcome::DecksUpdated(
                    (0..eng.config.num_decks).collect(),
                ))
            })
        }

        // ── not the engine's to execute ───────────────────────────────────────
        //
        // These belong to the library worker. Routing sends a command only to
        // its target's single inbox, so the engine never receives one; the arms
        // exist so a new command has to be classified by both dispatchers
        // instead of falling into a catch-all. Answering with an `Error` would
        // turn another owner's valid command into a toast.
        Command::AnalyzeTrack(_)
        | Command::GenerateStems(_)
        | Command::RefreshTrack(_)
        | Command::SaveBeatGrid(_)
        | Command::SaveTrackCue(_)
        | Command::SaveTrackHotCue(_)
        | Command::DeleteTrackHotCue(_)
        | Command::SaveTrackLoop(_)
        | Command::DeleteTrackLoop(_) => Ok(CmdOutcome::Silent),
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

fn tick(engine: &EngineHandle, bus: &EventBus<Command, Event>, peak_hold: &mut PeakHoldState) {
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
        if let Err(err) = publish_deck_updated(bus, eng, deck_id) {
            tracing::error!(error = %err, "failed to publish the ended deck");
        }
    }

    // A live stem producer publishes its first window asynchronously and loses
    // coverage whenever it falls behind; emit one Updated on either edge so
    // pads/banner/stem-EQ state reflect what the deck is really playing.
    for deck_id in 0..eng.config.num_decks {
        let ready = eng.deck_stems_ready(deck_id).unwrap_or(false);
        let audible = eng.deck_stems_audible(deck_id).unwrap_or(false);
        let changed =
            ready != eng.stems_ready_seen[deck_id] || audible != eng.stems_audible_seen[deck_id];
        if changed {
            eng.stems_ready_seen[deck_id] = ready;
            eng.stems_audible_seen[deck_id] = audible;
            if let Err(err) = publish_deck_updated(bus, eng, deck_id) {
                tracing::error!(error = %err, "failed to publish a stems state change");
            }
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

    /// A control loop over a detached engine handle and a private bus.
    ///
    /// `None` for the engine makes `tick` and every dispatch a no-op, so these
    /// tests measure scheduling only.
    fn scheduling_loop<'a>(
        engine: &'a EngineHandle,
        bus: &'a EventBus<Command, Event>,
        shutdown: &'a AtomicBool,
        library: Option<Subscriber<Arc<Event>>>,
        start: Instant,
    ) -> ControlLoop<'a> {
        ControlLoop {
            engine,
            bus,
            shutdown,
            cmd_rx: bus
                .subscribe_cmd(CommandTarget::Engine, "test")
                .expect("engine inbox"),
            library_rx: library,
            peak_hold: PeakHoldState::new(),
            pending_command: None,
            next_tick: start + TICK_INTERVAL,
            command_lane_closed: false,
        }
    }

    fn hot_cues_changed() -> Event {
        Event::HotCuesChanged(bus_api::HotCuesChanged {
            track_id: "t".into(),
            hot_cues: Vec::new(),
        })
    }

    #[test]
    fn a_thousand_queued_commands_do_not_produce_a_thousand_ticks() {
        let bus = EventBus::<Command, Event>::new();
        let engine: EngineHandle = Arc::new(Mutex::new(None));
        let shutdown = AtomicBool::new(false);
        let start = Instant::now();
        let mut control = scheduling_loop(&engine, &bus, &shutdown, None, start);

        for _ in 0..1_000 {
            bus.publish_cmd(Command::SetMasterVolume(bus_api::SetMasterVolume {
                volume: 0.5,
                soft_takeover: false,
            }))
            .expect("engine inbox");
        }

        // A frozen clock: no tick deadline can come due during these passes.
        let mut handled = 0;
        let mut ticks = 0;
        let mut passes = 0;
        while handled < 1_000 && passes < 100 {
            let summary = control.pass(|| start);
            handled += summary.commands;
            ticks += usize::from(summary.ticked);
            passes += 1;
        }
        assert_eq!(handled, 1_000, "every queued command is handled");
        assert_eq!(ticks, 0, "ticks follow the deadline, not the message count");
        assert_eq!(passes, 16, "1,000 commands in bounded 64-command passes");
    }

    #[test]
    fn each_pass_handles_at_most_one_batch_of_each_lane() {
        let bus = EventBus::<Command, Event>::new();
        let engine: EngineHandle = Arc::new(Mutex::new(None));
        let shutdown = AtomicBool::new(false);
        let start = Instant::now();
        let library = bus
            .subscribe_evt_topics(
                "engine-library",
                &[EventTopic::HotCuesChanged, EventTopic::BeatGridChanged],
            )
            .expect("library subscription");
        let mut control = scheduling_loop(&engine, &bus, &shutdown, Some(library), start);

        for _ in 0..1_000 {
            bus.publish_cmd(Command::SetMasterVolume(bus_api::SetMasterVolume {
                volume: 0.5,
                soft_takeover: false,
            }))
            .expect("engine inbox");
        }
        for _ in 0..300 {
            bus.publish_evt(hot_cues_changed()).expect("publish");
        }

        let first = control.pass(|| start);
        assert_eq!(first.commands, MAX_COMMANDS_PER_PASS);
        assert_eq!(first.library_events, MAX_LIBRARY_EVENTS_PER_PASS);
        assert!(!first.ticked);

        let second = control.pass(|| start);
        assert_eq!(second.commands, MAX_COMMANDS_PER_PASS);
        assert_eq!(second.library_events, MAX_LIBRARY_EVENTS_PER_PASS);

        // 300 acknowledgements: 64 + 64, then 64 + 64 + 44.
        for expected in [64, 64, 44] {
            let summary = control.pass(|| start);
            assert_eq!(summary.library_events, expected);
        }
        assert_eq!(
            control.pass(|| start).library_events,
            0,
            "the batch stops at an empty queue"
        );
    }

    #[test]
    fn a_due_tick_still_runs_while_the_library_queue_stays_full() {
        let bus = EventBus::<Command, Event>::new();
        let engine: EngineHandle = Arc::new(Mutex::new(None));
        let shutdown = AtomicBool::new(false);
        let start = Instant::now();
        let library = bus
            .subscribe_evt_topics("engine-library", &[EventTopic::HotCuesChanged])
            .expect("library subscription");
        let mut control = scheduling_loop(&engine, &bus, &shutdown, Some(library), start);
        for _ in 0..1_000 {
            bus.publish_evt(hot_cues_changed()).expect("publish");
        }

        let before = control.pass(|| start);
        assert!(!before.ticked, "the deadline has not passed yet");
        assert_eq!(before.library_events, MAX_LIBRARY_EVENTS_PER_PASS);

        let due = control.pass(|| start + TICK_INTERVAL);
        assert!(due.ticked, "a non-empty queue must not starve the tick");
        assert_eq!(due.library_events, MAX_LIBRARY_EVENTS_PER_PASS);

        let after = control.pass(|| start + TICK_INTERVAL);
        assert!(
            !after.ticked,
            "one late tick is one tick: no catch-up burst"
        );
    }

    #[test]
    fn the_time_budget_bounds_each_lane_separately() {
        let bus = EventBus::<Command, Event>::new();
        let engine: EngineHandle = Arc::new(Mutex::new(None));
        let shutdown = AtomicBool::new(false);
        let start = Instant::now();
        let library = bus
            .subscribe_evt_topics("engine-library", &[EventTopic::HotCuesChanged])
            .expect("library subscription");
        let mut control = scheduling_loop(&engine, &bus, &shutdown, Some(library), start);

        for _ in 0..10 {
            bus.publish_cmd(Command::SetMasterVolume(bus_api::SetMasterVolume {
                volume: 0.5,
                soft_takeover: false,
            }))
            .expect("engine inbox");
        }
        for _ in 0..5 {
            bus.publish_evt(hot_cues_changed()).expect("publish");
        }

        // Stand in for a slow handler: the clock stays put for the first three
        // reads, then jumps past the 2 ms command budget.
        let reads = std::cell::Cell::new(0);
        let clock = || {
            let count = reads.get() + 1;
            reads.set(count);
            if count <= 3 {
                start
            } else {
                start + BATCH_BUDGET + Duration::from_millis(1)
            }
        };
        let summary = control.pass(clock);
        assert_eq!(summary.commands, 2, "the command batch stops on its budget");
        assert_eq!(
            summary.library_events, 5,
            "the library batch keeps its own budget"
        );
    }

    #[test]
    fn each_lane_expires_its_own_time_budget() {
        let bus = EventBus::<Command, Event>::new();
        let engine: EngineHandle = Arc::new(Mutex::new(None));
        let shutdown = AtomicBool::new(false);
        let start = Instant::now();
        let library = bus
            .subscribe_evt_topics("engine-library", &[EventTopic::HotCuesChanged])
            .expect("library subscription");
        let mut control = scheduling_loop(&engine, &bus, &shutdown, Some(library), start);

        for _ in 0..10 {
            bus.publish_cmd(Command::SetMasterVolume(bus_api::SetMasterVolume {
                volume: 0.5,
                soft_takeover: false,
            }))
            .expect("engine inbox");
        }
        for _ in 0..5 {
            bus.publish_evt(hot_cues_changed()).expect("publish");
        }

        // Every read advances 1 ms, so each lane's own 2 ms budget runs out
        // mid-batch: one message each, not one lane draining the other's time.
        let reads = std::cell::Cell::new(0u32);
        let clock = || {
            let count = reads.get();
            reads.set(count + 1);
            start + Duration::from_millis(u64::from(count))
        };
        let summary = control.pass(clock);
        assert_eq!(summary.commands, 1, "the command budget expires on its own");
        assert_eq!(
            summary.library_events, 1,
            "the library budget starts fresh and expires on its own"
        );
        assert!(!summary.ticked);
    }

    #[test]
    fn an_overdue_tick_rebases_instead_of_catching_up() {
        let bus = EventBus::<Command, Event>::new();
        let engine: EngineHandle = Arc::new(Mutex::new(None));
        let shutdown = AtomicBool::new(false);
        let start = Instant::now();
        let mut control = scheduling_loop(&engine, &bus, &shutdown, None, start);

        // Ten intervals late: exactly one tick, not a burst of ten.
        let late = start + TICK_INTERVAL * 10;
        let first = control.pass(|| late);
        assert!(first.ticked, "an overdue tick still runs");
        let second = control.pass(|| late);
        assert!(
            !second.ticked,
            "the deadline is rebased to now, so a second pass at the same instant is early"
        );

        let third = control.pass(|| late + TICK_INTERVAL);
        assert!(
            third.ticked,
            "the next deadline is one interval after the rebase"
        );
    }

    #[test]
    fn a_shutdown_flag_stops_a_batch_between_messages() {
        let bus = EventBus::<Command, Event>::new();
        let engine: EngineHandle = Arc::new(Mutex::new(None));
        let shutdown = AtomicBool::new(true);
        let start = Instant::now();
        let mut control = scheduling_loop(&engine, &bus, &shutdown, None, start);
        for _ in 0..10 {
            bus.publish_cmd(Command::SetMasterVolume(bus_api::SetMasterVolume {
                volume: 0.5,
                soft_takeover: false,
            }))
            .expect("engine inbox");
        }
        let summary = control.pass(|| start);
        assert_eq!(summary.commands, 0, "a stopping worker handles nothing new");
    }
}
