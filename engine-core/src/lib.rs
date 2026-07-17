//! Core engine orchestration for rust-dj-engine
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

mod backend;
mod callback;
mod config;
mod engine;
mod producer;
mod routing;
mod transport;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{DeviceInfo, LoadableAudio, LoadedAudio};
pub use backend::{create_backend, AudioBackend, AudioBackendTrait};
pub use config::{AdvancedConfig, AudioConfig, DeviceConfig, EngineConfig};
pub use engine::Engine;
pub use library_core::{AudioSource, FileAudioSource, TrackId};
pub use transport::TransportEvent;
