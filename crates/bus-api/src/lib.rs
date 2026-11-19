//! Engine schema: every payload, plus the closed [`Command`] and [`Event`] enums.
//!
//! [`commands::Command`] is the control direction (one owner per command, see
//! [`CommandTarget`]); [`events::Event`] is the egress direction every observer
//! may watch. Payload structs live in `messages` and are plain data — no wire
//! format — so Flutter receives them through FRB as generated classes.
//!
//! # Direction is a compile-time fact
//!
//! Commands and events are disjoint enums, so a wrong-direction publication is
//! a compile error instead of a message a worker silently ignores:
//!
//! ```compile_fail
//! use bus_api::{Command, Event};
//!
//! let bus = event_bus::EventBus::<Command, Event>::new();
//! // A notice is egress: publishing it as a command does not compile.
//! let _ = bus.publish_cmd(Event::Notice(bus_api::Notice {
//!     message: "hi".into(),
//! }));
//! ```
//!
//! ```compile_fail
//! use bus_api::{Command, Event};
//!
//! let bus = event_bus::EventBus::<Command, Event>::new();
//! // Play is control: publishing it as an event does not compile.
//! let _ = bus.publish_evt(Command::Play(bus_api::Play { deck: 1 }));
//! ```

mod commands;
mod events;
pub mod library;
mod messages;
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

pub use messages::{
    AnalyzeTrack, AssignSampler, AssignSamplerTrack, BeatGridChanged, BeatJump, BeatJumpPadPress,
    BeatJumpPadRelease, BeginCueHold, BeginLoopRoll, ClearSampler, DeckLevels, DeckPosition,
    DeckUpdated, DeleteDeckHotCue, DeleteTrackHotCue, DeleteTrackLoop, EndCueHold, EndLoopRoll,
    EngineSnapshot, Error, ExitLoop, FaderStart, FaderStop, Focus, GenerateStems,
    HistorySessionUpdated, HotCuePadPress, HotCuePadRelease, HotCuesChanged, JogTouch, JogTurn,
    KeyShiftPadPress, KeyShiftPadRelease, KeyboardPadPress, KeyboardPadRelease, Load, LoopIn,
    LoopOut, LoopRollPadPress, LoopRollPadRelease, LoopsChanged, MappingAttached, MappingDetached,
    MappingOffer, Navigate, Notice, PadPress, PadRelease, Pause, Play, RecallSavedLoop,
    RefreshTrack, SamplerPadPress, SamplerPadRelease, SaveBeatGrid, SaveDeckHotCue, SaveTrackCue,
    SaveTrackHotCue, SaveTrackLoop, Seek, SetAutoLoop, SetChannelFaderCurve, SetCrossfader,
    SetCrossfaderAssign, SetCrossfaderBehavior, SetCrossfaderCurve, SetCrossfaderCurveAmount,
    SetCueMix, SetCuePoint, SetCueVolume, SetCurveLibrary, SetDeckFxRouted, SetEq, SetEqBand,
    SetFaderStartNative, SetFilter, SetFxLevelCurve, SetFxUnitParam, SetFxUnitSlot,
    SetFxUnitSlotEnabled, SetFxUnitWet, SetGainTrim, SetHeadphoneCue, SetJogMode, SetKeyLock,
    SetKeyShift, SetKeyShiftPage, SetKeyboardPage, SetKeyboardRoot, SetMasterCue, SetMasterDeck,
    SetMasterVolume, SetPadMode, SetQuantize, SetSlip, SetSpeed, SetStemEqBand, SetStemEqEnabled,
    SetTempoCurve, SetTempoRange, SetVolume, ToggleHeadphoneCue, ToggleMasterCue, TogglePlay,
    ToggleQuantize, ToggleSlip, ToggleStemEq, ToggleSync, TrackAnalyzed, TrackProgress,
    TrackUpdated, TriggerHotCue, Unload,
};

pub use commands::{Command, CommandTarget};
pub use events::{Event, EventTopic};
