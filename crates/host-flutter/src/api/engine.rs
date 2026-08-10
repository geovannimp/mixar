//! Thin FRB surface for engine smoke: backends, devices, start/stop.

use std::sync::Mutex;

use engine_core::{AudioBackend, EngineConfig, EngineSession};

// ponytail: process-wide Mutex around EngineSession — one engine, FRB calls may wait on the UI isolate.
// Upgrade: host lifecycle manager that starts/stops off the UI isolate and owns the session.
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

/// Compiled-in backend names, with `"auto"` first (config default; not from `list_names`).
#[flutter_rust_bridge::frb(sync)]
pub fn list_backend_names() -> Vec<String> {
    let mut names = AudioBackend::list_names();
    if !names.iter().any(|n| n == "auto") {
        names.insert(0, "auto".into());
    }
    names
}

/// List output devices for a backend (`AudioBackend::new` + `list_output_devices`).
pub fn list_output_devices(backend: String) -> Result<Vec<OutputDevice>, String> {
    let devices = AudioBackend::new(&backend)
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
    {
        let slot = SESSION
            .lock()
            .map_err(|_| "session lock poisoned".to_string())?;
        if slot.is_some() {
            return Ok(());
        }
    }

    let mut config = EngineConfig {
        backend,
        ..EngineConfig::default()
    };
    if let Some(sr) = sample_rate {
        config.sample_rate = sr;
    }
    if let Some(bs) = buffer_size {
        config.buffer_size = bs;
    }

    let session = EngineSession::new(config).map_err(|e| e.to_string())?;
    session
        .with_engine(|engine| engine.start())
        .map_err(|e| e.to_string())?;

    let mut slot = SESSION
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?;
    if slot.is_some() {
        // Lost a race with another start; keep the existing session.
        let _ = session.with_engine(|engine| engine.stop());
        return Ok(());
    }
    *slot = Some(session);
    Ok(())
}

/// Stop the engine and drop the session (only after a successful stop).
pub fn stop_engine() -> Result<(), String> {
    let mut slot = SESSION
        .lock()
        .map_err(|_| "session lock poisoned".to_string())?;
    let Some(session) = slot.as_mut() else {
        return Ok(());
    };
    session
        .with_engine(|engine| engine.stop())
        .map_err(|e| e.to_string())?;
    *slot = None;
    Ok(())
}

/// Whether a session is currently held (async FRB — avoids blocking the UI isolate on sync dispatch).
pub fn engine_is_running() -> bool {
    SESSION.lock().map(|s| s.is_some()).unwrap_or(false)
}
