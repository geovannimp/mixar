//! Integration tests for Mixar
//!
//! These tests verify that the different components work together correctly.

mod common;

use anyhow::Result;
use audio_core::BusId;
use common::short_tone_fixture;
use engine_core::{Engine, EngineConfig, EngineSession};
use library::{LibraryConfig, LibraryManager};
use library_core::{AudioSource, FileAudioSource};
use std::sync::{Arc, Mutex};

#[test]
fn test_engine_with_null_backend() -> Result<()> {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };

    let mut engine = Engine::new(config)?;
    engine.start()?;

    // Test basic operations
    engine.load_track(
        0,
        AudioSource::File(FileAudioSource::from_path(short_tone_fixture())),
    )?;
    engine.play(0)?;
    engine.pause(0)?;
    engine.stop()?;

    Ok(())
}

#[test]
fn test_engine_loads_library_prepared_track() -> Result<()> {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };

    let lib = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default())?);
    let imported = {
        let guard = lib.lock().expect("library lock");
        guard.import_file_path(&short_tone_fixture())?
    };
    let prepared = LibraryManager::prepare_track_for_playback(&lib, imported.id())?;

    let mut engine = Engine::new(config)?;
    engine.start()?;
    engine.load_prepared_track(0, prepared)?;
    engine.play(0)?;
    engine.pause(0)?;
    engine.stop()?;

    Ok(())
}

#[test]
fn test_engine_loads_track_via_library_manager() -> Result<()> {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };

    let lib = Arc::new(Mutex::new(LibraryManager::open_in_memory(
        LibraryConfig::default(),
    )?));
    let imported = lib
        .lock()
        .unwrap()
        .import_file_path(&short_tone_fixture())?;

    let mut engine = Engine::new_with_library(config, Arc::clone(&lib))?;
    engine.start()?;
    engine.load_track_from_library(0, imported.id())?;
    engine.play(0)?;
    engine.pause(0)?;
    engine.stop()?;

    Ok(())
}

#[test]
fn test_engine_session_loads_track_via_shared_library_manager() -> Result<()> {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };

    let lib = Arc::new(Mutex::new(LibraryManager::open_in_memory(
        LibraryConfig::default(),
    )?));
    let imported = lib
        .lock()
        .unwrap()
        .import_file_path(&short_tone_fixture())?;

    let session = EngineSession::new_with_library(config, Arc::clone(&lib))?;
    session.with_engine(|engine| engine.start())?;
    session.with_engine(|engine| engine.load_track_from_library(0, imported.id()))?;
    session.with_engine(|engine| engine.play(0))?;
    session.with_engine(|engine| engine.pause(0))?;

    Ok(())
}

#[test]
fn test_engine_with_auto_backend_selects_a_usable_backend() -> Result<()> {
    // "auto" resolves to CPAL when the runner has a usable device, otherwise it
    // falls back to null. Either way backend selection and device enumeration
    // must succeed; do not assert that a real stream opens, which depends on the
    // runner's audio stack (and previously made this test environment-dependent).
    let config = EngineConfig {
        backend: "auto".to_string(),
        ..Default::default()
    };

    let engine = Engine::new(config)?;
    let devices = engine.list_devices()?;
    assert!(
        !devices.is_empty(),
        "auto backend must expose at least the null device"
    );

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
    let dir = tempfile::tempdir()?;
    let temp_path = dir.path().join("config.toml");

    // Save config to file
    config.to_toml_file(&temp_path)?;

    // Load config from file
    let loaded_config = EngineConfig::from_toml_file(&temp_path)?;
    assert_eq!(loaded_config.sample_rate, config.sample_rate);
    assert_eq!(loaded_config.backend, config.backend);

    // `tempdir` removes the file even if an assertion above panics.
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
            AudioSource::File(FileAudioSource::from_path(short_tone_fixture())),
        )
        .is_ok());

    // Test invalid deck
    assert!(engine.play(2).is_err());

    engine.stop()?;
    Ok(())
}

#[test]
fn test_backend_fallback() -> Result<()> {
    // "auto" must resolve to a backend and expose its devices without opening a
    // stream: whether a real device accepts our exact buffer size is a property
    // of the runner's audio stack, not of the engine.
    let config = EngineConfig {
        backend: "auto".to_string(),
        ..Default::default()
    };

    let engine = Engine::new(config)?;
    assert!(!engine.list_devices()?.is_empty());

    Ok(())
}

#[test]
fn test_producer_consumer_architecture() -> Result<()> {
    let mut config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    // Add a master bus to the config
    config.buses.push(audio_core::BusConfig::new(
        BusId::new("master".to_string()),
        "Master Bus".to_string(),
        audio_core::DeviceId::new("null-device".to_string()),
        audio_core::ChannelMapping::new(1, 2),
    ));

    let mut engine = Engine::new(config)?;

    // Test starting the engine with producer/consumer model
    engine.start()?;

    // Test that we can perform operations while the engine is running
    engine.play(0)?;
    engine.pause(0)?;
    engine.load_track(
        0,
        AudioSource::File(FileAudioSource::from_path(short_tone_fixture())),
    )?;

    // Test device operations
    let devices = engine.list_devices()?;
    assert!(!devices.is_empty());

    let default_device = engine.default_device()?;
    assert!(!default_device.name.is_empty());

    // Test bus operations
    let master_bus_id = BusId::new("master".to_string());
    let bus_config = engine.get_bus_config(&master_bus_id);
    assert!(bus_config.is_some());

    // Stop the engine
    engine.stop()?;

    Ok(())
}

#[test]
fn starts_with_master_and_cue_buses_on_null() {
    let config = EngineConfig {
        backend: "null".into(),
        buses: vec![
            audio_core::BusConfig::new(
                BusId::new("master"),
                "Master".into(),
                audio_core::DeviceId::new("null-device"),
                audio_core::ChannelMapping::new(3, 4),
            ),
            audio_core::BusConfig::new(
                BusId::new("cue"),
                "Preview".into(),
                audio_core::DeviceId::new("null-device"),
                audio_core::ChannelMapping::new(1, 2),
            ),
        ],
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
    assert!(engine.start().is_ok());
    engine.set_master_cue(true).expect("master cue");
    engine.set_cue_mix(1.0).expect("cue mix");
    assert_eq!(engine.master_cue(), Some(true));
    assert_eq!(engine.cue_mix(), Some(1.0));
    engine.stop().unwrap();
}

#[test]
fn starts_with_mono_master_and_cue_on_null() {
    let config = EngineConfig {
        backend: "null".into(),
        buses: vec![
            audio_core::BusConfig::new(
                BusId::new("master"),
                "Master".into(),
                audio_core::DeviceId::new("null-device"),
                audio_core::ChannelMapping::mono(1),
            ),
            audio_core::BusConfig::new(
                BusId::new("cue"),
                "Preview".into(),
                audio_core::DeviceId::new("null-device"),
                audio_core::ChannelMapping::mono(2),
            ),
        ],
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
    assert!(engine.start().is_ok());
    engine
        .set_deck_headphone_cue(0, true)
        .expect("headphone cue API");
    // The buses are configured mono; assert the engine actually adopted the
    // mono mapping rather than only that `start()` did not error.
    assert_eq!(
        engine
            .get_bus_config(&BusId::new("master"))
            .expect("master bus")
            .channels,
        audio_core::ChannelMapping::mono(1),
    );
    assert_eq!(
        engine
            .get_bus_config(&BusId::new("cue"))
            .expect("cue bus")
            .channels,
        audio_core::ChannelMapping::mono(2),
    );
    engine.stop().unwrap();
}
