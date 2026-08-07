use serde::{Deserialize, Serialize};

/// Which EQ band a single-band set targets (MIDI soft-takeover).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqBand {
    Low,
    Mid,
    High,
}

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
    pub in_ms: i32,
    pub out_ms: i32,
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

/// Jog platter policy for top (touched) or outer (untouched) turns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JogMode {
    /// Scratch / vinyl platter rate from ticks (stop, reverse).
    #[default]
    Vinyl,
    /// Temporary pitch bend; decays when idle.
    PitchBend,
    /// Ignore turns.
    Ignore,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckHotCue {
    pub slot: u8,
    pub position_ms: i32,
    pub loop_length_beats: Option<i32>,
    pub color: Option<String>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckSavedLoop {
    pub slot: u8,
    pub in_ms: i32,
    pub out_ms: i32,
    pub label: Option<String>,
    pub color: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplerPlayMode {
    #[default]
    Oneshot,
    Hold,
    Loop,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplerSlotInfo {
    pub label: Option<String>,
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub duration_ms: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplerBankInfo {
    pub id: String,
    pub name: String,
    pub play_mode: Option<SamplerPlayMode>,
    pub sort_index: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SamplerStatus {
    pub banks: Vec<SamplerBankInfo>,
    pub active_bank_id: Option<String>,
    pub active_bank_name: Option<String>,
    pub bank_play_mode: Option<SamplerPlayMode>,
    pub deck_slots: Vec<Vec<SamplerSlotInfo>>,
    pub effective_play_modes: Vec<SamplerPlayMode>,
}

/// Slim deck snapshot for status patches and full snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckSnapshot {
    pub id: u16,
    pub track: Option<String>,
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub playing: bool,
    pub volume: f32,
    pub speed: f32,
    pub eq: DeckEq,
    pub filter_db: f32,
    pub gain_trim_db: f32,
    pub headphone_cue: bool,
    pub sync_mode: SyncMode,
    pub cue_point_ms: Option<i32>,
    pub quantize: bool,
    pub active_loop: Option<LoopRegion>,
    pub pad_mode: PadMode,
    pub position_ms: Option<i32>,
    pub duration_ms: Option<i32>,
    pub hot_cues: Vec<DeckHotCue>,
    pub saved_loops: Vec<DeckSavedLoop>,
    pub loudness_lufs: Option<f64>,
    pub auto_gain_db: f32,
    pub active_sampler_bank_id: Option<String>,
    pub top_jog_mode: JogMode,
    pub outer_jog_mode: JogMode,
    pub jog_touching: bool,
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
    pub sampler: SamplerStatus,
}

/// Command bus payload nested inside [`crate::WireMessage::body`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CmdBody {
    Empty,
    Seek {
        position_ms: i32,
    },
    SetVolume {
        volume: f32,
        #[serde(default)]
        soft_takeover: bool,
    },
    SetEq {
        low: f32,
        mid: f32,
        high: f32,
    },
    SetEqBand {
        band: EqBand,
        gain_db: f32,
        #[serde(default)]
        soft_takeover: bool,
    },
    SetSpeed {
        speed: f32,
        #[serde(default)]
        soft_takeover: bool,
    },
    SetFilter {
        filter_db: f32,
        #[serde(default)]
        soft_takeover: bool,
    },
    SetGainTrim {
        gain_db: f32,
        #[serde(default)]
        soft_takeover: bool,
    },
    SetHeadphoneCue {
        enabled: bool,
    },
    SetCrossfader {
        position: f32,
        #[serde(default)]
        soft_takeover: bool,
    },
    SetCueMix {
        mix: f32,
        #[serde(default)]
        soft_takeover: bool,
    },
    SetMasterCue {
        enabled: bool,
    },
    ToggleSync {
        beat_sync: bool,
    },
    SetQuantize {
        enabled: bool,
    },
    SetAutoLoop {
        beats: u32,
    },
    BeatJump {
        beats: i32,
    },
    SetPadMode {
        mode: PadMode,
    },
    BeginLoopRoll {
        beats: u32,
    },
    TriggerHotCue {
        position_ms: i32,
    },
    RecallSavedLoop {
        in_ms: i32,
        out_ms: i32,
    },
    TriggerSampler {
        slot: u8,
    },
    EndSampler {
        slot: u8,
    },
    AssignSampler {
        slot: u8,
        path: String,
    },
    AssignSamplerTrack {
        slot: u8,
        track_id: String,
    },
    ClearSampler {
        slot: u8,
    },
    SetSamplerBank {
        bank_id: String,
    },
    CreateSamplerBank {
        name: Option<String>,
        play_mode: Option<String>,
    },
    UpdateSamplerBank {
        bank_id: String,
        name: String,
        play_mode: Option<String>,
    },
    DeleteSamplerBank {
        bank_id: String,
    },
    SaveHotCue {
        slot: u8,
    },
    DeleteHotCue {
        slot: u8,
    },
    SaveLoop {
        slot: u8,
    },
    DeleteLoop {
        slot: u8,
    },
    LoadPath {
        path: String,
    },
    LoadLibraryTrack {
        track_id: String,
    },
    JogTouch {
        touching: bool,
    },
    JogTurn {
        delta: i32,
    },
    SetJogMode {
        top: JogMode,
        outer: JogMode,
    },
}

/// Event bus payload nested inside [`crate::WireMessage::body`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvtBody {
    Empty,
    DeckUpdated {
        id: u16,
        track: Option<String>,
        track_id: Option<String>,
        title: Option<String>,
        artist: Option<String>,
        bpm: Option<f64>,
        key: Option<String>,
        playing: bool,
        volume: f32,
        speed: f32,
        eq: DeckEq,
        filter_db: f32,
        gain_trim_db: f32,
        headphone_cue: bool,
        sync_mode: SyncMode,
        cue_point_ms: Option<i32>,
        quantize: bool,
        active_loop: Option<LoopRegion>,
        pad_mode: PadMode,
        position_ms: Option<i32>,
        duration_ms: Option<i32>,
        hot_cues: Vec<DeckHotCue>,
        saved_loops: Vec<DeckSavedLoop>,
        loudness_lufs: Option<f64>,
        auto_gain_db: f32,
        active_sampler_bank_id: Option<String>,
        top_jog_mode: JogMode,
        outer_jog_mode: JogMode,
        jog_touching: bool,
    },
    Position {
        position_ms: i32,
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
