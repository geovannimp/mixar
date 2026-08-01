//! Host status shapes still mirrored on `DeckInfo` (UI reads cues via library `useTrack`).

use serde::Serialize;

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
