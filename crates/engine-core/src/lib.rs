//! Core engine orchestration for rust-dj-engine
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

mod backend;
mod bus;
mod callback;
mod config;
mod control;
mod engine;
mod producer;
mod routing;
mod session;
mod sync;
mod transport;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{DeviceInfo, LoadableAudio, LoadedAudio};
pub use backend::{create_backend, AudioBackend, AudioBackendTrait};
pub use bus::{Evt, EvtReceiver};
pub use config::{
    validate_buffer_size, AdvancedConfig, AudioConfig, DeviceConfig, EngineConfig,
    SamplerStripRouteSetting, BUFFER_SIZE_MULTIPLE,
};
pub use engine::Engine;
pub use engine_dsp::{SamplerPlayMode, SamplerStripRoute};
pub use library_core::{AudioSource, FileAudioSource, TrackId};
pub use session::EngineSession;
pub use transport::TransportEvent;
