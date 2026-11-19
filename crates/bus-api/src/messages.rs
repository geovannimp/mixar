//! One ordinary struct per payload that travels on the bus.
//!
//! These are plain data: field names and types are the schema, the direction a
//! payload travels is declared in [`crate::commands`] and [`crate::events`],
//! and shared data types live in [`crate::payload`], [`crate::library`] and
//! [`crate::pads`].

use crate::library::BeatGrid;
use crate::library::HotCue;
use crate::library::SavedLoop;
use crate::library::TrackSummary;
use crate::payload::CurveLibrarySpec;
use crate::payload::DeckEq;
use crate::payload::DeckHotCue;
use crate::payload::DeckSavedLoop;
use crate::payload::DeckSnapshot;
use crate::payload::EngineStatus;
use crate::payload::EqBand;
use crate::payload::FxKind;
use crate::payload::FxSlotState;
use crate::payload::JogMode;
use crate::payload::LoopRegion;
use crate::payload::PadMode;
use crate::payload::SamplerSlotInfo;
use crate::payload::SyncMode;
use audio_core::CrossfaderAssign;

#[derive(Clone, Debug, PartialEq)]
pub struct Play {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pause {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TogglePlay {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Seek {
    pub deck: u16,
    pub position_ms: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unload {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetVolume {
    pub deck: u16,
    pub volume: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetEq {
    pub deck: u16,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetEqBand {
    pub deck: u16,
    pub band: EqBand,
    pub gain: f32,
    pub soft_takeover: bool,
}

/// Stem EQ band, independent of the deck's pad mode (low = drums, mid = bass + other, high = vocals).
#[derive(Clone, Debug, PartialEq)]
pub struct SetStemEqBand {
    pub deck: u16,
    pub band: EqBand,
    pub gain: f32,
    pub soft_takeover: bool,
}

/// Route the deck's EQ knobs to the stem EQ.
#[derive(Clone, Debug, PartialEq)]
pub struct SetStemEqEnabled {
    pub deck: u16,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleStemEq {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetFilter {
    pub deck: u16,
    pub filter: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetGainTrim {
    pub deck: u16,
    pub gain_trim: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetHeadphoneCue {
    pub deck: u16,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleHeadphoneCue {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetCrossfaderAssign {
    pub deck: u16,
    pub assignment: CrossfaderAssign,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetSpeed {
    pub deck: u16,
    pub speed: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetTempoRange {
    pub deck: u16,
    pub tempo_range: f32,
}

/// Select the deck's tempo-fader curve by canonical id.
///
/// An unknown or multi-channel id installs `linear` and publishes a notice.
#[derive(Clone, Debug, PartialEq)]
pub struct SetTempoCurve {
    pub deck: u16,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetKeyLock {
    pub deck: u16,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetKeyShift {
    pub deck: u16,
    pub semitones: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetKeyboardRoot {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetKeyboardPage {
    pub deck: u16,
    pub page: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetKeyShiftPage {
    pub deck: u16,
    pub page: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleSync {
    pub deck: u16,
    pub beat_sync: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetMasterDeck {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetCuePoint {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeginCueHold {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndCueHold {
    pub deck: u16,
}

/// Native controller trigger: this deck's channel fader left its zero band.
#[derive(Clone, Debug, PartialEq)]
pub struct FaderStart {
    pub deck: u16,
}

/// Native controller trigger: this deck's channel fader returned to its zero band.
#[derive(Clone, Debug, PartialEq)]
pub struct FaderStop {
    pub deck: u16,
}

/// Set whether a controller mapping owns fader-start events for this deck.
/// `native = true` suppresses the software volume threshold; `false` restores it.
/// This ownership flag does not change whether fader start is enabled.
#[derive(Clone, Debug, PartialEq)]
pub struct SetFaderStartNative {
    pub deck: u16,
    pub native: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetQuantize {
    pub deck: u16,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleQuantize {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetSlip {
    pub deck: u16,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleSlip {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetAutoLoop {
    pub deck: u16,
    pub beats: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopIn {
    pub deck: u16,
    pub position_ms: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopOut {
    pub deck: u16,
    pub position_ms: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExitLoop {
    pub deck: u16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecallSavedLoop {
    pub deck: u16,
    pub in_ms: i32,
    pub out_ms: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeginLoopRoll {
    pub deck: u16,
    pub beats: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndLoopRoll {
    pub deck: u16,
}

/// Relative beat jump: repeats within one batch add up.
#[derive(Clone, Debug, PartialEq)]
pub struct BeatJump {
    pub deck: u16,
    pub beats: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetPadMode {
    pub deck: u16,
    pub mode: PadMode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PadPress {
    pub deck: u16,
    pub slot: u8,
    pub shift: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PadRelease {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HotCuePadPress {
    pub deck: u16,
    pub slot: u8,
    pub shift: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HotCuePadRelease {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopRollPadPress {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopRollPadRelease {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeatJumpPadPress {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeatJumpPadRelease {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SamplerPadPress {
    pub deck: u16,
    pub slot: u8,
    pub shift: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SamplerPadRelease {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardPadPress {
    pub deck: u16,
    pub slot: u8,
    pub shift: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyboardPadRelease {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyShiftPadPress {
    pub deck: u16,
    pub slot: u8,
    pub shift: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyShiftPadRelease {
    pub deck: u16,
    pub slot: u8,
}

/// A retrigger is a new hit; two different cues in one batch must both fire.
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerHotCue {
    pub deck: u16,
    pub position_ms: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveDeckHotCue {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteDeckHotCue {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssignSampler {
    pub deck: u16,
    pub slot: u8,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AssignSamplerTrack {
    pub deck: u16,
    pub slot: u8,
    pub track_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClearSampler {
    pub deck: u16,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JogTouch {
    pub deck: u16,
    pub touching: bool,
}

/// Relative platter ticks: repeats within one batch add up.
#[derive(Clone, Debug, PartialEq)]
pub struct JogTurn {
    pub deck: u16,
    pub delta: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetJogMode {
    pub deck: u16,
    pub top: JogMode,
    pub outer: JogMode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetCrossfader {
    pub position: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetMasterVolume {
    pub volume: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetCueVolume {
    pub volume: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetCueMix {
    pub mix: f32,
    pub soft_takeover: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetMasterCue {
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToggleMasterCue {}

/// Replace the custom curve library. Duplicate ids reject the whole update.
#[derive(Clone, Debug, PartialEq)]
pub struct SetCurveLibrary {
    pub library: CurveLibrarySpec,
}

/// Select a preset, custom group, or curve set by canonical id.
///
/// An unknown id installs `constant_power` and publishes a notice.
#[derive(Clone, Debug, PartialEq)]
pub struct SetCrossfaderCurve {
    pub id: String,
}

/// Sweep the curve set `set` to morph position `amount` (`0..=1`).
#[derive(Clone, Debug, PartialEq)]
pub struct SetCrossfaderCurveAmount {
    pub set: String,
    pub amount: f32,
    pub soft_takeover: bool,
}

/// Set the curve-bypass and hamster flags together.
#[derive(Clone, Debug, PartialEq)]
pub struct SetCrossfaderBehavior {
    pub hamster: bool,
    pub disabled: bool,
}

/// Select the channel-fader curve for every channel by canonical id.
///
/// An unknown or multi-channel id installs `linear` and publishes a notice.
#[derive(Clone, Debug, PartialEq)]
pub struct SetChannelFaderCurve {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetFxUnitSlot {
    pub slot: u8,
    pub kind: FxKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetFxUnitSlotEnabled {
    pub slot: u8,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetFxUnitWet {
    pub slot: u8,
    pub wet: f32,
    pub soft_takeover: bool,
}

/// Select the shared FX level/depth curve by canonical id.
///
/// An unknown or multi-channel id installs `linear` and publishes a notice.
#[derive(Clone, Debug, PartialEq)]
pub struct SetFxLevelCurve {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetFxUnitParam {
    pub slot: u8,
    pub param: u8,
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetDeckFxRouted {
    pub deck: u16,
    pub routed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckUpdated {
    pub deck: u16,
    pub track: Option<String>,
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub isrc: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub playing: bool,
    pub crossfader_assign: CrossfaderAssign,
    pub volume: f32,
    pub speed: f32,
    pub tempo_range: f32,
    pub playback_ratio: f32,
    pub key_lock: bool,
    pub key_shift: f32,
    pub keyboard_page: u8,
    pub key_shift_page: u8,
    pub keyboard_root_hot_cue: u8,
    pub eq: DeckEq,
    pub filter: f32,
    pub gain_trim: f32,
    pub fx_routed: bool,
    pub fx: Vec<FxSlotState>,
    pub headphone_cue: bool,
    pub sync_mode: SyncMode,
    pub cue_point_ms: Option<i32>,
    pub quantize: bool,
    pub active_loop: Option<LoopRegion>,
    pub pending_loop_in_ms: Option<i32>,
    pub slip_enabled: bool,
    pub slip_shadow_position_ms: Option<i32>,
    pub pad_mode: PadMode,
    pub stems_ready: bool,
    pub stems_audible: bool,
    pub stems_live: bool,
    pub stem_mute: Vec<bool>,
    pub stem_isolate: Option<u8>,
    pub stem_eq: Vec<f32>,
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
    pub sampler_slots: Vec<SamplerSlotInfo>,
}

/// Playhead tick for one deck.
#[derive(Clone, Debug, PartialEq)]
pub struct DeckPosition {
    pub deck: u16,
    pub position_ms: i32,
    pub slip_shadow_position_ms: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckLevels {
    pub deck: Option<u16>,
    pub peak_l: f32,
    pub peak_r: f32,
    pub peak_hold_l: f32,
    pub peak_hold_r: f32,
}

/// Whole-engine snapshot: mixer state plus every deck.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineSnapshot {
    pub status: EngineStatus,
}

/// Two errors are two problems to report; never folded.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub message: String,
    pub track_id: Option<String>,
    pub deck: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnalyzeTrack {
    pub track_id: String,
    pub force: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateStems {
    pub track_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefreshTrack {
    pub track_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveBeatGrid {
    pub track_id: String,
    pub bpm: f64,
    pub first_beat_secs: f32,
    pub replace: bool,
}

/// Persist a deck's cue point as the track's manual cue.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveTrackCue {
    pub track_id: String,
    pub position_ms: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveTrackHotCue {
    pub track_id: String,
    pub slot: u8,
    pub position_ms: i32,
    pub loop_length_beats: Option<i32>,
    pub color: Option<String>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteTrackHotCue {
    pub track_id: String,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveTrackLoop {
    pub track_id: String,
    pub slot: u8,
    pub in_ms: i32,
    pub out_ms: i32,
    pub label: Option<String>,
    pub color: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteTrackLoop {
    pub track_id: String,
    pub slot: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackAnalyzed {
    pub track: TrackSummary,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackUpdated {
    pub track: TrackSummary,
}

/// Coarse progress for analysis / stem jobs (`phase` is a stable snake_case token).
#[derive(Clone, Debug, PartialEq)]
pub struct TrackProgress {
    pub track_id: String,
    pub phase: String,
    pub fraction: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HotCuesChanged {
    pub track_id: String,
    pub hot_cues: Vec<HotCue>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopsChanged {
    pub track_id: String,
    pub loops: Vec<SavedLoop>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BeatGridChanged {
    pub track_id: String,
    pub beat_grid: BeatGrid,
}

/// Open history session entries or boundaries changed (thin signal).
#[derive(Clone, Debug, PartialEq)]
pub struct HistorySessionUpdated {
    pub session_id: Option<String>,
}

/// UI-only: move library table focus by `delta` (signed row steps).
#[derive(Clone, Debug, PartialEq)]
pub struct Navigate {
    pub delta: i32,
}

/// UI-only: step browser focus to another pane (`+1` next, `-1` previous).
#[derive(Clone, Debug, PartialEq)]
pub struct Focus {
    pub delta: i32,
}

/// UI-only: load the focused library row onto `deck` (0-based).
#[derive(Clone, Debug, PartialEq)]
pub struct Load {
    pub deck: u16,
}

/// A mapped controller is plugged in but not trusted yet: offer it once.
#[derive(Clone, Debug, PartialEq)]
pub struct MappingOffer {
    pub mapping_id: String,
    pub device_id: String,
    pub device_name: String,
    pub port_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappingAttached {
    pub mapping_id: String,
    pub port_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MappingDetached {
    pub mapping_id: String,
    pub port_name: String,
}

impl From<DeckSnapshot> for DeckUpdated {
    fn from(snap: DeckSnapshot) -> Self {
        Self {
            deck: snap.id,
            track: snap.track,
            track_id: snap.track_id,
            title: snap.title,
            artist: snap.artist,
            album: snap.album,
            isrc: snap.isrc,
            bpm: snap.bpm,
            key: snap.key,
            playing: snap.playing,
            crossfader_assign: snap.crossfader_assign,
            volume: snap.volume,
            speed: snap.speed,
            tempo_range: snap.tempo_range,
            playback_ratio: snap.playback_ratio,
            key_lock: snap.key_lock,
            key_shift: snap.key_shift,
            keyboard_page: snap.keyboard_page,
            key_shift_page: snap.key_shift_page,
            keyboard_root_hot_cue: snap.keyboard_root_hot_cue,
            eq: snap.eq,
            filter: snap.filter,
            gain_trim: snap.gain_trim,
            fx_routed: snap.fx_routed,
            fx: snap.fx,
            headphone_cue: snap.headphone_cue,
            sync_mode: snap.sync_mode,
            cue_point_ms: snap.cue_point_ms,
            quantize: snap.quantize,
            active_loop: snap.active_loop,
            pending_loop_in_ms: snap.pending_loop_in_ms,
            slip_enabled: snap.slip_enabled,
            slip_shadow_position_ms: snap.slip_shadow_position_ms,
            pad_mode: snap.pad_mode,
            stems_ready: snap.stems_ready,
            stems_audible: snap.stems_audible,
            stems_live: snap.stems_live,
            stem_mute: snap.stem_mute,
            stem_isolate: snap.stem_isolate,
            stem_eq: snap.stem_eq,
            stem_eq_enabled: snap.stem_eq_enabled,
            position_ms: snap.position_ms,
            duration_ms: snap.duration_ms,
            hot_cues: snap.hot_cues,
            saved_loops: snap.saved_loops,
            loudness_lufs: snap.loudness_lufs,
            auto_gain_db: snap.auto_gain_db,
            active_sampler_bank_id: snap.active_sampler_bank_id,
            top_jog_mode: snap.top_jog_mode,
            outer_jog_mode: snap.outer_jog_mode,
            jog_touching: snap.jog_touching,
            // Filled by the host from its ephemeral sampler pad chrome.
            sampler_slots: Vec::new(),
        }
    }
}
