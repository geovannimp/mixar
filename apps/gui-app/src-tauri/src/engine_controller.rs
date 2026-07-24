//! Central publish/apply path for engine → UI state (see docs/deck-spec.md §9).

use tauri::AppHandle;

use crate::engine_events::{emit_deck_updated, emit_status};
use crate::deck_sampler::SamplerStatus;
use crate::{deck_playback_secs, AppState, DeckInfo, DeckStatus, EngineStatus};

pub fn bump_revision(state: &mut AppState) -> u64 {
    state.revision += 1;
    state.revision
}

pub fn deck_status(state: &AppState, id: usize, deck: &DeckInfo) -> DeckStatus {
    let (position_secs, duration_secs) = deck_playback_secs(state, id);
    DeckStatus {
        id,
        track: deck.track.clone(),
        track_id: deck.track_id.clone(),
        title: deck.title.clone(),
        artist: deck.artist.clone(),
        bpm: deck.bpm,
        key: deck.key.clone(),
        playing: deck.playing,
        volume: deck.volume,
        speed: deck.speed,
        eq: deck.eq.clone(),
        position_secs,
        duration_secs,
        cue_point_secs: deck.cue_point_secs,
        quantize: deck.quantize,
        hot_cues: deck.hot_cues.clone(),
        saved_loops: deck.saved_loops.clone(),
        active_loop: deck.active_loop.clone(),
        filter_db: deck.filter_db,
        gain_trim_db: deck.gain_trim_db,
        loudness_lufs: deck.loudness_lufs,
        auto_gain_db: deck.auto_gain_db,
        sync_mode: deck.sync_mode,
        is_master: id == state.master_deck,
        pad_mode: deck.pad_mode,
        headphone_cue: deck.headphone_cue,
        active_sampler_bank_id: deck.active_sampler_bank_id.clone(),
    }
}

fn deck_statuses(state: &AppState) -> Vec<DeckStatus> {
    state
        .decks
        .iter()
        .enumerate()
        .map(|(id, deck)| deck_status(state, id, deck))
        .collect()
}

pub fn engine_status(state: &AppState) -> EngineStatus {
    EngineStatus {
        running: state.engine.is_some(),
        backend: "cpal".to_string(),
        sample_rate: 48_000,
        crossfader: state.crossfader,
        cue_mix: state.cue_mix,
        master_cue: state.master_cue,
        decks: deck_statuses(state),
        sampler: SamplerStatus::from_state(state),
    }
}

pub fn publish_status(app: &AppHandle, state: &mut AppState) -> EngineStatus {
    let revision = bump_revision(state);
    let status = engine_status(state);
    emit_status(app, revision, status.clone());
    status
}

pub fn publish_deck(app: &AppHandle, state: &mut AppState, deck_id: usize) -> DeckStatus {
    let revision = bump_revision(state);
    let deck = deck_status(state, deck_id, &state.decks[deck_id]);
    emit_deck_updated(app, revision, deck.clone());
    deck
}
