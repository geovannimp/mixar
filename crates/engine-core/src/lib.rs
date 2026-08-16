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
mod engine;
mod producer;
mod routing;
mod session;
mod soft_takeover;
mod sync;
mod transport;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{DeviceInfo, LoadableAudio, LoadedAudio};
pub use backend::{create_backend, AudioBackend, AudioBackendTrait};
pub use bus::{EngineBuses, Evt, EvtReceiver};
pub use config::{
    validate_buffer_size, AdvancedConfig, AudioConfig, DeviceConfig, EngineConfig,
    SamplerStripRouteSetting, BUFFER_SIZE_MULTIPLE, DEFAULT_TEMPO_RANGE, DEFAULT_TEMPO_RANGE_STEPS,
};
pub use control::{deck_snapshot_to_evt, spawn_engine_worker, EngineWorker};
pub use engine::Engine;
pub use engine_dsp::{SamplerPlayMode, SamplerStripRoute};
pub use library::PreparedTrackPlayback;
pub use library_core::{AudioSource, FileAudioSource, TrackId};
pub use session::EngineSession;
pub use transport::TransportEvent;
