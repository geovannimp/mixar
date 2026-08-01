//! Deck performance helpers (library hydrate for hot cues / saved loops).

use library::{HotCueRecord, LoopRecord};
use library_api::{EvtBody, HotCue, Kind, Origin, SavedLoop};
use library_core::TrackId;
use serde::Serialize;

use crate::{AppState, DeckInfo};

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
) {
    deck.hot_cues = hot_cues;
    deck.saved_loops = saved_loops;
}

/// Publish current cue/loop rows on the library evt bus so FE can hydrate per-track.
pub fn publish_performance_hydrate(
    state: &AppState,
    track_id: &str,
    hot_cues: &[HotCueStatus],
    saved_loops: &[SavedLoopStatus],
) {
    let origin = Origin::Track(track_id.to_string());
    let _ = state.library_session.publish_evt(
        origin.clone(),
        Kind::HotCuesChanged,
        EvtBody::HotCuesChanged {
            track_id: track_id.to_string(),
            hot_cues: hot_cues
                .iter()
                .map(|cue| HotCue {
                    slot: cue.slot,
                    position_ms: cue.position_ms,
                    loop_length_beats: cue.loop_length_beats,
                    color: cue.color.clone(),
                    label: cue.label.clone(),
                })
                .collect(),
        },
    );
    let _ = state.library_session.publish_evt(
        origin,
        Kind::LoopsChanged,
        EvtBody::LoopsChanged {
            track_id: track_id.to_string(),
            loops: saved_loops
                .iter()
                .map(|row| SavedLoop {
                    slot: row.slot,
                    in_ms: row.in_ms,
                    out_ms: row.out_ms,
                    label: row.label.clone(),
                    color: row.color.clone(),
                })
                .collect(),
        },
    );
}
