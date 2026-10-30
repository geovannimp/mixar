//! Core engine orchestration for Mixar
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

mod backend;
mod bus;
mod callback;
mod config;
mod control;
mod control_norm;
mod deck_pads;
mod engine;
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
mod transport;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{DeviceInfo, LoadableAudio, LoadedAudio};
pub use backend::{create_backend, AudioBackend, AudioBackendTrait};
pub use bus::{new_event_bus, CMD_CAPACITY, EVT_CAPACITY};
pub use config::{
    validate_buffer_size, AdvancedConfig, AudioConfig, DeviceConfig, EngineConfig,
    SamplerStripRouteSetting, BUFFER_SIZE_MULTIPLE, DEFAULT_TEMPO_RANGE, DEFAULT_TEMPO_RANGE_STEPS,
    MAX_DECKS,
};
pub use control::{spawn_engine_worker, EngineWorker};
pub use engine::Engine;
pub use engine_dsp::{SamplerPlayMode, SamplerStripRoute};
pub use library::PreparedTrackPlayback;
pub use library_core::{AudioSource, FileAudioSource, TrackId};
pub use pads::{
    key_shift_page_action, key_shift_page_next, key_shift_page_prev, keyboard_page_action,
    keyboard_page_next, keyboard_page_prev, PitchPadAction, DEFAULT_PITCH_PAGE,
    KEYBOARD_PAGE_COUNT, KEY_SHIFT_PAGE_COUNT,
};
pub use session::EngineSession;
pub use transport::TransportEvent;
