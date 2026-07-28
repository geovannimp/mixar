//! Central publish/apply path for engine → UI state (see docs/deck-spec.md §9).

use engine_api::{
    encode_evt_body, encode_wire, DeckEq as ApiDeckEq, DeckHotCue as ApiDeckHotCue,
    DeckSavedLoop as ApiDeckSavedLoop, DeckSnapshot as ApiDeckSnapshot,
    EngineStatus as ApiEngineStatus, EvtBody, Kind, LoopRegion as ApiLoopRegion, Origin,
    PadMode as ApiPadMode, SamplerBankInfo as ApiSamplerBankInfo, SamplerPlayMode as ApiSamplerPlayMode,
    SamplerSlotInfo as ApiSamplerSlotInfo, SamplerStatus as ApiSamplerStatus, WireMessage,
};
use tauri::{AppHandle, Emitter};

use crate::deck_sampler::SamplerStatus;
use crate::deck_sync::SyncMode;
use crate::bus_bridge::ENGINE_BUS_EVENT;
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
        master_deck: Some(state.master_deck),
        decks: deck_statuses(state),
        sampler: SamplerStatus::from_state(state),
    }
}

fn to_api_pad_mode(mode: crate::deck_sync::PadMode) -> ApiPadMode {
    match mode {
        crate::deck_sync::PadMode::HotCue => ApiPadMode::HotCue,
        crate::deck_sync::PadMode::LoopRoll => ApiPadMode::LoopRoll,
        crate::deck_sync::PadMode::BeatJump => ApiPadMode::BeatJump,
        crate::deck_sync::PadMode::Sampler => ApiPadMode::Sampler,
    }
}

fn to_api_sync_mode(mode: crate::deck_sync::SyncMode) -> engine_api::SyncMode {
    match mode {
        crate::deck_sync::SyncMode::Off => engine_api::SyncMode::Off,
        crate::deck_sync::SyncMode::Tempo => engine_api::SyncMode::Tempo,
        crate::deck_sync::SyncMode::Beat => engine_api::SyncMode::Beat,
    }
}

fn to_api_play_mode(mode: crate::deck_sampler::SamplerPlayModeSetting) -> ApiSamplerPlayMode {
    match mode {
        crate::deck_sampler::SamplerPlayModeSetting::Oneshot => ApiSamplerPlayMode::Oneshot,
        crate::deck_sampler::SamplerPlayModeSetting::Hold => ApiSamplerPlayMode::Hold,
        crate::deck_sampler::SamplerPlayModeSetting::Loop => ApiSamplerPlayMode::Loop,
    }
}

