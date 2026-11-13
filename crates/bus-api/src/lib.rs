//! Engine event schema: every engine payload and the closed [`Event`] enum.
//!
//! `events::Event` is the only thing that crosses the bus. Payload structs are
//! plain data with no wire format — the bus carries them typed, and Flutter
//! receives them through FRB as generated sealed classes.

mod events;
pub mod library;
pub mod pads;
mod payload;

/// Product display name shared by desktop hosts (Tauri / Flutter).
pub const APP_DISPLAY_NAME: &str = "Mixar";

/// Internal sentinel: the engine asks the library worker to allocate or update
/// an automatic loop slot. It is resolved by the worker and never persisted.
pub const AUTO_LOOP_SLOT: u8 = u8::MAX;

pub use audio_core::CrossfaderAssign;
pub use library::BeatGrid;
pub use library::BeatSegment;
pub use library::HotCue;
pub use library::SavedLoop;
pub use library::TrackSummary;
pub use pads::{
    key_shift_page_action, key_shift_page_next, key_shift_page_prev, keyboard_page_action,
    keyboard_page_next, keyboard_page_prev, PitchPadAction, DEFAULT_PITCH_PAGE,
    KEYBOARD_PAGE_COUNT, KEY_SHIFT_PAGE_COUNT,
};
pub use payload::{
    default_pitch_page, default_stem_eq, CrossfaderBehaviorSetting, CrossfaderCurveStatus,
    CurveGroupSpec, CurveLibrarySpec, CurvePointSpec, CurveRefSpec, CurveSegmentKind,
    CurveSegmentSpec, CurveSetSpec, CurveSpecSpec, DeckEq, DeckHotCue, DeckSavedLoop, DeckSnapshot,
    EngineStatus, EqBand, FxKind, FxSlotState, JogMode, LoopRegion, PadMode, SamplerSlotInfo,
    SyncMode, CROSSFADER_CURVE_CYCLE, DEFAULT_CHANNEL_FADER_CURVE_ID, DEFAULT_CROSSFADER_CURVE_ID,
    DEFAULT_FX_LEVEL_CURVE_ID, DEFAULT_TEMPO_CURVE_ID, FX_DIVISION_COUNT, FX_PARAM_COUNT,
    FX_SLOT_COUNT,
};

pub use events::Event;
pub use events::Topic;
pub use events::{
    AnalyzeTrack, AssignSampler, AssignSamplerTrack, BeatGridChanged, BeatJump, BeatJumpPadPress,
    BeatJumpPadRelease, BeginCueHold, BeginLoopRoll, ClearSampler, DeckLevels, DeckPosition,
    DeckUpdated, DeleteDeckHotCue, DeleteTrackHotCue, DeleteTrackLoop, EndCueHold, EndLoopRoll,
    EngineSnapshot, Error, ExitLoop, Focus, GenerateStems, HistorySessionUpdated, HotCuePadPress,
    HotCuePadRelease, HotCuesChanged, JogTouch, JogTurn, KeyShiftPadPress, KeyShiftPadRelease,
    KeyboardPadPress, KeyboardPadRelease, Load, LoopIn, LoopOut, LoopRollPadPress,
    LoopRollPadRelease, LoopsChanged, MappingAttached, MappingDetached, MappingOffer, Navigate,
    Notice, PadPress, PadRelease, Pause, Play, RecallSavedLoop, RefreshTrack, SamplerPadPress,
    SamplerPadRelease, SaveBeatGrid, SaveDeckHotCue, SaveTrackCue, SaveTrackHotCue, SaveTrackLoop,
    Seek, SetAutoLoop, SetChannelFaderCurve, SetCrossfader, SetCrossfaderAssign,
    SetCrossfaderBehavior, SetCrossfaderCurve, SetCrossfaderCurveAmount, SetCueMix, SetCuePoint,
    SetCueVolume, SetCurveLibrary, SetDeckFxRouted, SetEq, SetEqBand, SetFilter, SetFxLevelCurve,
    SetFxUnitParam, SetFxUnitSlot, SetFxUnitSlotEnabled, SetFxUnitWet, SetGainTrim,
    SetHeadphoneCue, SetJogMode, SetKeyLock, SetKeyShift, SetKeyShiftPage, SetKeyboardPage,
    SetKeyboardRoot, SetMasterCue, SetMasterDeck, SetMasterVolume, SetPadMode, SetQuantize,
    SetSlip, SetSpeed, SetStemEqBand, SetStemEqEnabled, SetTempoCurve, SetTempoRange, SetVolume,
    ToggleHeadphoneCue, ToggleMasterCue, TogglePlay, ToggleQuantize, ToggleSlip, ToggleStemEq,
    ToggleSync, TrackAnalyzed, TrackProgress, TrackUpdated, TriggerHotCue, Unload,
};
