//! Central publish/apply path for engine → UI state (see docs/deck-spec.md §9).

use tauri::AppHandle;

use crate::deck_sampler::SamplerStatus;
use crate::deck_sync::SyncMode;
use crate::engine_events::{emit_deck_updated, emit_status};
use crate::{deck_playback_secs, AppState, DeckInfo, DeckStatus, EngineStatus, NUM_DECKS};

/// Overlay engine-owned transport/mix fields onto AppState before publish.
fn sync_app_state_from_engine(state: &mut AppState) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    let Ok(Some(engine_status)) = session.with_engine(|eng| Ok(eng.engine_status_snapshot())) else {
        return;
    };

    state.crossfader = engine_status.crossfader;
    state.cue_mix = engine_status.cue_mix;
    state.master_cue = engine_status.master_cue;
    state.master_deck = engine_status.master_deck as usize;

    for snap in engine_status.decks {
        let id = snap.id as usize;
        if id >= NUM_DECKS {
            continue;
        }
        let deck = &mut state.decks[id];
        deck.playing = snap.playing;
        deck.volume = snap.volume;
        deck.speed = snap.speed;
        deck.eq.low = snap.eq.low;
        deck.eq.mid = snap.eq.mid;
        deck.eq.high = snap.eq.high;
        deck.filter_db = snap.filter_db;
        deck.gain_trim_db = snap.gain_trim_db;
        deck.headphone_cue = snap.headphone_cue;
        deck.sync_mode = match snap.sync_mode {
            engine_api::SyncMode::Off => SyncMode::Off,
            engine_api::SyncMode::Tempo => SyncMode::Tempo,
            engine_api::SyncMode::Beat => SyncMode::Beat,
        };
        deck.cue_point_secs = snap.cue_point_secs;
        deck.quantize = snap.quantize;
        deck.active_loop = snap.active_loop.map(|region| crate::deck_performance::LoopRegionStatus {
            in_secs: region.in_secs,
            out_secs: region.out_secs,
            active: region.active,
        });
    }
}

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
        running: state.session.is_some(),
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
    sync_app_state_from_engine(state);
    let revision = bump_revision(state);
    let status = engine_status(state);
    emit_status(app, revision, status.clone());
    status
}

pub fn publish_deck(app: &AppHandle, state: &mut AppState, deck_id: usize) -> DeckStatus {
    sync_app_state_from_engine(state);
    let revision = bump_revision(state);
    let deck = deck_status(state, deck_id, &state.decks[deck_id]);
    emit_deck_updated(app, revision, deck.clone());
    deck
}
