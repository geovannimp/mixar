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

/// Playback ratio assumed when a payload predates the field (`1.0` = unity).
pub fn default_playback_ratio() -> f32 {
    1.0
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

/// Neutral stem-EQ positions (`[low, mid, high]`).
pub fn default_stem_eq() -> Vec<f32> {
    vec![0.5; 3]
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
    /// Actual media playback ratio (`1.0` = unity), including the tempo-sync
    /// override. Tempo sync can require a ratio the fader span cannot reach, so
    /// this — not `speed` × `tempo_range` — is authoritative for effective tempo.
    #[serde(default = "default_playback_ratio")]
    pub playback_ratio: f32,
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
    /// True when four stem layers are attached and usable.
    #[serde(default)]
    pub stems_ready: bool,
    /// True while the deck really plays stem audio (live stems: coverage covers
    /// the playhead, otherwise the deck is on its dry mix).
    #[serde(default)]
    pub stems_audible: bool,
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
    /// Stem EQ band positions `[low, mid, high]` as `0..1` (center `0.5`).
    #[serde(default = "default_stem_eq")]
    pub stem_eq: Vec<f32>,
    /// True when the deck's EQ knobs drive the stem EQ instead of the channel EQ.
    #[serde(default)]
    pub stem_eq_enabled: bool,
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

/// Canonical id of the crossfader curve used when none is configured.
///
/// Keep in sync with `engine_core::curve_presets::DEFAULT_PRESET_ID`; the
/// engine pins the two together in a test.
pub const DEFAULT_CROSSFADER_CURVE_ID: &str = "constant_power";

/// Preset ids a button's `cycle_crossfader_curve` walks, in order.
///
/// Canonical snake_case, and exactly the presets the engine catalog marks
/// available (pinned by an engine test).
pub const CROSSFADER_CURVE_CYCLE: [&str; 3] = ["constant_power", "linear", "fast_cut"];

/// Canonical id of the channel-fader curve used when none is configured.
///
/// Keep in sync with `engine_core::config::default_channel_fader_curve`; the
/// engine pins the two together in a test.
pub const DEFAULT_CHANNEL_FADER_CURVE_ID: &str = "linear";

/// Canonical id of the tempo-fader curve used when none is configured.
///
/// Keep in sync with `engine_core::config::default_tempo_curve`; the engine
/// pins the two together in a test.
pub const DEFAULT_TEMPO_CURVE_ID: &str = "linear";

/// Canonical id of the shared FX level/depth curve used when none is configured.
///
/// Keep in sync with `engine_core::config::default_fx_level_curve`; the engine
/// pins the two together in a test.
pub const DEFAULT_FX_LEVEL_CURVE_ID: &str = "linear";

/// Crossfader behavior flags: curve bypass and A/B swap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossfaderBehaviorSetting {
    /// Curve bypassed: both sides stay at full gain at every position.
    #[serde(default)]
    pub disabled: bool,
    /// Swap the A and B gains (hamster); the position is not mirrored.
    #[serde(default)]
    pub hamster: bool,
}

/// One point of a custom curve, in the normalized `0..=1` domain.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePointSpec {
    pub x: f32,
    pub y: f32,
}

/// Interpolation rule for the span between two adjacent points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveSegmentKind {
    #[default]
    Linear,
    Exponential,
    Logarithmic,
    CubicBezier,
}

/// One segment of a custom curve.
///
/// `rate` applies to [`CurveSegmentKind::Exponential`] and
/// [`CurveSegmentKind::Logarithmic`]; the four handles apply to
/// [`CurveSegmentKind::CubicBezier`]. Unused fields stay at `0`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveSegmentSpec {
    pub kind: CurveSegmentKind,
    #[serde(default)]
    pub rate: f32,
    #[serde(default)]
    pub c1x: f32,
    #[serde(default)]
    pub c1y: f32,
    #[serde(default)]
    pub c2x: f32,
    #[serde(default)]
    pub c2y: f32,
}

/// A validated custom curve: points plus one segment per adjacent pair.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveSpecSpec {
    pub points: Vec<CurvePointSpec>,
    pub segments: Vec<CurveSegmentSpec>,
}

/// A named custom curve group (one curve per output).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveGroupSpec {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    pub curves: Vec<CurveSpecSpec>,
}

/// A reference to a curve group by id.
///
/// A preset id resolves through the crossfader preset registry; any other id
/// must name a custom group in the same [`CurveLibrarySpec`]. `parameter` is
/// the preset's optional numeric argument (the `cut` of `fast_cut`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveRefSpec {
    pub id: String,
    #[serde(default)]
    pub parameter: Option<f32>,
}

/// An ordered curve set a knob sweeps by morph position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveSetSpec {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    pub members: Vec<CurveRefSpec>,
}

/// Named custom curve groups plus ordered curve sets.
///
/// This is the persisted settings form; the engine validates and compiles it
/// before installing anything. Duplicate group or set ids are rejected.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CurveLibrarySpec {
    #[serde(default)]
    pub groups: Vec<CurveGroupSpec>,
    #[serde(default)]
    pub sets: Vec<CurveSetSpec>,
}

/// Active crossfader curve as reported by [`EngineStatus`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrossfaderCurveStatus {
    /// Canonical id of the active preset, custom group, or curve set.
    pub id: String,
    /// Human-readable name for Settings and status badges.
    pub display_name: String,
    /// Curve-set morph position `0..=1` (`0` for a single curve).
    pub morph: f32,
    /// A and B gains swapped (hamster).
    #[serde(default)]
    pub hamster: bool,
    /// Curve bypassed (both sides at full gain).
    #[serde(default)]
    pub disabled: bool,
}

impl Default for CrossfaderCurveStatus {
    fn default() -> Self {
        Self {
            id: DEFAULT_CROSSFADER_CURVE_ID.to_string(),
            display_name: "Constant Power".to_string(),
            morph: 0.0,
            hamster: false,
            disabled: false,
        }
    }
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
    /// Active crossfader curve, curve-set morph, and behavior flags.
    #[serde(default)]
    pub crossfader_curve: CrossfaderCurveStatus,
    pub decks: Vec<DeckSnapshot>,
}
