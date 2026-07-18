//! Phase 2 deck performance commands (seek, cue, loops, hot cues).

use library::{HotCueRecord, LoopRecord};
use library_core::TrackId;
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::{
    bump_revision, clear_deck_info, deck_playback_secs, deck_status, with_engine, AppState,
    DeckInfo, DeckStatus, SharedAppState, NUM_DECKS,
};

#[derive(Debug, Clone, Serialize)]
pub struct HotCueStatus {
    pub slot: u8,
    pub position_secs: f64,
    pub loop_length_beats: Option<i32>,
    pub color: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedLoopStatus {
    pub slot: u8,
    pub in_secs: f64,
    pub out_secs: f64,
    pub label: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoopRegionStatus {
    pub in_secs: f64,
    pub out_secs: f64,
    pub active: bool,
}

pub fn snap_secs(secs: f64, bpm: Option<f64>, quantize: bool) -> f64 {
    if !quantize {
        return secs.max(0.0);
    }
    let Some(bpm) = bpm else {
        return secs.max(0.0);
    };
    if bpm <= 0.0 {
        return secs.max(0.0);
    }
    let beat = 60.0 / bpm;
    ((secs / beat).round() * beat).max(0.0)
}

fn hot_cue_from_record(record: HotCueRecord) -> HotCueStatus {
    HotCueStatus {
        slot: record.slot_index,
        position_secs: record.position_secs,
        loop_length_beats: record.loop_length_beats,
        color: record.color,
        label: record.label,
    }
}

fn saved_loop_from_record(record: LoopRecord) -> SavedLoopStatus {
    SavedLoopStatus {
        slot: record.slot_index,
        in_secs: record.in_secs,
        out_secs: record.out_secs,
        label: record.label,
        color: record.color,
    }
}

pub fn fetch_deck_performance(
    library: &library::LibraryManager,
    track_id: Option<&str>,
) -> (Vec<HotCueStatus>, Vec<SavedLoopStatus>) {
    let Some(track_id) = track_id else {
        return (Vec::new(), Vec::new());
    };

    let id = TrackId::new(track_id);
    let hot_cues = library
        .list_track_hot_cues(&id)
        .map(|cues| cues.into_iter().map(hot_cue_from_record).collect())
        .unwrap_or_default();
    let saved_loops = library
        .list_track_loops(&id)
        .map(|loops| loops.into_iter().map(saved_loop_from_record).collect())
        .unwrap_or_default();
    (hot_cues, saved_loops)
}

pub fn apply_deck_performance(
    deck: &mut DeckInfo,
    hot_cues: Vec<HotCueStatus>,
    saved_loops: Vec<SavedLoopStatus>,
    reset_transport: bool,
) {
    if reset_transport {
        deck.quantize = true;
        deck.cue_point_secs = Some(0.0);
        deck.active_loop = None;
        deck.sync_mode = crate::SyncMode::Off;
        deck.speed = 1.0;
    }
    deck.hot_cues = hot_cues;
    deck.saved_loops = saved_loops;
}

pub fn transport_snapshot_from_engine(
    state: &AppState,
    deck_id: usize,
) -> Option<(Option<f64>, Option<(f64, f64)>)> {
    state.engine.as_ref()?.deck_transport_state(deck_id)
}

pub fn apply_transport_snapshot(
    deck: &mut DeckInfo,
    snapshot: Option<(Option<f64>, Option<(f64, f64)>)>,
) {
    let Some((cue_point_secs, loop_region)) = snapshot else {
        return;
    };
    deck.cue_point_secs = cue_point_secs;
    deck.active_loop = loop_region.map(|(in_secs, out_secs)| LoopRegionStatus {
        in_secs,
        out_secs,
        active: true,
    });
}

fn deck_status_with_transport(state: &AppState, id: usize, deck: &DeckInfo) -> DeckStatus {
    let mut status = deck_status(state, id, deck);
    status.cue_point_secs = deck.cue_point_secs;
    status.quantize = deck.quantize;
    status.hot_cues = deck.hot_cues.clone();
    status.saved_loops = deck.saved_loops.clone();
    status.active_loop = deck.active_loop.clone();
    status
}

fn publish_deck_transport(app: &AppHandle, state: &mut AppState, deck_id: usize) -> DeckStatus {
    let revision = bump_revision(state);
    let deck = deck_status_with_transport(state, deck_id, &state.decks[deck_id]);
    crate::engine_events::emit_deck_updated(app, revision, deck.clone());
    deck
}

#[tauri::command]
pub fn seek_deck(
    app: AppHandle,
    deck_id: usize,
    position_secs: f64,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.decks[deck_id].track.is_none() {
        return Err("Load a track before seeking.".to_string());
    }

    let snapped = snap_secs(
        position_secs,
        state.decks[deck_id].bpm,
        state.decks[deck_id].quantize,
    );
    with_engine(&mut state, |engine| {
        engine
            .seek_deck(deck_id, snapped)
            .map_err(|e| e.to_string())
    })?;
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn unload_deck(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if let Some(engine) = state.engine.as_mut() {
        engine.unload_deck(deck_id).map_err(|e| e.to_string())?;
    }
    clear_deck_info(&mut state.decks[deck_id]);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_cue_point(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.decks[deck_id].track.is_none() {
        return Err("Load a track before setting cue.".to_string());
    }

    let (position_secs, _) = deck_playback_secs(&state, deck_id);
    let raw = position_secs.unwrap_or(0.0);
    let target = snap_secs(raw, state.decks[deck_id].bpm, state.decks[deck_id].quantize);

    with_engine(&mut state, |engine| {
        engine
            .set_deck_cue_point(deck_id, target)
            .map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].cue_point_secs = Some(target);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn begin_deck_cue_hold(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    with_engine(&mut state, |engine| {
        engine
            .begin_deck_cue_hold(deck_id)
            .map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].playing = true;
    let transport = transport_snapshot_from_engine(&state, deck_id);
    apply_transport_snapshot(&mut state.decks[deck_id], transport);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn end_deck_cue_hold(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    with_engine(&mut state, |engine| {
        engine.end_deck_cue_hold(deck_id).map_err(|e| e.to_string())
    })?;
    let transport = transport_snapshot_from_engine(&state, deck_id);
    apply_transport_snapshot(&mut state.decks[deck_id], transport);
    state.decks[deck_id].playing = state
        .engine
        .as_ref()
        .and_then(|engine| engine.deck_is_playing(deck_id))
        .unwrap_or(false);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_quantize(
    app: AppHandle,
    deck_id: usize,
    enabled: bool,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.decks[deck_id].quantize = enabled;
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_auto_loop(
    app: AppHandle,
    deck_id: usize,
    beats: u32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if beats == 0 {
        return Err("Loop length must be at least 1 beat.".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let deck = &state.decks[deck_id];
    let bpm = deck
        .bpm
        .ok_or_else(|| "Track BPM is required for auto loop.".to_string())?;
    let (position_secs, duration_secs) = deck_playback_secs(&state, deck_id);
    let position = position_secs.unwrap_or(0.0);
    let duration = duration_secs.unwrap_or(0.0);
    let beat_len = 60.0 / bpm;
    let snapped = snap_secs(position, Some(bpm), deck.quantize);
    let in_secs = snapped;
    let out_secs = (snapped + beat_len * beats as f64).min(duration.max(in_secs + beat_len));

    with_engine(&mut state, |engine| {
        engine
            .set_deck_loop_region(deck_id, in_secs, out_secs)
            .map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].active_loop = Some(LoopRegionStatus {
        in_secs,
        out_secs,
        active: true,
    });
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_loop_in(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let (position_secs, _) = deck_playback_secs(&state, deck_id);
    let in_secs = snap_secs(
        position_secs.unwrap_or(0.0),
        state.decks[deck_id].bpm,
        state.decks[deck_id].quantize,
    );
    let out_secs = state.decks[deck_id]
        .active_loop
        .as_ref()
        .map(|region| region.out_secs)
        .unwrap_or(in_secs + 60.0 / state.decks[deck_id].bpm.unwrap_or(120.0) * 4.0);

    with_engine(&mut state, |engine| {
        engine
            .set_deck_loop_region(deck_id, in_secs, out_secs.max(in_secs + 0.01))
            .map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].active_loop = Some(LoopRegionStatus {
        in_secs,
        out_secs: out_secs.max(in_secs + 0.01),
        active: true,
    });
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_loop_out(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let (position_secs, _) = deck_playback_secs(&state, deck_id);
    let out_secs = snap_secs(
        position_secs.unwrap_or(0.0),
        state.decks[deck_id].bpm,
        state.decks[deck_id].quantize,
    );
    let in_secs = state.decks[deck_id]
        .active_loop
        .as_ref()
        .map(|region| region.in_secs)
        .unwrap_or(0.0);

    if out_secs <= in_secs {
        return Err("Loop out must be after loop in.".to_string());
    }

    with_engine(&mut state, |engine| {
        engine
            .set_deck_loop_region(deck_id, in_secs, out_secs)
            .map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].active_loop = Some(LoopRegionStatus {
        in_secs,
        out_secs,
        active: true,
    });
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn exit_deck_loop(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    with_engine(&mut state, |engine| {
        engine.clear_deck_loop(deck_id).map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].active_loop = None;
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn trigger_hot_cue(
    app: AppHandle,
    deck_id: usize,
    slot: u8,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let cue = state.decks[deck_id]
        .hot_cues
        .iter()
        .find(|cue| cue.slot == slot)
        .cloned()
        .ok_or_else(|| format!("Hot cue {} is empty.", slot + 1))?;

    let snapped = snap_secs(
        cue.position_secs,
        state.decks[deck_id].bpm,
        state.decks[deck_id].quantize,
    );

    with_engine(&mut state, |engine| {
        engine
            .seek_deck(deck_id, snapped)
            .map_err(|e| e.to_string())?;
        engine.play(deck_id).map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].playing = true;
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn save_hot_cue(
    app: AppHandle,
    deck_id: usize,
    slot: u8,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if slot > 7 {
        return Err("Hot cue slot must be 0..=7.".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let track_id = state.decks[deck_id]
        .track_id
        .clone()
        .ok_or_else(|| "Only library tracks can persist hot cues.".to_string())?;
    let (position_secs, _) = deck_playback_secs(&state, deck_id);
    let position = snap_secs(
        position_secs.unwrap_or(0.0),
        state.decks[deck_id].bpm,
        state.decks[deck_id].quantize,
    );

    state
        .library
        .save_track_hot_cue(&TrackId::new(track_id), slot, position, None, None, None)
        .map_err(|e| e.to_string())?;

    let track_id = state.decks[deck_id].track_id.clone();
    let (hot_cues, saved_loops) = fetch_deck_performance(&state.library, track_id.as_deref());
    apply_deck_performance(&mut state.decks[deck_id], hot_cues, saved_loops, false);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn delete_hot_cue(
    app: AppHandle,
    deck_id: usize,
    slot: u8,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if let Some(track_id) = state.decks[deck_id].track_id.clone() {
        state
            .library
            .delete_track_hot_cue(&TrackId::new(track_id), slot)
            .map_err(|e| e.to_string())?;
    }
    state.decks[deck_id].hot_cues.retain(|cue| cue.slot != slot);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn save_loop(
    app: AppHandle,
    deck_id: usize,
    slot: u8,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let track_id = state.decks[deck_id]
        .track_id
        .clone()
        .ok_or_else(|| "Only library tracks can persist loops.".to_string())?;
    let region = state.decks[deck_id]
        .active_loop
        .clone()
        .ok_or_else(|| "Set an active loop before saving.".to_string())?;

    state
        .library
        .save_track_loop(
            &TrackId::new(track_id),
            slot,
            region.in_secs,
            region.out_secs,
            None,
            None,
        )
        .map_err(|e| e.to_string())?;

    let track_id = state.decks[deck_id].track_id.clone();
    let (hot_cues, saved_loops) = fetch_deck_performance(&state.library, track_id.as_deref());
    apply_deck_performance(&mut state.decks[deck_id], hot_cues, saved_loops, false);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn recall_saved_loop(
    app: AppHandle,
    deck_id: usize,
    slot: u8,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.decks[deck_id].track.is_none() {
        return Err("Load a track before recalling a saved loop.".to_string());
    }

    let saved = state.decks[deck_id]
        .saved_loops
        .iter()
        .find(|loop_region| loop_region.slot == slot)
        .cloned()
        .ok_or_else(|| format!("Saved loop {} is empty.", slot + 1))?;

    with_engine(&mut state, |engine| {
        engine
            .set_deck_loop_region(deck_id, saved.in_secs, saved.out_secs)
            .map_err(|e| e.to_string())?;
        engine
            .seek_deck(deck_id, saved.in_secs)
            .map_err(|e| e.to_string())?;
        engine.play(deck_id).map_err(|e| e.to_string())
    })?;
    state.decks[deck_id].active_loop = Some(LoopRegionStatus {
        in_secs: saved.in_secs,
        out_secs: saved.out_secs,
        active: true,
    });
    state.decks[deck_id].playing = true;
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn delete_loop(
    app: AppHandle,
    deck_id: usize,
    slot: u8,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if let Some(track_id) = state.decks[deck_id].track_id.clone() {
        state
            .library
            .delete_track_loop(&TrackId::new(track_id), slot)
            .map_err(|e| e.to_string())?;
    }
    state.decks[deck_id]
        .saved_loops
        .retain(|row| row.slot != slot);
    Ok(publish_deck_transport(&app, &mut state, deck_id))
}
