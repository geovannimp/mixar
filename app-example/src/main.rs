//! Example application for rust-dj-engine
//!
//! This is a minimal example showing how to use the engine.

use engine_core::{Engine, EngineConfig};
use anyhow::Result;
use log::info;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    info!("Starting rust-dj-engine example");
    
    // Create engine configuration
    let config = EngineConfig::default();
    info!("Engine config: {:?}", config);
    
    // Create engine
    let mut engine = Engine::new(config)?;
    info!("Engine created successfully");
    
    // Start engine
    engine.start()?;
    info!("Engine started");
    
    // Load a track (placeholder)
    engine.load_track(0, "example.mp3")?;
    info!("Track loaded");
    
    // Play the track
    engine.play(0)?;
    info!("Track playing");
    
    // Let it play for a bit (in a real app, this would be event-driven)
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // Pause the track
    engine.pause(0)?;
    info!("Track paused");
    
    // Stop the engine
    engine.stop()?;
    info!("Engine stopped");
    
    info!("Example completed successfully");
    Ok(())
}
