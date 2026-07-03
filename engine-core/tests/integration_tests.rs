//! Integration tests for rust-dj-engine
//!
//! These tests verify that the different components work together correctly.

use anyhow::Result;
use audio_core::BusId;
use engine_core::{Engine, EngineConfig, FileAudioSource};

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
    };

    let mut engine = Engine::new(config)?;
    engine.start()?;

    // Test basic operations
    engine.load_track(0, &FileAudioSource::new("test.mp3"))?;
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
        .load_track(0, &FileAudioSource::new("test.mp3"))
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

    let mut engine = Engine::new(config)?;
    // Should not panic even if miniaudio is not available
    assert!(engine.start().is_ok());

    Ok(())
}

#[test]
fn test_producer_consumer_architecture() -> Result<()> {
    let mut config = EngineConfig::default();
    // Add a master bus to the config
    config.buses.push(audio_core::BusConfig::new(
        BusId::new("master".to_string()),
        "Master Bus".to_string(),
        audio_core::DeviceId::new("default".to_string()),
        audio_core::ChannelMapping::new(1, 2),
    ));

    let mut engine = Engine::new(config)?;

    // Test starting the engine with producer/consumer model
    engine.start()?;

    // Test that we can perform operations while the engine is running
    engine.play(0)?;
    engine.pause(0)?;
    engine.load_track(0, &FileAudioSource::new("test.mp3"))?;

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
fn test_ring_buffer_integration() -> Result<()> {
    use rtrb::RingBuffer;

    // Test ring buffer creation
    let (mut producer, mut consumer) = RingBuffer::new(1024);

    // Test basic producer/consumer operations
    let test_data = vec![1.0, 2.0, 3.0, 4.0];
    let mut written = 0;
    for &sample in &test_data {
        match producer.push(sample) {
            Ok(()) => written += 1,
            Err(_) => break,
        }
    }
    assert_eq!(written, 4);

    let mut read_buffer = vec![0.0; 4];
    let mut read = 0;
    for sample in read_buffer.iter_mut() {
        match consumer.pop() {
            Ok(value) => {
                *sample = value;
                read += 1;
            }
            Err(_) => break,
        }
    }
    assert_eq!(read, 4);
    assert_eq!(read_buffer, test_data);

    // Test overflow handling
    let large_data = vec![5.0; 2000]; // Larger than buffer capacity
    let mut written = 0;
    for &sample in &large_data {
        match producer.push(sample) {
            Ok(()) => written += 1,
            Err(_) => break,
        }
    }
    assert!(written < large_data.len()); // Should not write all data

    // Test underflow handling - read remaining data
    let mut read_buffer = vec![0.0; 1000];
    let mut read = 0;
    for sample in read_buffer.iter_mut() {
        match consumer.pop() {
            Ok(value) => {
                *sample = value;
                read += 1;
            }
            Err(_) => break,
        }
    }
    // Should read some data (the remaining samples from the large write)
    assert!(read > 0);

    Ok(())
}
