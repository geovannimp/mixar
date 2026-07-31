//! Control thread: cmd dispatch and high-rate evt egress.

use crate::bus::EngineBus;
use crate::engine::Engine;
use crate::transport::TransportEvent;
use anyhow::{anyhow, Result};
use engine_api::{decode_cmd_body, encode_evt_body, CmdBody, DeckSnapshot, EvtBody, Kind, Origin};
use omnibus::Event;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TICK_INTERVAL: Duration = Duration::from_millis(33);
const PEAK_HOLD_DECAY_PER_TICK: f32 = 0.04;
const LEVEL_IDLE_EPSILON: f32 = 1e-5;

enum CmdOutcome {
    DeckUpdated(usize),
    DecksUpdated(Vec<usize>),
    EngineStatus,
    Silent,
}

struct PeakHoldState {
    hold_l: Vec<f32>,
    hold_r: Vec<f32>,
}

impl PeakHoldState {
    fn new() -> Self {
        Self {
            hold_l: vec![0.0; 2],
            hold_r: vec![0.0; 2],
        }
    }

    fn ensure_capacity(&mut self, deck_id: usize) {
        let need = deck_id + 1;
        if self.hold_l.len() < need {
            self.hold_l.resize(need, 0.0);
            self.hold_r.resize(need, 0.0);
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
}

pub fn control_thread_loop(
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
    let _ = ready.send(Ok(()));
    let mut peak_hold = PeakHoldState::new();

    while !shutdown.load(Ordering::Relaxed) {
        match rx.recv_timeout(TICK_INTERVAL) {
            Ok(Some(event)) => handle_cmd_event(&event, &engine, &evt_bus, &revision),
            Ok(None) => {}
            Err(_) => break,
        }
        tick(&engine, &evt_bus, &revision, &mut peak_hold);
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

fn deck_snapshot_to_evt(snap: DeckSnapshot) -> EvtBody {
    EvtBody::DeckUpdated {
        id: snap.id,
        track: snap.track,
        track_id: snap.track_id,
        title: snap.title,
        artist: snap.artist,
        bpm: snap.bpm,
        key: snap.key,
        playing: snap.playing,
        volume: snap.volume,
        speed: snap.speed,
        eq: snap.eq,
        filter_db: snap.filter_db,
        gain_trim_db: snap.gain_trim_db,
        headphone_cue: snap.headphone_cue,
        sync_mode: snap.sync_mode,
        cue_point_ms: snap.cue_point_ms,
        quantize: snap.quantize,
        active_loop: snap.active_loop,
        pad_mode: snap.pad_mode,
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
                publish_error(evt_bus, origin, e.to_string());
            }
        }
        Ok(CmdOutcome::DecksUpdated(deck_ids)) => {
            for deck_id in deck_ids {
                if let Err(e) = with_engine_ref(engine, |eng| {
                    publish_deck_updated(evt_bus, revision, eng, deck_id)
                }) {
                    publish_error(evt_bus, origin.clone(), e.to_string());
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
        Ok(CmdOutcome::Silent) => {}
        Err(e) => publish_error(evt_bus, origin, e.to_string()),
    }
}

fn decode_cmd_body_for(kind: Kind, payload: &[u8]) -> Result<CmdBody> {
    let body = decode_cmd_body(payload).map_err(|e| anyhow!("invalid cmd body: {e}"))?;
    match (&kind, &body) {
        (
            Kind::Play
            | Kind::Pause
            | Kind::SetMasterDeck
            | Kind::Unload
            | Kind::SetCuePoint
            | Kind::BeginCueHold
            | Kind::EndCueHold
            | Kind::LoopIn
            | Kind::LoopOut
            | Kind::ExitLoop
            | Kind::EndLoopRoll,
            CmdBody::Empty,
        ) => Ok(body),
        (Kind::Seek, CmdBody::Seek { .. })
        | (Kind::SetVolume, CmdBody::SetVolume { .. })
        | (Kind::SetEq, CmdBody::SetEq { .. })
        | (Kind::SetSpeed, CmdBody::SetSpeed { .. })
        | (Kind::SetFilter, CmdBody::SetFilter { .. })
        | (Kind::SetGainTrim, CmdBody::SetGainTrim { .. })
        | (Kind::SetHeadphoneCue, CmdBody::SetHeadphoneCue { .. })
        | (Kind::ToggleSync, CmdBody::ToggleSync { .. })
        | (Kind::SetQuantize, CmdBody::SetQuantize { .. })
        | (Kind::SetAutoLoop, CmdBody::SetAutoLoop { .. })
        | (Kind::BeatJump, CmdBody::BeatJump { .. })
        | (Kind::SetPadMode, CmdBody::SetPadMode { .. })
        | (Kind::BeginLoopRoll, CmdBody::BeginLoopRoll { .. })
        | (Kind::TriggerHotCue, CmdBody::TriggerHotCue { .. })
        | (Kind::RecallSavedLoop, CmdBody::RecallSavedLoop { .. })
        | (Kind::TriggerSampler, CmdBody::TriggerSampler { .. })
        | (Kind::EndSampler, CmdBody::EndSampler { .. })
        | (Kind::SetCrossfader, CmdBody::SetCrossfader { .. })
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
        Kind::Seek => {
            let CmdBody::Seek { position_ms } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.seek_deck(deck_id, position_ms)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetVolume => {
            let CmdBody::SetVolume { volume } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_volume(deck_id, volume)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetEq => {
            let CmdBody::SetEq { low, mid, high } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_eq_bands(deck_id, low, mid, high)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetSpeed => {
            let CmdBody::SetSpeed { speed } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            let updated = eng.set_deck_speed(deck_id, speed)?;
            Ok(CmdOutcome::DecksUpdated(updated))
        }
        Kind::SetFilter => {
            let CmdBody::SetFilter { filter_db } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_filter_db(deck_id, filter_db)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetGainTrim => {
            let CmdBody::SetGainTrim { gain_db } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_gain_trim_db(deck_id, gain_db)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::SetHeadphoneCue => {
            let CmdBody::SetHeadphoneCue { enabled } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
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
        Kind::SetAutoLoop => {
            let CmdBody::SetAutoLoop { beats } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_deck_auto_loop(deck_id, beats)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::LoopIn => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.set_deck_loop_in_at_playhead(deck_id)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::LoopOut => {
            let _ = decode_cmd_body_for(kind, payload)?;
            eng.set_deck_loop_out_at_playhead(deck_id)?;
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
        Kind::TriggerHotCue => {
            let CmdBody::TriggerHotCue { position_ms } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.trigger_deck_hot_cue(deck_id, position_ms)?;
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
        Kind::TriggerSampler => {
            let CmdBody::TriggerSampler { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.trigger_deck_sampler(deck_id, slot as usize)?;
            Ok(CmdOutcome::DeckUpdated(deck_id))
        }
        Kind::EndSampler => {
            let CmdBody::EndSampler { slot } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.end_deck_sampler(deck_id, slot as usize)?;
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
            Ok(CmdOutcome::Silent)
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

fn dispatch_mixer_cmd(
    kind: Kind,
    payload: &[u8],
    engine: &Arc<Mutex<Option<Engine>>>,
) -> Result<CmdOutcome> {
    with_engine_mut(engine, |eng| match kind {
        Kind::SetCrossfader => {
            let CmdBody::SetCrossfader { position } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
            eng.set_crossfader(position)?;
            Ok(CmdOutcome::EngineStatus)
        }
        Kind::SetCueMix => {
            let CmdBody::SetCueMix { mix } = decode_cmd_body_for(kind, payload)? else {
                unreachable!()
            };
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

    let playback = eng.deck_playback_snapshot();
    for (deck_id, position, _duration) in playback {
        if eng.deck_is_playing(deck_id) == Some(true) {
            publish_evt(
                evt_bus,
                Origin::Deck(deck_id as u16),
                Kind::Position,
                EvtBody::Position {
                    position_ms: position,
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
            continue;
        }
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
}
