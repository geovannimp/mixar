//! Example application for rust-dj-engine
//!
//! This is a comprehensive example showing how to use the engine with
//! different backends, configuration, and audio processing features.

use anyhow::Result;
use engine_core::{AudioBackend, Engine, EngineConfig};
use log::info;
use std::path::Path;

fn main() -> Result<()> {
    // Default to info-level logs when RUST_LOG is not set.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting rust-dj-engine example");

    // Discover backends and devices without creating an engine (for building config)
    let backend_names = AudioBackend::list_names();
    info!("Available backends: {:?}", backend_names);

    let backend_name = "cpal";
    if let Ok(backend) = AudioBackend::new(backend_name) {
        if let Ok(devices) = backend.list_output_devices() {
            info!("{} output devices:", backend_name);
            for d in &devices {
                let default_tag = if d.is_default { " [default]" } else { "" };
                info!("  - {} (id: {}){}", d.name, d.id.as_str(), default_tag);
            }
        }
    }

    let mut config = if Path::new("config.toml").exists() {
        info!("Loading configuration from config.toml");
        EngineConfig::from_toml_file("config.toml")?
    } else {
        info!("Using default configuration");
        EngineConfig::default()
    };

    config.backend = backend_name.to_string();
    info!("Using {} backend for audio output", backend_name);
    info!("Engine config: {:?}", config);

    // Start audio first so the deck targets the actual stream sample rate, then load and play.
    let mut engine = Engine::new(config.clone())?;
    info!("Engine created successfully");

    engine.start()?;
    info!("Engine started");

    let sample_path =
        "samples/Z8phyR - Nameless Elegy (Second Mix) (Mastered with Aurora at 57pct).wav";
    engine.load_track(0, sample_path)?;
    info!("Sample track loaded: {}", sample_path);

    // Play the track
    engine.play(0)?;
    info!("Track playing");

    // Let it play for a bit (in a real app, this would be event-driven)
    info!("Playing sample for 8 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(8));

    // Pause the track
    engine.pause(0)?;
    info!("Track paused");

    // Stop the engine
    engine.stop()?;
    info!("Engine stopped");

    info!("Example completed successfully");
    Ok(())
}

/// Demonstrate different backend capabilities
fn demonstrate_backends() -> Result<()> {
    info!("Demonstrating backend capabilities...");

    // Test null backend
    info!("Testing null backend...");
    let null_config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let mut null_engine = Engine::new(null_config)?;
    null_engine.start()?;
    null_engine.stop()?;
    info!("Null backend test completed");

    // Test miniaudio backend
    info!("Testing miniaudio backend...");
    let miniaudio_config = EngineConfig {
        backend: "miniaudio".to_string(),
        ..Default::default()
    };
    match Engine::new(miniaudio_config) {
        Ok(mut miniaudio_engine) => {
            if miniaudio_engine.start().is_ok() {
                info!("Miniaudio backend test successful");
                miniaudio_engine.stop()?;
            } else {
                info!("Miniaudio backend test failed");
            }
        }
        Err(e) => {
            info!("Miniaudio backend not available: {}", e);
        }
    }

    // Test auto backend selection
    info!("Testing auto backend selection...");
    let auto_config = EngineConfig {
        backend: "auto".to_string(),
        ..Default::default()
    };
    let mut auto_engine = Engine::new(auto_config)?;
    auto_engine.start()?;
    auto_engine.stop()?;
    info!("Auto backend selection test completed");

    Ok(())
}
