//! Example application for rust-dj-engine
//!
//! This is a comprehensive example showing how to use the engine with
//! different backends, configuration, and audio processing features.

use anyhow::Result;
use engine_core::{AudioBackend, AudioSource, Engine, EngineConfig, FileAudioSource};
use log::info;
use std::path::Path;

fn main() -> Result<()> {
    // Disable sqlx query logs; SeaORM logs statements with bound parameters via
    // the `sea_orm` target when RUST_LOG includes `sea_orm=debug`.
    let env = env_logger::Env::default().filter_or("RUST_LOG", "info,sea_orm=debug,sqlx=warn");
    env_logger::Builder::from_env(env).init();

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
    engine.load_track(
        0,
        AudioSource::File(FileAudioSource::from_path(sample_path)),
    )?;
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
