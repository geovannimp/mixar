//! Core engine orchestration for Mixar
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

mod backend;
mod bus;
mod callback;
mod channel_fader_curve;
mod config;
mod control;
mod control_norm;
mod curve_library;
pub mod curve_presets;
mod deck_pads;
mod engine;
pub mod fader_start;
mod fx_level_curve;
mod library_bridge;
mod load;
mod mixer;
mod pads;
mod playback;
mod producer;
mod routing;
mod sampler;
mod session;
mod soft_takeover;
mod status;
mod stems;
mod sync;
mod tempo_curve;
mod transport;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{CrossfaderCurveBehavior, ResolvedCrossfaderCurve};
pub use audio_core::{DeviceInfo, LoadableAudio, LoadedAudio};
pub use backend::{create_backend, AudioBackend, AudioBackendTrait};
pub use bus::{new_event_bus, CMD_CAPACITY, EVT_CAPACITY};
pub use channel_fader_curve::{
    linear_channel_fader_curve, resolve_channel_fader_curve, ChannelFaderCurveSelection,
    ResolvedChannelFaderCurve, CHANNEL_FADER_CHANNELS,
};
pub use config::{
    validate_buffer_size, AdvancedConfig, AudioConfig, DeviceConfig, EngineConfig,
    SamplerStripRouteSetting, BUFFER_SIZE_MULTIPLE, DEFAULT_TEMPO_RANGE, DEFAULT_TEMPO_RANGE_STEPS,
    MAX_DECKS,
};
pub use control::{spawn_engine_worker, EngineWorker};
pub use curve_core::{CurveGroup, ResolvedCurveGroup};
pub use curve_library::{
    behavior_from_setting, behavior_setting, group_from_spec, group_to_spec, library_from_spec,
    resolve_selection, CrossfaderCurveSelection, ResolvedCurveSelection,
};
pub use curve_presets::{
    available_presets, catalog as crossfader_preset_catalog, CrossfaderCurveError,
    CrossfaderPreset, CrossfaderPresetError, CrossfaderPresetInfo, CrossfaderPresetParameter,
    DEFAULT_PRESET_ID,
};
pub use engine::Engine;
pub use engine_dsp::{SamplerPlayMode, SamplerStripRoute};
pub use fx_level_curve::{
    linear_fx_level_curve, resolve_fx_level_curve, FxLevelCurveSelection, ResolvedFxLevelCurve,
    FX_LEVEL_CURVE_CHANNELS,
};
pub use library::PreparedTrackPlayback;
pub use library_core::{AudioSource, FileAudioSource, TrackId};
pub use pads::{
    key_shift_page_action, key_shift_page_next, key_shift_page_prev, keyboard_page_action,
    keyboard_page_next, keyboard_page_prev, PitchPadAction, DEFAULT_PITCH_PAGE,
    KEYBOARD_PAGE_COUNT, KEY_SHIFT_PAGE_COUNT,
};
pub use session::EngineSession;
pub use tempo_curve::{
    linear_tempo_curve, resolve_tempo_curve, ResolvedTempoCurve, TempoCurveSelection,
    TEMPO_CURVE_CHANNELS,
};
pub use transport::TransportEvent;
