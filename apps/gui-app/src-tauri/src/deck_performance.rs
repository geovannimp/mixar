//! Phase 2 deck performance commands (seek, cue, loops, hot cues).

use library::{HotCueRecord, LoopRecord};
use library_core::TrackId;
use serde::Serialize;
use tauri::AppHandle;

use crate::{
    bump_revision, deck_playback_secs, deck_status, AppState, DeckInfo, DeckStatus, NUM_DECKS,
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

pub(crate) fn save_hot_cue_inner(
    app: &AppHandle,
    state: &mut AppState,
    deck_id: usize,
    slot: u8,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if slot > 7 {
        return Err("Hot cue slot must be 0..=7.".to_string());
    }

    let track_id = state.decks[deck_id]
        .track_id
        .clone()
        .ok_or_else(|| "Only library tracks can persist hot cues.".to_string())?;
    let (position_secs, _) = deck_playback_secs(state, deck_id);
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
    Ok(publish_deck_transport(app, state, deck_id))
}

pub(crate) fn delete_hot_cue_inner(
    app: &AppHandle,
    state: &mut AppState,
    deck_id: usize,
    slot: u8,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    if let Some(track_id) = state.decks[deck_id].track_id.clone() {
        state
            .library
            .delete_track_hot_cue(&TrackId::new(track_id), slot)
            .map_err(|e| e.to_string())?;
    }
    state.decks[deck_id].hot_cues.retain(|cue| cue.slot != slot);
    Ok(publish_deck_transport(app, state, deck_id))
}

pub(crate) fn save_loop_inner(
    app: &AppHandle,
    state: &mut AppState,
    deck_id: usize,
    slot: u8,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

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
    Ok(publish_deck_transport(app, state, deck_id))
}

pub(crate) fn delete_loop_inner(
    app: &AppHandle,
    state: &mut AppState,
    deck_id: usize,
    slot: u8,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    if let Some(track_id) = state.decks[deck_id].track_id.clone() {
        state
            .library
            .delete_track_loop(&TrackId::new(track_id), slot)
            .map_err(|e| e.to_string())?;
    }
    state.decks[deck_id]
        .saved_loops
        .retain(|row| row.slot != slot);
    Ok(publish_deck_transport(app, state, deck_id))
}