fn to_api_deck_snapshot(deck: DeckStatus) -> ApiDeckSnapshot {
    ApiDeckSnapshot {
        id: deck.id as u16,
        track: deck.track,
        track_id: deck.track_id,
        title: deck.title,
        artist: deck.artist,
        bpm: deck.bpm,
        key: deck.key,
        playing: deck.playing,
        volume: deck.volume,
        speed: deck.speed,
        eq: ApiDeckEq {
            low: deck.eq.low,
            mid: deck.eq.mid,
            high: deck.eq.high,
        },
        filter_db: deck.filter_db,
        gain_trim_db: deck.gain_trim_db,
        headphone_cue: deck.headphone_cue,
        sync_mode: to_api_sync_mode(deck.sync_mode),
        cue_point_secs: deck.cue_point_secs,
        quantize: deck.quantize,
        active_loop: deck.active_loop.map(|region| ApiLoopRegion {
            in_secs: region.in_secs,
            out_secs: region.out_secs,
            active: region.active,
        }),
        pad_mode: to_api_pad_mode(deck.pad_mode),
        position_secs: deck.position_secs,
        duration_secs: deck.duration_secs,
        hot_cues: deck
            .hot_cues
            .into_iter()
            .map(|cue| ApiDeckHotCue {
                slot: cue.slot,
                position_secs: cue.position_secs,
                loop_length_beats: cue.loop_length_beats,
                color: cue.color,
                label: cue.label,
            })
            .collect(),
        saved_loops: deck
            .saved_loops
            .into_iter()
            .map(|saved| ApiDeckSavedLoop {
                slot: saved.slot,
                in_secs: saved.in_secs,
                out_secs: saved.out_secs,
                label: saved.label,
                color: saved.color,
            })
            .collect(),
        loudness_lufs: deck.loudness_lufs,
        auto_gain_db: deck.auto_gain_db,
        active_sampler_bank_id: deck.active_sampler_bank_id,
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
                slots.into_iter()
                    .map(|slot| ApiSamplerSlotInfo {
                        label: slot.label,
                        track_id: slot.track_id,
                        path: slot.path,
                        duration_secs: slot.duration_secs,
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

fn to_api_engine_status(status: EngineStatus) -> ApiEngineStatus {
    ApiEngineStatus {
        running: status.running,
        sample_rate: status.sample_rate,
        crossfader: status.crossfader,
        cue_mix: status.cue_mix,
        master_cue: status.master_cue,
        master_deck: status.master_deck.unwrap_or_default() as u16,
        decks: status.decks.into_iter().map(to_api_deck_snapshot).collect(),
        sampler: to_api_sampler_status(status.sampler),
    }
}

fn encode_bus_event(origin: Origin, kind: Kind, revision: u64, body: EvtBody) -> Result<Vec<u8>, String> {
    let body = encode_evt_body(&body).map_err(|e| e.to_string())?;
    encode_wire(&WireMessage {
        origin,
        kind,
        revision,
        action_timestamp_ms: 0,
        body,
    })
    .map_err(|e| e.to_string())
}

pub fn emit_bus_payload(app: &AppHandle, payload: Vec<u8>) {
    if let Err(err) = app.emit(ENGINE_BUS_EVENT, payload) {
        log::warn!("failed to emit {ENGINE_BUS_EVENT}: {err}");
    }
}

pub fn prepare_deck_event(state: &mut AppState, deck_id: usize) -> Result<(DeckStatus, Vec<u8>), String> {
    sync_app_state_from_engine(state);
    let revision = bump_revision(state);
    let deck = deck_status(state, deck_id, &state.decks[deck_id]);
    let snapshot = to_api_deck_snapshot(deck.clone());
    let payload = encode_bus_event(
        Origin::Deck(deck_id as u16),
        Kind::Updated,
        revision,
        EvtBody::DeckUpdated {
            id: snapshot.id,
            track: snapshot.track,
            track_id: snapshot.track_id,
            title: snapshot.title,
            artist: snapshot.artist,
            bpm: snapshot.bpm,
            key: snapshot.key,
            playing: snapshot.playing,
            volume: snapshot.volume,
            speed: snapshot.speed,
            eq: snapshot.eq,
            filter_db: snapshot.filter_db,
            gain_trim_db: snapshot.gain_trim_db,
            headphone_cue: snapshot.headphone_cue,
            sync_mode: snapshot.sync_mode,
            cue_point_secs: snapshot.cue_point_secs,
            quantize: snapshot.quantize,
            active_loop: snapshot.active_loop,
            pad_mode: snapshot.pad_mode,
            position_secs: snapshot.position_secs,
            duration_secs: snapshot.duration_secs,
            hot_cues: snapshot.hot_cues,
            saved_loops: snapshot.saved_loops,
            loudness_lufs: snapshot.loudness_lufs,
            auto_gain_db: snapshot.auto_gain_db,
            active_sampler_bank_id: snapshot.active_sampler_bank_id,
        },
    )?;
    Ok((deck, payload))
}

pub fn publish_status(app: &AppHandle, state: &mut AppState) -> EngineStatus {
    sync_app_state_from_engine(state);
    let revision = bump_revision(state);
    let status = engine_status(state);
    if let Ok(payload) = encode_bus_event(
        Origin::Mixer,
        Kind::Status,
        revision,
        EvtBody::EngineStatus {
            status: to_api_engine_status(status.clone()),
        },
    ) {
        emit_bus_payload(app, payload);
    }
    status
}

pub fn publish_deck(app: &AppHandle, state: &mut AppState, deck_id: usize) -> DeckStatus {
    match prepare_deck_event(state, deck_id) {
        Ok((deck, payload)) => {
            emit_bus_payload(app, payload);
            deck
        }
        Err(err) => {
            log::warn!("failed to prepare deck bus event: {err}");
            deck_status(state, deck_id, &state.decks[deck_id])
        }
    }
}
