//! Integration tests for Mixar
//!
//! These tests verify that the different components work together correctly.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use anyhow::Result;
use audio_core::BusId;
use common::short_tone_fixture;
use engine_core::{AnalysisDurationMode, Engine, EngineConfig, EngineSession};
use library::{LibraryConfig, LibraryManager};
use library_core::{AudioSource, FileAudioSource};
use std::sync::{Arc, Mutex};

#[test]
fn test_engine_with_null_backend() -> Result<()> {
    let config = EngineConfig {
        backend: "null".to_string(),
        analysis_duration: AnalysisDurationMode::Complete,
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
        analysis_duration: AnalysisDurationMode::Complete,
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
        analysis_duration: AnalysisDurationMode::Complete,
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
        analysis_duration: AnalysisDurationMode::Complete,
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

/// `prepare_track_for_playback` resolves stems for a native `.stem.mp4` track;
/// `load_track_from_library` must land them on the deck instead of dropping the
/// decoded stem buffers, so the stems pad works (`stems_ready()` true) without
/// the host taking stems itself.
#[test]
fn test_engine_load_track_from_library_attaches_native_stems() -> Result<()> {
    use codec::{encode_stem_mp4, StemAtom, StemMuxFormat};

    let dir = tempfile::tempdir()?;
    let stem_path = dir.path().join("library.stem.mp4");
    let rate: u32 = 48_000;
    let frames = (rate / 10) as usize;
    let tone = |amp: f32| -> Vec<f32> {
        (0..frames * 2)
            .map(|i| amp * ((i / 2) as f32 * 0.01).sin())
            .collect()
    };
    let mix = tone(0.1);
    let stems = [tone(0.2), tone(0.3), tone(0.4), tone(0.5)];
    encode_stem_mp4(
        &stem_path,
        StemMuxFormat::Opus,
        rate,
        &mix,
        [&stems[0], &stems[1], &stems[2], &stems[3]],
        &StemAtom::default_ni(),
    )?;

    let config = EngineConfig {
        backend: "null".to_string(),
        analysis_duration: AnalysisDurationMode::Complete,
        ..Default::default()
    };

    let lib = Arc::new(Mutex::new(LibraryManager::open_in_memory(
        LibraryConfig::default(),
    )?));
    let imported = lib.lock().unwrap().import_file_path(&stem_path)?;

    let mut engine = Engine::new_with_library(config, Arc::clone(&lib))?;
    engine.start()?;
    engine.load_track_from_library(0, imported.id())?;

    assert!(
        engine
            .deck_snapshot(0)
            .expect("deck snapshot available")
            .stems_ready,
        "stems returned by prepare_track_for_playback must be attached to the deck"
    );

    engine.stop()?;
    Ok(())
}

#[test]
fn test_engine_with_auto_backend_selects_a_usable_backend() -> Result<()> {
    // "auto" must resolve to a concrete backend: CPAL when it initialises,
    // otherwise the null fallback. Assert the resolution itself — deterministic —
    // rather than that a real device opens.
    //
    // Deliberately does not `start()`/`stop()`: whether a real device accepts
    // this exact buffer size is a property of the runner's audio stack (the
    // previous version of this test passed on CI and failed on any machine with
    // a sound card: "Device callback size is 1024 frames but 512 frames were
    // configured"). Device-open behaviour is covered by `backend-cpal`'s tests.
    let backend = engine_core::AudioBackend::new("auto")?;
    assert!(
        matches!(backend.name(), "cpal" | "null"),
        "auto must resolve to cpal or fall back to null, got `{}`",
        backend.name()
    );

    // The engine must also accept the same config and enumerate devices without
    // error. A headless runner where CPAL initialises but lists no usable output
    // devices legitimately returns an empty list, so only the call is required to
    // succeed; the non-empty guarantee for the null backend is asserted by
    // `backend-null`'s own tests.
    let engine = Engine::new(EngineConfig {
        backend: "auto".to_string(),
        ..Default::default()
    })?;
    engine.list_devices()?;

    Ok(())
}

#[test]
fn test_config_serialization() -> Result<()> {
    let config = EngineConfig::default();

    // Test TOML serialization
    let toml_string = toml::to_string(&config)?;
    assert!(toml_string.contains("backend = \"auto\""));

    // Test TOML deserialization
    let parsed_config: EngineConfig = toml::from_str(&toml_string)?;
    assert_eq!(parsed_config.buffer_size, config.buffer_size);
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
    assert_eq!(loaded_config.buffer_size, config.buffer_size);
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
    // `get_bus_config` reads back the very config this test constructed, and
    // `start()` never rewrites `config.buses`, so asserting the mapping here
    // would be tautological — it would pass even if the engine ignored the mono
    // mapping entirely. The mapping itself is covered by
    // `routing::tests::mono_master_and_cue_on_adjacent_channels`; what this test
    // adds is that the engine starts with a mono master+cue config and accepts
    // deck cue routing.
    engine.stop().unwrap();
}
