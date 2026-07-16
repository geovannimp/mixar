//! Engine → UI event bus (see docs/deck-spec.md §9).

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{DeckStatus, EngineStatus};

pub const ENGINE_EVENT: &str = "engine://event";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    Status {
        revision: u64,
        status: EngineStatus,
    },
    DeckUpdated {
        revision: u64,
        deck: DeckStatus,
    },
    Position {
        deck_id: usize,
        position_secs: f64,
    },
    Levels {
        deck_id: usize,
        peak_l: f32,
        peak_r: f32,
        peak_hold_l: f32,
        peak_hold_r: f32,
    },
    #[allow(dead_code)]
    Notice {
        message: String,
    },
    #[allow(dead_code)]
    Error {
        message: String,
    },
}

pub fn emit_event(app: &AppHandle, event: EngineEvent) {
    if let Err(err) = app.emit(ENGINE_EVENT, &event) {
        log::warn!("failed to emit {ENGINE_EVENT}: {err}");
    }
}

pub fn emit_status(app: &AppHandle, revision: u64, status: EngineStatus) {
    emit_event(
        app,
        EngineEvent::Status {
            revision,
            status,
        },
    );
}

pub fn emit_deck_updated(app: &AppHandle, revision: u64, deck: DeckStatus) {
    emit_event(
        app,
        EngineEvent::DeckUpdated {
            revision,
            deck,
        },
    );
}

pub fn emit_position(app: &AppHandle, deck_id: usize, position_secs: f64) {
    emit_event(
        app,
        EngineEvent::Position {
            deck_id,
            position_secs,
        },
    );
}

pub fn emit_levels(
    app: &AppHandle,
    deck_id: usize,
    peak_l: f32,
    peak_r: f32,
    peak_hold_l: f32,
    peak_hold_r: f32,
) {
    emit_event(
        app,
        EngineEvent::Levels {
            deck_id,
            peak_l,
            peak_r,
            peak_hold_l,
            peak_hold_r,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeckEq, EngineStatus, SyncMode};

    #[test]
    fn engine_event_status_serializes_with_type_tag() {
        let event = EngineEvent::Status {
            revision: 1,
            status: EngineStatus {
                running: true,
                backend: "cpal".to_string(),
                sample_rate: 48_000,
                crossfader: 0.5,
                cue_mix: 0.0,
                master_cue: false,
                decks: vec![DeckStatus {
                    id: 0,
                    track: None,
                    track_id: None,
                    title: None,
                    artist: None,
                    bpm: None,
                    key: None,
                    playing: false,
                    volume: 1.0,
                    speed: 1.0,
                    eq: DeckEq::default(),
                    position_secs: None,
                    duration_secs: None,
                    cue_point_secs: None,
                    quantize: true,
                    hot_cues: vec![],
                    saved_loops: vec![],
                    active_loop: None,
                    filter_db: 0.0,
                    gain_trim_db: 0.0,
                    loudness_lufs: None,
                    auto_gain_db: 0.0,
                    sync_mode: SyncMode::Off,
                    is_master: true,
                    pad_mode: crate::deck_sync::PadMode::HotCue,
                    headphone_cue: false,
                }],
            },
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("status"));
        assert_eq!(json.get("revision").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn engine_event_levels_serializes_with_type_tag() {
        let event = EngineEvent::Levels {
            deck_id: 0,
            peak_l: 0.5,
            peak_r: 0.6,
            peak_hold_l: 0.7,
            peak_hold_r: 0.8,
        };
        let json = serde_json::to_value(&event).expect("serialize");
        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("levels"));
        assert_eq!(json.get("deck_id").and_then(|v| v.as_u64()), Some(0));
    }
}
