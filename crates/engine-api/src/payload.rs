use serde::{Deserialize, Serialize};

/// Three-band deck EQ gains.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckEq {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

/// Slim deck snapshot for status patches and full snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckSnapshot {
    pub id: u16,
    pub playing: bool,
    pub volume: f32,
    pub speed: f32,
    pub eq: DeckEq,
    pub position_secs: Option<f64>,
    pub duration_secs: Option<f64>,
}

/// Full engine snapshot for hydrate and multi-deck changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineStatus {
    pub running: bool,
    pub sample_rate: u32,
    pub crossfader: f32,
    pub cue_mix: f32,
    pub master_cue: bool,
    pub decks: Vec<DeckSnapshot>,
}

/// Command bus payload nested inside [`crate::WireMessage::body`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CmdBody {
    Empty,
    Seek { position_secs: f64 },
    SetVolume { volume: f32 },
    SetEq { low: f32, mid: f32, high: f32 },
    SetSpeed { speed: f32 },
    SetCrossfader { position: f32 },
    SetCueMix { mix: f32 },
    SetMasterCue { enabled: bool },
}

/// Event bus payload nested inside [`crate::WireMessage::body`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EvtBody {
    Empty,
    DeckUpdated {
        id: u16,
        playing: bool,
        volume: f32,
        speed: f32,
        eq: DeckEq,
        position_secs: Option<f64>,
        duration_secs: Option<f64>,
    },
    Position {
        position_secs: f64,
    },
    Levels {
        peak_l: f32,
        peak_r: f32,
        peak_hold_l: f32,
        peak_hold_r: f32,
    },
    EngineStatus(EngineStatus),
    Error {
        message: String,
    },
    Notice {
        message: String,
    },
}
