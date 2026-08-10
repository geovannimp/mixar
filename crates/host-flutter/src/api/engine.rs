//! Thin FRB surface for engine smoke: backends, devices, start/stop.

use std::sync::Mutex;

use engine_core::{create_backend, AudioBackend, EngineConfig, EngineSession};

static SESSION: Mutex<Option<EngineSession>> = Mutex::new(None);

/// Output device summary for the Flutter smoke UI.
#[derive(Clone, Debug)]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub max_channels: u16,
    pub default_sample_rates: Vec<u32>,
}

/// Compiled-in backend names (`auto` / `cpal` / `miniaudio` / `null`, …).
#[flutter_rust_bridge::frb(sync)]
pub fn list_backend_names() -> Vec<String> {
    AudioBackend::list_names()
}

/// List output devices for a backend (`create_backend` + `list_output_devices`).
pub fn list_output_devices(backend: String) -> Result<Vec<OutputDevice>, String> {
    let devices = create_backend(&backend)
        .map_err(|e| e.to_string())?
        .list_output_devices()
        .map_err(|e| e.to_string())?;
    Ok(devices
        .into_iter()
        .map(|d| OutputDevice {
            id: d.id.as_str().to_string(),
            name: d.name,
            is_default: d.is_default,
            max_channels: d.max_channels,
            default_sample_rates: d.default_sample_rates,
        })
        .collect())
}

/// Start the engine. Idempotent if a session is already running.
pub fn start_engine(
    backend: String,
    sample_rate: Option<u32>,
    buffer_size: Option<u32>,
) -> Result<(), String> {
    let mut slot = SESSION
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?;
    if slot.is_some() {
        return Ok(());
    }

    let mut config = EngineConfig::default();
    config.backend = backend;
    if let Some(sr) = sample_rate {
        config.sample_rate = sr;
    }
    if let Some(bs) = buffer_size {
        config.buffer_size = bs;
    }

    let session = EngineSession::new(config).map_err(|e| e.to_string())?;
    session
        .with_engine(|engine| engine.start().map_err(anyhow::Error::from))
        .map_err(|e| e.to_string())?;
    *slot = Some(session);
    Ok(())
}

/// Stop the engine and drop the session.
pub fn stop_engine() -> Result<(), String> {
    let mut slot = SESSION
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?;
    if let Some(session) = slot.take() {
        session
            .with_engine(|engine| engine.stop().map_err(anyhow::Error::from))
            .map_err(|e| e.to_string())?;
        drop(session);
    }
    Ok(())
}

/// Whether a session is currently held.
#[flutter_rust_bridge::frb(sync)]
pub fn engine_is_running() -> bool {
    SESSION.lock().map(|s| s.is_some()).unwrap_or(false)
}
