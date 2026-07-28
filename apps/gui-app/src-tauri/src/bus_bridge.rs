//! Tauri bridge for engine cmd/evt omnibus (MessagePack wire bytes).

use engine_api::{decode_cmd_body, decode_wire, encode_wire, CmdBody, Kind, Origin, WireMessage};
use engine_core::{EngineSession, Evt};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

use crate::deck_sampler::SamplerPlayModeSetting;

pub const ENGINE_BUS_EVENT: &str = "engine://bus";

pub type SharedSession = Arc<Mutex<Option<Arc<EngineSession>>>>;

pub struct EvtForwarder {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn is_high_rate(kind: &Kind) -> bool {
    matches!(kind, Kind::Position | Kind::Levels)
}

impl EvtForwarder {
    pub fn start(app: AppHandle, session: Arc<EngineSession>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let rx = session.subscribe_evt_all().expect("evt bus subscribe");
        let thread = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let first = match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(ev)) => ev,
                    Ok(None) => continue,
                    Err(_) => break,
                };

                // Drain the queue: keep every discrete evt, coalesce high-rate by origin+kind.
                // Prevents Position/Levels from starving Pause/Updated when emit is slow.
                let mut discrete: Vec<Arc<Evt>> = Vec::new();
                let mut high_rate: HashMap<(Origin, Kind), Arc<Evt>> = HashMap::new();
                let mut push = |ev: Arc<Evt>| {
                    if is_high_rate(ev.kind()) {
                        high_rate.insert((ev.origin().clone(), ev.kind().clone()), ev);
                    } else {
                        discrete.push(ev);
                    }
                };
                push(first);
                loop {
                    match rx.recv() {
                        Ok(Some(ev)) => push(ev),
                        Ok(None) => break,
                        Err(_) => return,
                    }
                }

