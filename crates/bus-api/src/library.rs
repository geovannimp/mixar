//! Library records carried by library bus events.
//!
//! These are the shapes the library worker and the UI exchange; they live here
//! because the bus payloads carry them, not because they are wire types.

use serde::Deserialize;
use serde::Serialize;

/// Library track summary for the UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackSummary {
    pub id: String,
    pub display_name: String,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub duration_ms: Option<i32>,
    pub path: String,
    pub isrc: Option<String>,
    pub rating: Option<u8>,
    pub comment: Option<String>,
    pub year: Option<u16>,
    pub track_number: Option<u16>,
    pub disc_number: Option<u16>,
    pub color_label: Option<String>,
}

/// Persisted hot cue row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HotCue {
    pub slot: u8,
    pub position_ms: i32,
    pub loop_length_beats: Option<i32>,
    pub color: Option<String>,
    pub label: Option<String>,
}

/// Beat grid snapshot (matches the library's JSON storage).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeatGrid {
    pub beats: Vec<f32>,
    pub downbeats: Vec<f32>,
    pub bpm: f64,
}

/// Persisted saved-loop row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedLoop {
    pub slot: u8,
    pub in_ms: i32,
    pub out_ms: i32,
    pub label: Option<String>,
    pub color: Option<String>,
}
