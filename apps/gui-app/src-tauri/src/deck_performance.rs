//! Phase 2 deck performance commands (seek, cue, loops, hot cues).

use audio_core::{ms_to_secs, secs_to_ms};
use library::{HotCueRecord, LoopRecord};
use library_core::TrackId;
use serde::Serialize;
use tauri::AppHandle;

use crate::{deck_playback_ms, AppState, DeckInfo, DeckStatus, NUM_DECKS};

#[derive(Debug, Clone, Serialize)]
pub struct HotCueStatus {
    pub slot: u8,
    pub position_ms: i32,
    pub loop_length_beats: Option<i32>,
    pub color: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SavedLoopStatus {
    pub slot: u8,
    pub in_ms: i32,
    pub out_ms: i32,
    pub label: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoopRegionStatus {
    pub in_ms: i32,
    pub out_ms: i32,
    pub active: bool,
}

/// Beat-quantize media time without clamping (negative positions allowed).
pub fn snap_ms(ms: i32, bpm: Option<f64>, quantize: bool) -> i32 {
    if !quantize {
        return ms;
    }
    let Some(bpm) = bpm else {
        return ms;
    };
    if bpm <= 0.0 {
        return ms;
    }
    let secs = ms_to_secs(ms);
    let beat = 60.0 / bpm;
    secs_to_ms((secs / beat).round() * beat)
}

fn hot_cue_from_record(record: HotCueRecord) -> HotCueStatus {
    HotCueStatus {
        slot: record.slot_index,
        position_ms: record.position_ms,
        loop_length_beats: record.loop_length_beats,
        color: record.color,
        label: record.label,
    }
}

fn saved_loop_from_record(record: LoopRecord) -> SavedLoopStatus {
    SavedLoopStatus {
        slot: record.slot_index,
        in_ms: record.in_ms,
        out_ms: record.out_ms,
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
        deck.cue_point_ms = Some(0);
        deck.active_loop = None;
        deck.sync_mode = crate::SyncMode::Off;
        deck.speed = 1.0;
    }
    deck.hot_cues = hot_cues;
    deck.saved_loops = saved_loops;
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
    let (position_ms, _) = deck_playback_ms(state, deck_id);
    let position = snap_ms(
        position_ms.unwrap_or(0),
        state.decks[deck_id].bpm,
        state.decks[deck_id].quantize,
    );

    state
        .library
        .lock()
        .unwrap()
        .save_track_hot_cue(&TrackId::new(track_id), slot, position, None, None, None)
        .map_err(|e| e.to_string())?;

    let track_id = state.decks[deck_id].track_id.clone();
    let (hot_cues, saved_loops) = {
        let library = state.library.lock().unwrap();
        fetch_deck_performance(&library, track_id.as_deref())
    };
    apply_deck_performance(&mut state.decks[deck_id], hot_cues, saved_loops, false);
    Ok(crate::engine_controller::publish_deck(app, state, deck_id))
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
            .lock()
            .unwrap()
            .delete_track_hot_cue(&TrackId::new(track_id), slot)
            .map_err(|e| e.to_string())?;
    }
    state.decks[deck_id].hot_cues.retain(|cue| cue.slot != slot);
    Ok(crate::engine_controller::publish_deck(app, state, deck_id))
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
        .lock()
        .unwrap()
        .save_track_loop(
            &TrackId::new(track_id),
            slot,
            region.in_ms,
            region.out_ms,
            None,
            None,
        )
        .map_err(|e| e.to_string())?;

    let track_id = state.decks[deck_id].track_id.clone();
    let (hot_cues, saved_loops) = {
        let library = state.library.lock().unwrap();
        fetch_deck_performance(&library, track_id.as_deref())
    };
    apply_deck_performance(&mut state.decks[deck_id], hot_cues, saved_loops, false);
    Ok(crate::engine_controller::publish_deck(app, state, deck_id))
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
            .lock()
            .unwrap()
            .delete_track_loop(&TrackId::new(track_id), slot)
            .map_err(|e| e.to_string())?;
    }
    state.decks[deck_id]
        .saved_loops
        .retain(|row| row.slot != slot);
    Ok(crate::engine_controller::publish_deck(app, state, deck_id))
}