                for ev in discrete.into_iter().chain(high_rate.into_values()) {
                    let Ok(data) = encode_wire(&WireMessage {
                        origin: ev.origin().clone(),
                        kind: ev.kind().clone(),
                        revision: session.revision(),
                        action_timestamp_ms: 0,
                        body: ev.payload().as_ref().to_vec(),
                    }) else {
                        continue;
                    };
                    if let Err(err) = app.emit(ENGINE_BUS_EVENT, data) {
                        log::warn!("failed to emit {ENGINE_BUS_EVENT}: {err}");
                    }
                }
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for EvtForwarder {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn new_shared_session() -> SharedSession {
    Arc::new(Mutex::new(None))
}

pub fn install_session(holder: &SharedSession, session: Arc<EngineSession>) {
    *holder.lock().expect("shared session lock") = Some(session);
}

pub fn clear_session(holder: &SharedSession) {
    *holder.lock().expect("shared session lock") = None;
}

#[tauri::command]
pub fn engine_publish(
    app: AppHandle,
    session: State<'_, SharedSession>,
    app_state: State<'_, crate::SharedAppState>,
    payload: Vec<u8>,
) -> Result<(), String> {
    let msg = decode_wire(&payload).map_err(|e| e.to_string())?;

    // Host-handled sampler bank/slot cmds (library + AppState); do not forward to omnibus.
    if let Origin::Deck(deck_id) = msg.origin {
        let deck_id = deck_id as usize;
        match msg.kind {
            Kind::AssignSampler => {
                let CmdBody::AssignSampler { slot, path } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("assign_sampler body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_sampler::assign_sampler_slot_inner(
                    &app,
                    &mut state,
                    slot as usize,
                    path,
                    None,
                    deck_id,
                )?;
                return Ok(());
            }
            Kind::AssignSamplerTrack => {
                let CmdBody::AssignSamplerTrack { slot, track_id } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("assign_sampler_track body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_sampler::assign_sampler_slot_from_track_inner(
                    &app,
                    &mut state,
                    slot as usize,
                    track_id,
                    None,
                    deck_id,
                )?;
                return Ok(());
            }
            Kind::ClearSampler => {
                let CmdBody::ClearSampler { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("clear_sampler body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_sampler::clear_sampler_slot_inner(
                    &app,
                    &mut state,
                    slot as usize,
                    None,
                    deck_id,
                )?;
                return Ok(());
            }
            Kind::SetSamplerBank => {
                let CmdBody::SetSamplerBank { bank_id } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("set_sampler_bank body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_sampler::set_deck_sampler_bank_inner(&app, &mut state, deck_id, bank_id)?;
                return Ok(());
            }
            Kind::CreateSamplerBank => {
                let CmdBody::CreateSamplerBank { name, play_mode } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("create_sampler_bank body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_sampler::create_sampler_bank_inner(
                    &app,
                    &mut state,
                    deck_id,
                    name,
                    play_mode
                        .as_deref()
                        .map(SamplerPlayModeSetting::from_str)
                        .transpose()
                        .map_err(|e| e.to_string())?,
                )?;
                return Ok(());
            }
            Kind::SaveHotCue => {
                let CmdBody::SaveHotCue { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("save_hot_cue body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_performance::save_hot_cue_inner(&app, &mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::DeleteHotCue => {
                let CmdBody::DeleteHotCue { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("delete_hot_cue body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_performance::delete_hot_cue_inner(&app, &mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::SaveLoop => {
                let CmdBody::SaveLoop { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("save_loop body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_performance::save_loop_inner(&app, &mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::DeleteLoop => {
                let CmdBody::DeleteLoop { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("delete_loop body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_performance::delete_loop_inner(&app, &mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::LoadPath => {
                let CmdBody::LoadPath { path } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("load_path body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::load_path_to_deck_inner(&app, &mut state, deck_id, path)?;
                return Ok(());
            }
            Kind::LoadLibraryTrack => {
                let CmdBody::LoadLibraryTrack { track_id } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("load_library_track body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::load_library_track_to_deck_inner(&app, &mut state, deck_id, track_id)?;
                return Ok(());
            }
            _ => {}
        }
    }

    if msg.origin == Origin::Mixer {
        match msg.kind {
            Kind::UpdateSamplerBank => {
                let CmdBody::UpdateSamplerBank {
                    bank_id,
                    name,
                    play_mode,
                } = decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("update_sampler_bank body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_sampler::update_sampler_bank_inner(
                    &app,
                    &mut state,
                    bank_id,
                    name,
                    play_mode
                        .as_deref()
                        .map(SamplerPlayModeSetting::from_str)
                        .transpose()
                        .map_err(|e| e.to_string())?,
                )?;
                return Ok(());
            }
            Kind::DeleteSamplerBank => {
                let CmdBody::DeleteSamplerBank { bank_id } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("delete_sampler_bank body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_sampler::delete_sampler_bank_inner(&app, &mut state, bank_id)?;
                return Ok(());
            }
            _ => {}
        }
    }

    // ponytail: AppState still owns track metadata until load migrates; clear on Unload so
    // leftover library invokes don't see a ghost track.
    if matches!((&msg.origin, &msg.kind), (Origin::Deck(_), Kind::Unload)) {
        let Origin::Deck(deck_id) = msg.origin else {
            unreachable!()
        };
        let deck_id = deck_id as usize;
        if deck_id < crate::NUM_DECKS {
            let mut state = app_state.lock().map_err(|e| e.to_string())?;
            crate::clear_deck_info(&mut state.decks[deck_id]);
        }
    }
    // ponytail: AppState.pad_mode still mirrors for leftover sampler bank/assign invokes.
    if matches!((&msg.origin, &msg.kind), (Origin::Deck(_), Kind::SetPadMode)) {
        let Origin::Deck(deck_id) = msg.origin else {
            unreachable!()
        };
        let deck_id = deck_id as usize;
        if deck_id < crate::NUM_DECKS {
            if let Ok(CmdBody::SetPadMode { mode }) = decode_cmd_body(&msg.body) {
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                state.decks[deck_id].pad_mode = match mode {
                    engine_api::PadMode::HotCue => crate::deck_sync::PadMode::HotCue,
                    engine_api::PadMode::LoopRoll => crate::deck_sync::PadMode::LoopRoll,
                    engine_api::PadMode::BeatJump => crate::deck_sync::PadMode::BeatJump,
                    engine_api::PadMode::Sampler => crate::deck_sync::PadMode::Sampler,
                };
                if mode == engine_api::PadMode::Sampler {
                    let _ = crate::deck_sampler::ensure_deck_bank_loaded(&mut state, deck_id);
                }
            }
        }
    }
    // ponytail: bank load + play mode + last-used bank stay host-owned until bank cmds migrate.
    let mut remember_sampler_bank: Option<(String, String)> = None;
    if matches!((&msg.origin, &msg.kind), (Origin::Deck(_), Kind::TriggerSampler)) {
        let Origin::Deck(deck_id) = msg.origin else {
            unreachable!()
        };
        let deck_id = deck_id as usize;
        if deck_id < crate::NUM_DECKS {
            let mut state = app_state.lock().map_err(|e| e.to_string())?;
            crate::deck_sampler::apply_effective_play_mode(&mut state, deck_id)?;
            crate::deck_sampler::ensure_deck_bank_loaded(&mut state, deck_id)?;
            if let (Some(track_id), Some(bank_id)) = (
                state.decks[deck_id].track_id.clone(),
                state.decks[deck_id].active_sampler_bank_id.clone(),
            ) {
                remember_sampler_bank = Some((track_id, bank_id));
            }
        }
    }
    let guard = session.lock().map_err(|e| e.to_string())?;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Engine session not running.".to_string())?;
    session
        .publish_cmd(msg.origin, msg.kind, msg.body)
        .map_err(|e| e.to_string())?;
    if let Some((track_id, bank_id)) = remember_sampler_bank {
        let state = app_state.lock().map_err(|e| e.to_string())?;
        let _ = state
            .library
            .set_track_last_sampler_bank_id(&library_core::TrackId::new(track_id), Some(&bank_id));
    }
    Ok(())
}
