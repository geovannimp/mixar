//! Phase 3 deck commands: beat jump, pad modes, loop roll.
//! Tempo/beat sync + master deck live on the engine cmd bus.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::deck_performance::{snap_secs, LoopRegionStatus};
use crate::deck_sampler::ensure_deck_bank_loaded;
use crate::engine_controller::{publish_deck, publish_status};
use crate::{deck_playback_secs, with_engine, DeckStatus, SharedAppState, NUM_DECKS};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    #[default]
    Off,
    Tempo,
    Beat,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadMode {
    #[default]
    HotCue,
    LoopRoll,
    BeatJump,
    Sampler,
}

#[tauri::command]
pub fn beat_jump_deck(
    app: AppHandle,
    deck_id: usize,
    beats: i32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if beats == 0 {
        return Err("Beat jump requires a non-zero beat count.".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let bpm = state.decks[deck_id]
        .bpm
        .ok_or_else(|| "Track BPM is required for beat jump.".to_string())?;
    let (position_secs, duration_secs) = deck_playback_secs(&state, deck_id);
    let position = position_secs.unwrap_or(0.0);
    let duration = duration_secs.unwrap_or(0.0);
    let beat_len = 60.0 / bpm;
    let raw = (position + beat_len * f64::from(beats)).clamp(0.0, duration);
    let target = snap_secs(raw, Some(bpm), state.decks[deck_id].quantize);

    with_engine(&mut state, |engine| {
        engine.seek_deck(deck_id, target).map_err(|e| e.to_string())
    })?;

    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_pad_mode(
    app: AppHandle,
    deck_id: usize,
    mode: PadMode,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.decks[deck_id].pad_mode = mode;
    if mode == PadMode::Sampler {
        let _ = ensure_deck_bank_loaded(&mut state, deck_id);
        publish_status(&app, &mut state);
    }
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn begin_loop_roll(
    app: AppHandle,
    deck_id: usize,
    beats: u32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }
    if beats == 0 {
        return Err("Loop roll requires at least 1 beat.".to_string());
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let bpm = state.decks[deck_id]
        .bpm
        .ok_or_else(|| "Track BPM is required for loop roll.".to_string())?;
    let (position_secs, duration_secs) = deck_playback_secs(&state, deck_id);
    let position = position_secs.unwrap_or(0.0);
    let duration = duration_secs.unwrap_or(0.0);
    let beat_len = 60.0 / bpm;
    let in_secs = snap_secs(position, Some(bpm), state.decks[deck_id].quantize);
    let out_secs = (in_secs + beat_len * beats as f64).min(duration.max(in_secs + beat_len));

    state.decks[deck_id].loop_roll_restore = state.decks[deck_id].active_loop.clone();

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
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn end_loop_roll(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let restore = state.decks[deck_id].loop_roll_restore.take();

    if let Some(region) = restore.filter(|region| region.active) {
        with_engine(&mut state, |engine| {
            engine
                .set_deck_loop_region(deck_id, region.in_secs, region.out_secs)
                .map_err(|e| e.to_string())
        })?;
        state.decks[deck_id].active_loop = Some(region);
    } else {
        with_engine(&mut state, |engine| {
            engine.clear_deck_loop(deck_id).map_err(|e| e.to_string())
        })?;
        state.decks[deck_id].active_loop = None;
    }

    Ok(publish_deck(&app, &mut state, deck_id))
}
