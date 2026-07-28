use serde::{Deserialize, Serialize};

/// Three-band deck EQ gains.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckEq {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

/// Deck sync follow mode (slave → master).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    #[default]
    Off,
    Tempo,
    Beat,
}

/// Active loop region on a deck.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoopRegion {
    pub in_secs: f64,
    pub out_secs: f64,
    pub active: bool,
}

/// Controller pad mode for a deck.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadMode {
    #[default]
    HotCue,
    LoopRoll,
    BeatJump,
    Sampler,
}

/// Slim deck snapshot for status patches and full snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckSnapshot {
    pub id: u16,
    pub playing: bool,
    pub volume: f32,
    pub speed: f32,
    pub eq: DeckEq,
    pub filter_db: f32,
    pub gain_trim_db: f32,
    pub headphone_cue: bool,
    pub sync_mode: SyncMode,
    pub cue_point_secs: Option<f64>,
    pub quantize: bool,
    pub active_loop: Option<LoopRegion>,
    pub pad_mode: PadMode,
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
    pub master_deck: u16,
    pub decks: Vec<DeckSnapshot>,
}

/// Command bus payload nested inside [`crate::WireMessage::body`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CmdBody {
    Empty,
    Seek { position_secs: f64 },
    SetVolume { volume: f32 },
    SetEq { low: f32, mid: f32, high: f32 },
    SetSpeed { speed: f32 },
    SetFilter { filter_db: f32 },
    SetGainTrim { gain_db: f32 },
    SetHeadphoneCue { enabled: bool },
    SetCrossfader { position: f32 },
    SetCueMix { mix: f32 },
    SetMasterCue { enabled: bool },
    ToggleSync { beat_sync: bool },
    SetQuantize { enabled: bool },
    SetAutoLoop { beats: u32 },
    BeatJump { beats: i32 },
    SetPadMode { mode: PadMode },
    BeginLoopRoll { beats: u32 },
    TriggerHotCue { position_secs: f64 },
    RecallSavedLoop { in_secs: f64, out_secs: f64 },
}

/// Event bus payload nested inside [`crate::WireMessage::body`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvtBody {
    Empty,
    DeckUpdated {
        id: u16,
        playing: bool,
        volume: f32,
        speed: f32,
        eq: DeckEq,
        filter_db: f32,
        gain_trim_db: f32,
        headphone_cue: bool,
        sync_mode: SyncMode,
        cue_point_secs: Option<f64>,
        quantize: bool,
        active_loop: Option<LoopRegion>,
        pad_mode: PadMode,
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
    EngineStatus {
        status: EngineStatus,
    },
    Error {
        message: String,
    },
    Notice {
        message: String,
    },
}
