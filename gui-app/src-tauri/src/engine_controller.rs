//! Central publish/apply path for engine → UI state (see docs/deck-spec.md §9).

use tauri::AppHandle;

use crate::engine_events::{emit_deck_updated, emit_status};
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
        decks: deck_statuses(state),
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
