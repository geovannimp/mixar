//! Phase 3 deck commands: sync, beat jump, pad modes, filter, gain trim.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::deck_performance::{snap_secs, LoopRegionStatus};
use crate::engine_controller::publish_deck;
use crate::{
    clamp_eq_db, deck_playback_secs, with_engine, AppState, DeckStatus, SharedAppState, NUM_DECKS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    Off,
    Tempo,
    Beat,
}

impl Default for SyncMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadMode {
    HotCue,
    LoopRoll,
    BeatJump,
}

impl Default for PadMode {
    fn default() -> Self {
        Self::HotCue
    }
}

impl PadMode {
    pub fn next(self) -> Self {
        match self {
            PadMode::HotCue => PadMode::LoopRoll,
            PadMode::LoopRoll => PadMode::BeatJump,
            PadMode::BeatJump => PadMode::HotCue,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            PadMode::HotCue => PadMode::BeatJump,
            PadMode::LoopRoll => PadMode::HotCue,
            PadMode::BeatJump => PadMode::LoopRoll,
        }
    }
}

pub(crate) fn apply_tempo_sync_for_state(
    state: &mut AppState,
    slave_id: usize,
    master_id: usize,
) -> Result<(), String> {
    if slave_id == master_id {
        return Err("Cannot sync a deck to itself.".to_string());
    }

    let master = &state.decks[master_id];
    let slave_bpm = state.decks[slave_id]
        .bpm
        .ok_or_else(|| "Slave deck BPM is required for sync.".to_string())?;
    let master_bpm = master
        .bpm
        .ok_or_else(|| "Master deck BPM is required for sync.".to_string())?;

    let master_effective = master_bpm * f64::from(master.speed);
    let target_speed = (master_effective / slave_bpm) as f32;
    let target_speed = target_speed.clamp(0.5, 2.0);

    state.decks[slave_id].speed = target_speed;
    with_engine(state, |engine| {
        engine
            .set_deck_speed(slave_id, target_speed)
            .map_err(|e| e.to_string())
    })
}

pub(crate) fn align_beat_phase_for_state(
    state: &mut AppState,
    slave_id: usize,
    master_id: usize,
) -> Result<(), String> {
    let master_bpm = state.decks[master_id]
        .bpm
        .ok_or_else(|| "Master deck BPM is required for beat sync.".to_string())?;
    let slave_bpm = state.decks[slave_id]
        .bpm
        .ok_or_else(|| "Slave deck BPM is required for beat sync.".to_string())?;

    let (master_pos, _) = deck_playback_secs(state, master_id);
    let (slave_pos, duration) = deck_playback_secs(state, slave_id);
    let master_pos = master_pos.unwrap_or(0.0);
    let slave_pos = slave_pos.unwrap_or(0.0);
    let duration = duration.unwrap_or(0.0);

    let master_beat = 60.0 / master_bpm;
    let slave_beat = 60.0 / slave_bpm;
    let master_phase = master_pos % master_beat;

    let slave_beat_index = (slave_pos / slave_beat).floor();
    let mut target = slave_beat_index * slave_beat + master_phase;
    if target + slave_beat * 0.5 < slave_pos {
        target += slave_beat;
    }

    let snapped = snap_secs(
        target.min(duration),
        Some(slave_bpm),
        state.decks[slave_id].quantize,
    );

    with_engine(state, |engine| {
        engine
            .seek_deck(slave_id, snapped)
            .map_err(|e| e.to_string())
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatusLite {
    pub master_deck: usize,
    pub decks: Vec<DeckStatus>,
}

fn publish_mix_state(app: &AppHandle, state: &mut AppState) -> EngineStatusLite {
    use crate::engine_controller::{bump_revision, deck_status};
    use crate::engine_events::emit_deck_updated;

    let revision = bump_revision(state);
    let decks: Vec<DeckStatus> = state
        .decks
        .iter()
        .enumerate()
        .map(|(id, deck)| deck_status(state, id, deck))
        .collect();

    for deck in &decks {
        emit_deck_updated(app, revision, deck.clone());
    }

    EngineStatusLite {
        master_deck: state.master_deck,
        decks,
    }
}

#[tauri::command]
pub fn set_master_deck(
    app: AppHandle,
    deck_id: usize,
    state: State<'_, SharedAppState>,
) -> Result<EngineStatusLite, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.master_deck = deck_id;

    for slave_id in 0..NUM_DECKS {
        if slave_id == deck_id {
            continue;
        }
        if state.decks[slave_id].sync_mode != SyncMode::Off {
            let mode = state.decks[slave_id].sync_mode;
            apply_tempo_sync_for_state(&mut state, slave_id, deck_id)?;
            if mode == SyncMode::Beat {
                align_beat_phase_for_state(&mut state, slave_id, deck_id)?;
            }
        }
    }

    Ok(publish_mix_state(&app, &mut state))
}

#[tauri::command]
pub fn toggle_deck_sync(
    app: AppHandle,
    deck_id: usize,
    beat_sync: Option<bool>,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    if state.decks[deck_id].track.is_none() {
        return Err("Load a track before enabling sync.".to_string());
    }

    let master_id = state.master_deck;
    if deck_id == master_id {
        return Err("Master deck cannot sync to itself. Choose the other deck.".to_string());
    }

    let next_mode = if state.decks[deck_id].sync_mode == SyncMode::Off {
        if beat_sync.unwrap_or(false) {
            SyncMode::Beat
        } else {
            SyncMode::Tempo
        }
    } else {
        SyncMode::Off
    };

    state.decks[deck_id].sync_mode = next_mode;

    if next_mode != SyncMode::Off {
        apply_tempo_sync_for_state(&mut state, deck_id, master_id)?;
        if next_mode == SyncMode::Beat {
            align_beat_phase_for_state(&mut state, deck_id, master_id)?;
        }
    }

    Ok(publish_deck(&app, &mut state, deck_id))
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
        engine
            .seek_deck(deck_id, target)
            .map_err(|e| e.to_string())
    })?;

    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn cycle_deck_pad_mode(
    app: AppHandle,
    deck_id: usize,
    direction: i32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.decks[deck_id].pad_mode = if direction < 0 {
        state.decks[deck_id].pad_mode.prev()
    } else {
        state.decks[deck_id].pad_mode.next()
    };
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_filter(
    app: AppHandle,
    deck_id: usize,
    filter_db: f32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let clamped = clamp_eq_db(filter_db);
    state.decks[deck_id].filter_db = clamped;
    if let Some(engine) = state.engine.as_mut() {
        engine
            .set_deck_filter_db(deck_id, clamped)
            .map_err(|e| e.to_string())?;
    }
    Ok(publish_deck(&app, &mut state, deck_id))
}

#[tauri::command]
pub fn set_deck_gain_trim(
    app: AppHandle,
    deck_id: usize,
    gain_db: f32,
    state: State<'_, SharedAppState>,
) -> Result<DeckStatus, String> {
    if deck_id >= NUM_DECKS {
        return Err(format!("Invalid deck ID: {deck_id}"));
    }

    let mut state = state.lock().map_err(|e| e.to_string())?;
    let clamped = clamp_eq_db(gain_db);
    state.decks[deck_id].gain_trim_db = clamped;
    if let Some(engine) = state.engine.as_mut() {
        engine
            .set_deck_gain_trim_db(deck_id, clamped)
            .map_err(|e| e.to_string())?;
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
            engine
                .clear_deck_loop(deck_id)
                .map_err(|e| e.to_string())
        })?;
        state.decks[deck_id].active_loop = None;
    }

    Ok(publish_deck(&app, &mut state, deck_id))
}
