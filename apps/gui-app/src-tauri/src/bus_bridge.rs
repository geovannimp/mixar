//! Tauri bridge for engine cmd/evt omnibus (MessagePack wire bytes).

use engine_api::{
    decode_cmd_body, decode_wire, encode_wire, CmdBody, DeckHotCue as ApiDeckHotCue,
    DeckSavedLoop as ApiDeckSavedLoop, DeckSnapshot, EvtBody, Kind, Origin,
    PadMode as ApiPadMode, SamplerBankInfo as ApiSamplerBankInfo,
    SamplerPlayMode as ApiSamplerPlayMode, SamplerSlotInfo as ApiSamplerSlotInfo,
    SamplerStatus as ApiSamplerStatus, WireMessage,
};
use engine_core::{EngineSession, Evt};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

use crate::deck_performance::{HotCueStatus, SavedLoopStatus};
use crate::deck_sampler::{SamplerPlayModeSetting, SamplerStatus};
use crate::deck_sync::PadMode;
use crate::{AppState, DeckInfo, NUM_DECKS};

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

    if msg.origin == Origin::Engine {
        match msg.kind {
            Kind::StartEngine => {
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::start_engine_inner(&app, &mut state, session.inner())?;
                return Ok(());
            }
            _ => {}
        }
    }

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
                crate::deck_sampler::set_deck_sampler_bank_inner(
                    &mut state, deck_id, bank_id)?;
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
                crate::deck_performance::save_hot_cue_inner(&mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::DeleteHotCue => {
                let CmdBody::DeleteHotCue { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("delete_hot_cue body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_performance::delete_hot_cue_inner(&mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::SaveLoop => {
                let CmdBody::SaveLoop { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("save_loop body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_performance::save_loop_inner(&mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::DeleteLoop => {
                let CmdBody::DeleteLoop { slot } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("delete_loop body mismatch".into());
                };
                let mut state = app_state.lock().map_err(|e| e.to_string())?;
                crate::deck_performance::delete_loop_inner(&mut state, deck_id, slot)?;
                return Ok(());
            }
            Kind::LoadPath => {
                let CmdBody::LoadPath { path } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("load_path body mismatch".into());
                };
                // Decode/prepare outside AppState — holding the lock during decode freezes the UI.
                let library = {
                    let state = app_state.lock().map_err(|e| e.to_string())?;
                    Arc::clone(&state.library)
                };
                let prepared = library::LibraryManager::prepare_file_path_for_playback(
                    library.as_ref(),
                    std::path::Path::new(&path),
                )
                .map_err(|e| e.to_string())?;
                {
                    let mut state = app_state.lock().map_err(|e| e.to_string())?;
                    crate::load_prepared_to_deck_inner(&mut state, deck_id, path, prepared)?;
                    publish_deck_updated(&state, deck_id);
                }
                // Sampler bank after first Updated so UI isn't starved during bank slot loads.
                {
                    let mut state = app_state.lock().map_err(|e| e.to_string())?;
                    let track_id = state.decks[deck_id].track_id.clone();
                    let _ = crate::deck_sampler::select_bank_for_track_load(
                        &mut state,
                        deck_id,
                        track_id.as_deref(),
                    );
                    publish_deck_updated(&state, deck_id);
                }
                return Ok(());
            }
            Kind::LoadLibraryTrack => {
                let CmdBody::LoadLibraryTrack { track_id } =
                    decode_cmd_body(&msg.body).map_err(|e| e.to_string())?
                else {
                    return Err("load_library_track body mismatch".into());
                };
                let library = {
                    let state = app_state.lock().map_err(|e| e.to_string())?;
                    Arc::clone(&state.library)
                };
                let prepared = library::LibraryManager::prepare_track_for_playback(
                    library.as_ref(),
                    &library_core::TrackId::new(track_id),
                )
                .map_err(|e| e.to_string())?;
                let path = prepared
                    .source
                    .file()
                    .ok_or_else(|| "Only file tracks can be loaded to a deck.".to_string())?
                    .path()
                    .to_string_lossy()
                    .into_owned();
                {
                    let mut state = app_state.lock().map_err(|e| e.to_string())?;
                    crate::load_prepared_to_deck_inner(&mut state, deck_id, path, prepared)?;
                    publish_deck_updated(&state, deck_id);
                }
                {
                    let mut state = app_state.lock().map_err(|e| e.to_string())?;
                    let track_id = state.decks[deck_id].track_id.clone();
                    let _ = crate::deck_sampler::select_bank_for_track_load(
                        &mut state,
                        deck_id,
                        track_id.as_deref(),
                    );
                    publish_deck_updated(&state, deck_id);
                }
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
                crate::deck_sampler::delete_sampler_bank_inner(&mut state, bank_id)?;
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
            .lock()
            .unwrap()
            .set_track_last_sampler_bank_id(&library_core::TrackId::new(track_id), Some(&bank_id));
    }
    Ok(())
}

fn to_api_pad_mode(mode: PadMode) -> ApiPadMode {
    match mode {
        PadMode::HotCue => ApiPadMode::HotCue,
        PadMode::LoopRoll => ApiPadMode::LoopRoll,
        PadMode::BeatJump => ApiPadMode::BeatJump,
        PadMode::Sampler => ApiPadMode::Sampler,
    }
}

fn to_api_play_mode(mode: SamplerPlayModeSetting) -> ApiSamplerPlayMode {
    match mode {
        SamplerPlayModeSetting::Oneshot => ApiSamplerPlayMode::Oneshot,
        SamplerPlayModeSetting::Hold => ApiSamplerPlayMode::Hold,
        SamplerPlayModeSetting::Loop => ApiSamplerPlayMode::Loop,
    }
}

fn to_api_hot_cues(cues: &[HotCueStatus]) -> Vec<ApiDeckHotCue> {
    cues.iter()
        .map(|cue| ApiDeckHotCue {
            slot: cue.slot,
            position_ms: cue.position_ms,
            loop_length_beats: cue.loop_length_beats,
            color: cue.color.clone(),
            label: cue.label.clone(),
        })
        .collect()
}

fn to_api_saved_loops(loops: &[SavedLoopStatus]) -> Vec<ApiDeckSavedLoop> {
    loops
        .iter()
        .map(|saved| ApiDeckSavedLoop {
            slot: saved.slot,
            in_ms: saved.in_ms,
            out_ms: saved.out_ms,
            label: saved.label.clone(),
            color: saved.color.clone(),
        })
        .collect()
}

/// Overlay host-owned enrichment onto an engine transport snapshot.
fn overlay_host_enrichment(snap: &mut DeckSnapshot, deck: &DeckInfo) {
    snap.track = deck.track.clone();
    snap.track_id = deck.track_id.clone();
    snap.title = deck.title.clone();
    snap.artist = deck.artist.clone();
    snap.bpm = deck.bpm;
    snap.key = deck.key.clone();
    snap.hot_cues = to_api_hot_cues(&deck.hot_cues);
    snap.saved_loops = to_api_saved_loops(&deck.saved_loops);
    snap.loudness_lufs = deck.loudness_lufs;
    snap.auto_gain_db = deck.auto_gain_db;
    snap.active_sampler_bank_id = deck.active_sampler_bank_id.clone();
    // ponytail: AppState.pad_mode still mirrors for leftover sampler invokes.
    snap.pad_mode = to_api_pad_mode(deck.pad_mode);
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

fn to_api_sampler_status(status: SamplerStatus) -> ApiSamplerStatus {
    ApiSamplerStatus {
        banks: status
            .banks
            .into_iter()
            .map(|bank| ApiSamplerBankInfo {
                id: bank.id,
                name: bank.name,
                play_mode: bank.play_mode.map(to_api_play_mode),
                sort_index: bank.sort_index,
            })
            .collect(),
        active_bank_id: status.active_bank_id,
        active_bank_name: status.active_bank_name,
        bank_play_mode: status.bank_play_mode.map(to_api_play_mode),
        deck_slots: status
            .deck_slots
            .into_iter()
            .map(|slots| {
                slots
                    .into_iter()
                    .map(|slot| ApiSamplerSlotInfo {
                        label: slot.label,
                        track_id: slot.track_id,
                        path: slot.path,
                        duration_ms: slot.duration_ms,
                    })
                    .collect()
            })
            .collect(),
        effective_play_modes: status
            .effective_play_modes
            .into_iter()
            .map(to_api_play_mode)
            .collect(),
    }
}

/// Publish enriched `DeckUpdated` onto the session evt bus (`EvtForwarder` → UI).
pub fn publish_deck_updated(state: &AppState, deck_id: usize) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let Ok(Some(mut snap)) = session.with_engine(|eng| Ok(eng.deck_snapshot(deck_id))) else {
        log::warn!("publish_deck_updated: deck {deck_id} snapshot unavailable");
        return;
    };
    if deck_id < NUM_DECKS {
        overlay_host_enrichment(&mut snap, &state.decks[deck_id]);
    }
    if let Err(err) = session.publish_evt(
        Origin::Deck(deck_id as u16),
        Kind::Updated,
        deck_snapshot_to_evt(snap),
    ) {
        log::warn!("publish_deck_updated: {err}");
    }
}

/// Publish enriched `EngineStatus` onto the session evt bus (`EvtForwarder` → UI).
pub fn publish_engine_status(state: &AppState) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let Ok(Some(mut status)) = session.with_engine(|eng| Ok(eng.engine_status_snapshot())) else {
        log::warn!("publish_engine_status: snapshot unavailable");
        return;
    };
    for snap in &mut status.decks {
        let id = snap.id as usize;
        if id < NUM_DECKS {
            overlay_host_enrichment(snap, &state.decks[id]);
        }
    }
    status.sampler = to_api_sampler_status(SamplerStatus::from_state(state));
    status.running = true;
    if let Err(err) = session.publish_evt(
        Origin::Mixer,
        Kind::Status,
        EvtBody::EngineStatus { status },
    ) {
        log::warn!("publish_engine_status: {err}");
    }
}
