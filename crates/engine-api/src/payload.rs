use audio_core::CrossfaderAssign;
use serde::{Deserialize, Serialize};

/// FX chain capacity per deck (bounds a preset's effect count); keep in sync
/// with `engine_dsp::fx::FX_SLOT_COUNT`.
pub const FX_SLOT_COUNT: usize = 3;
/// Maximum knobs per effect; keep in sync with `engine_dsp::fx::FX_PARAM_COUNT`.
pub const FX_PARAM_COUNT: usize = 3;
/// Beat/division steps for Echo `param 0`; mirrors
/// `engine_dsp::fx::echo::ECHO_DIVISIONS.len()` (normalized value = index / (count - 1)).
pub const FX_DIVISION_COUNT: usize = 7;

/// Which EQ band a single-band set targets (MIDI soft-takeover).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqBand {
    Low,
    Mid,
    High,
}

/// Three-band deck EQ as fader positions `0..1` (center `0.5` = 0 dB).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckEq {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

/// FX unit effect selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FxKind {
    Off,
    Echo,
    Reverb,
}

/// Published per-slot FX state (shared across decks; see `DeckSnapshot::fx`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FxSlotState {
    pub kind: FxKind,
    pub enabled: bool,
    /// Wet/dry `0..1` (`0` = dry).
    pub wet: f32,
    /// Normalized knob positions `0..1` for [`Self::kind`]; unused entries `0.0`.
    pub params: Vec<f32>,
}

impl FxSlotState {
    /// Slot state the engine boots with: Echo loaded, unarmed, `1/2` beat.
    ///
    /// Mirrors `engine_dsp::fx::FxChain::new`; `engine-core` pins the two
    /// together in `tests/wire_constants.rs`, so a changed DSP default fails
    /// that test instead of drifting from what controllers mirror at boot.
    pub fn boot() -> Self {
        Self {
            kind: FxKind::Echo,
            enabled: false,
            wet: 0.5,
            params: vec![0.5, 0.5, 0.0],
        }
    }
}

impl Default for FxSlotState {
    fn default() -> Self {
        Self {
            kind: FxKind::Off,
            enabled: false,
            wet: 0.5,
            params: vec![0.0; FX_PARAM_COUNT],
        }
    }
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
    Stems,
    Keyboard,
    KeyShift,
}

/// Default Keyboard / Key Shift semitone page (Rekordbox DDJ-400 footnote *6).
pub fn default_pitch_page() -> u8 {
    crate::pads::DEFAULT_PITCH_PAGE
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SamplerSlotInfo {
    pub label: Option<String>,
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub duration_ms: Option<i32>,
}

fn default_crossfader_assign() -> CrossfaderAssign {
    CrossfaderAssign::A
}

/// Deck state for status patches and full engine snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeckSnapshot {
    pub id: u16,
    pub track: Option<String>,
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub isrc: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub playing: bool,
    /// How this deck is routed by the main crossfader.
    #[serde(default = "default_crossfader_assign")]
    pub crossfader_assign: CrossfaderAssign,
    /// Channel fader `0..1`.
    pub volume: f32,
    /// Tempo fader position `0..1` (not playback ratio).
    pub speed: f32,
    /// Tempo fader half-span as pitch fraction (`0.06` = ±6%).
    pub tempo_range: f32,
    /// Key lock (time-stretch); tempo changes without pitch when true.
    #[serde(default)]
    pub key_lock: bool,
    /// Session key-shift offset in semitones (`-16..=16`; `0` = bypass).
    #[serde(default)]
    pub key_shift: f32,
    /// Keyboard pad semitone page (`1..=4`; default 2).
    #[serde(default = "default_pitch_page")]
    pub keyboard_page: u8,
    /// Key Shift pad semitone page (`1..=5`; default 2).
    #[serde(default = "default_pitch_page")]
    pub key_shift_page: u8,
    /// Hot-cue slot used as the Keyboard pad root (default 0).
    #[serde(default)]
    pub keyboard_root_hot_cue: u8,
    pub eq: DeckEq,
    /// Filter knob `0..1` (center `0.5` = flat).
    pub filter: f32,
    /// Gain trim knob `0..1` (center `0.5` = 0 dB).
    pub gain_trim: f32,
    /// True when this deck's pre-fader signal runs through the shared FX unit.
    #[serde(default)]
    pub fx_routed: bool,
    /// Shared FX unit slots (identical for every deck).
    #[serde(default)]
    pub fx: Vec<FxSlotState>,
    pub headphone_cue: bool,
    pub sync_mode: SyncMode,
    pub cue_point_ms: Option<i32>,
    pub quantize: bool,
    pub active_loop: Option<LoopRegion>,
    /// Transient Loop In marker before Loop Out completes the region.
    #[serde(default)]
    pub pending_loop_in_ms: Option<i32>,
    #[serde(default)]
    pub slip_enabled: bool,
    #[serde(default)]
    pub slip_shadow_position_ms: Option<i32>,
    pub pad_mode: PadMode,
    /// True when four stem layers are attached for playback.
    #[serde(default)]
    pub stems_ready: bool,
    /// True when the attached stems come from the live StemgenRT ring
    /// (false for cached/offline stems).
    #[serde(default)]
    pub stems_live: bool,
    /// Per-stem mute flags (vocals, drums, bass, other).
    #[serde(default)]
    pub stem_mute: Vec<bool>,
    /// When set, only this stem index is audible (isolate).
    #[serde(default)]
    pub stem_isolate: Option<u8>,
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
    pub master_volume: f32,
    pub cue_volume: f32,
    pub cue_mix: f32,
    pub master_cue: bool,
    pub master_deck: u16,
    pub decks: Vec<DeckSnapshot>,
}
