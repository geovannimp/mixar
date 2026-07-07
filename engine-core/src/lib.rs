//! Core engine orchestration for rust-dj-engine
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

mod backend;
mod callback;
mod config;
mod engine;
mod producer;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{AudioSource, DeviceInfo, LoadedAudio};
pub use backend::{create_backend, AudioBackend, AudioBackendTrait};
pub use config::{AdvancedConfig, AudioConfig, DeviceConfig, EngineConfig};
pub use engine::Engine;
