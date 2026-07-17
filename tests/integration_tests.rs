//! Integration tests for rust-dj-engine
//!
//! These tests verify that the different components work together correctly.

use anyhow::Result;
use audio_core::AudioSource;
use engine_core::{AnalysisDurationMode, Engine, EngineConfig};
use library_core::FileAudioSource;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/fixtures/short-tone.wav")
}

#[test]
fn test_engine_with_null_backend() -> Result<()> {
    let config = EngineConfig {
        backend: "null".to_string(),
        sample_rate: 48000,
        buffer_size: 512,
        low_latency: false,
        buses: vec![],
        devices: None,
        advanced: None,
        audio: None,
        analysis_duration: AnalysisDurationMode::Complete,
    };

    let mut engine = Engine::new(config)?;
    engine.start()?;

    // Test basic operations
    engine.load_track(
        0,
        Arc::new(FileAudioSource::from_path(short_tone_fixture()).load()?),
        0.0,
    )?;
    engine.play(0)?;
    engine.pause(0)?;
    engine.stop()?;

    Ok(())
}

#[test]
fn test_engine_with_auto_backend() -> Result<()> {
    let config = EngineConfig {
        backend: "auto".to_string(),
        sample_rate: 48000,
        buffer_size: 512,
        low_latency: false,
        buses: vec![],
        devices: None,
        advanced: None,
        audio: None,
        analysis_duration: AnalysisDurationMode::Complete,
    };

    let mut engine = Engine::new(config)?;
    engine.start()?;
    engine.stop()?;

    Ok(())
}

#[test]
fn test_config_serialization() -> Result<()> {
    let config = EngineConfig::default();

    // Test TOML serialization
    let toml_string = toml::to_string(&config)?;
    assert!(toml_string.contains("sample_rate = 48000"));
    assert!(toml_string.contains("backend = \"auto\""));

    // Test TOML deserialization
    let parsed_config: EngineConfig = toml::from_str(&toml_string)?;
    assert_eq!(parsed_config.sample_rate, config.sample_rate);
    assert_eq!(parsed_config.backend, config.backend);

    Ok(())
}

#[test]
fn test_config_file_operations() -> Result<()> {
    let config = EngineConfig::default();
    let temp_path = std::env::temp_dir().join("test_config.toml");

    // Save config to file
    config.to_toml_file(&temp_path)?;

    // Load config from file
    let loaded_config = EngineConfig::from_toml_file(&temp_path)?;
    assert_eq!(loaded_config.sample_rate, config.sample_rate);
    assert_eq!(loaded_config.backend, config.backend);

    // Clean up
    std::fs::remove_file(&temp_path)?;

    Ok(())
}

#[test]
fn test_engine_deck_operations() -> Result<()> {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };

    let mut engine = Engine::new(config)?;
    engine.start()?;

    // Test deck operations
    assert!(engine.play(0).is_ok());
    assert!(engine.pause(0).is_ok());
    assert!(engine
        .load_track(
            0,
            Arc::new(FileAudioSource::from_path(short_tone_fixture()).load()?),
            0.0,
        )
        .is_ok());

    // Test invalid deck
    assert!(engine.play(2).is_err());

    engine.stop()?;
    Ok(())
}

#[test]
fn test_backend_fallback() -> Result<()> {
    // Test that auto backend falls back gracefully
    let config = EngineConfig {
        backend: "auto".to_string(),
        ..Default::default()
    };

    let engine = Engine::new(config)?;
    // Should not panic even if miniaudio is not available
    assert!(engine.start().is_ok());

    Ok(())
}
