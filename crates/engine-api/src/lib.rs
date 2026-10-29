//! Engine event schema: every engine payload and the closed [`Event`] enum.
//!
//! `events::Event` is the only thing that crosses the bus. Payload structs are
//! plain data with no wire format — the bus carries them typed, and Flutter
//! receives them through FRB as generated sealed classes.

mod events;
pub mod pads;
mod payload;

/// Product display name shared by desktop hosts (Tauri / Flutter).
pub const APP_DISPLAY_NAME: &str = "Mixar";

pub use audio_core::CrossfaderAssign;
pub use pads::{
    key_shift_page_action, key_shift_page_next, key_shift_page_prev, keyboard_page_action,
    keyboard_page_next, keyboard_page_prev, PitchPadAction, DEFAULT_PITCH_PAGE,
    KEYBOARD_PAGE_COUNT, KEY_SHIFT_PAGE_COUNT,
};
pub use payload::{
    default_pitch_page, DeckEq, DeckHotCue, DeckSavedLoop, DeckSnapshot, EngineStatus, EqBand,
    FxKind, FxSlotState, JogMode, LoopRegion, PadMode, SamplerSlotInfo, SyncMode,
    FX_DIVISION_COUNT, FX_PARAM_COUNT, FX_SLOT_COUNT,
};

pub use events::Event;
pub use events::Topic;
pub use events::{
    AssignSampler, AssignSamplerTrack, BeatJump, BeatJumpPadPress, BeatJumpPadRelease,
    BeginCueHold, BeginLoopRoll, ClearSampler, DeckLevels, DeckPosition, DeckUpdated,
    DeleteDeckHotCue, EndCueHold, EndLoopRoll, EngineSnapshot, Error, ExitLoop, HotCuePadPress,
    HotCuePadRelease, JogTouch, JogTurn, KeyShiftPadPress, KeyShiftPadRelease, KeyboardPadPress,
    KeyboardPadRelease, LoopIn, LoopOut, LoopRollPadPress, LoopRollPadRelease, Notice, PadPress,
    PadRelease, Pause, Play, RecallSavedLoop, SamplerPadPress, SamplerPadRelease, SaveDeckHotCue,
    Seek, SetAutoLoop, SetCrossfader, SetCrossfaderAssign, SetCueMix, SetCuePoint, SetCueVolume,
    SetDeckFxRouted, SetEq, SetEqBand, SetFilter, SetFxUnitParam, SetFxUnitSlot,
    SetFxUnitSlotEnabled, SetFxUnitWet, SetGainTrim, SetHeadphoneCue, SetJogMode, SetKeyLock,
    SetKeyShift, SetKeyShiftPage, SetKeyboardPage, SetKeyboardRoot, SetMasterCue, SetMasterDeck,
    SetMasterVolume, SetPadMode, SetQuantize, SetSlip, SetSpeed, SetTempoRange, SetVolume,
    ToggleHeadphoneCue, ToggleMasterCue, TogglePlay, ToggleQuantize, ToggleSlip, ToggleSync,
    TriggerHotCue, Unload,
};
