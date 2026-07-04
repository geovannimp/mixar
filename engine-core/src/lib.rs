//! Core engine orchestration for rust-dj-engine
//!
//! This crate orchestrates the engine lifecycle, configuration,
//! and provides the main Engine API.

mod backend;
mod callback;
mod config;
mod engine;
mod producer;

pub use audio_core::{AudioSource, LoadedAudio};
pub use backend::{AudioBackend, AudioBackendTrait};
pub use config::{AdvancedConfig, AudioConfig, DeviceConfig, EngineConfig};
pub use engine::Engine;
