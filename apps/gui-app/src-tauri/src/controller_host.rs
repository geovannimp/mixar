//! Tauri glue for [`controller::ControllerEngine`].

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use controller::{
    list_input_port_names, ActionPublish, ControllerEngine, ControllerEvent, DeviceInfo,
    MappingInfo,
};
use engine_api::{encode_cmd_body, CmdBody, Kind, Origin};
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::bus_bridge::SharedSession;
use crate::library_bus::SharedLibrarySession;

pub const CONTROLLER_EVENT: &str = "controller://event";

/// MIDI pump cadence — never blocks on ALSA port enumeration.
const PUMP_INTERVAL: Duration = Duration::from_millis(5);
/// Hotplug / offer scan — MidiInput::new is expensive on Linux; own thread.
const DEVICE_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub type SharedController = Arc<Mutex<ControllerEngine>>;

struct HostPublish {
    session: SharedSession,
    library: SharedLibrarySession,
}

impl ActionPublish for HostPublish {
    fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        let Ok(bytes) = encode_cmd_body(&body) else {
            log::warn!("controller: failed to encode engine cmd {kind:?}");
            return;
        };
        let Ok(guard) = self.session.lock() else {
            return;
        };
        let Some(session) = guard.as_ref() else {
            log::debug!("controller: drop engine cmd {kind:?} (engine not running)");
            return;
        };
        if let Err(err) = session.publish_cmd(origin, kind, bytes) {
            log::warn!("controller: publish_cmd failed: {err}");
        }
    }

    fn publish_library_evt(
        &mut self,
        origin: LibraryOrigin,
        kind: LibraryKind,
        body: LibraryEvtBody,
    ) {
        if let Err(err) = self.library.publish_evt(origin, kind, body) {
            log::warn!("controller: library publish_evt failed: {err}");
        }
    }
}

fn emit_events(app: &AppHandle, events: Vec<ControllerEvent>) {
    for ev in events {
        if let Err(err) = app.emit(CONTROLLER_EVENT, &ev) {
            log::warn!("failed to emit {CONTROLLER_EVENT}: {err}");
        }
    }
}

pub struct ControllerHost {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl ControllerHost {
    pub fn start(
        app: AppHandle,
        controller_engine: SharedController,
        session: SharedSession,
        library: SharedLibrarySession,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::with_capacity(2);

        // Port scan thread — ALSA open stays off the MIDI pump path.
        {
            let stop_flag = Arc::clone(&stop);
            let controller_engine = Arc::clone(&controller_engine);
            let app = app.clone();
            threads.push(thread::spawn(move || {
                // First scan immediately (may take seconds on cold ALSA); pump runs in parallel.
                loop {
                    match list_input_port_names() {
                        Ok(ports) => {
                            let events = if let Ok(mut eng) = controller_engine.lock() {
                                eng.apply_input_ports(ports);
                                eng.take_events()
                            } else {
                                Vec::new()
                            };
                            emit_events(&app, events);
                        }
                        Err(err) => log::debug!("controller poll_devices: {err}"),
                    }
                    // Sleep in slices so shutdown is responsive.
                    let mut waited = Duration::ZERO;
                    while waited < DEVICE_POLL_INTERVAL {
                        if stop_flag.load(Ordering::Relaxed) {
                            return;
                        }
                        thread::sleep(Duration::from_millis(100));
                        waited += Duration::from_millis(100);
                    }
                }
            }));
        }

        // MIDI pump: drain midir + session ticks (CC coalesce, idle heartbeat).
        {
            let stop_flag = Arc::clone(&stop);
            let controller_engine = Arc::clone(&controller_engine);
            let app = app.clone();
            threads.push(thread::spawn(move || {
                while !stop_flag.load(Ordering::Relaxed) {
                    let mut bus = HostPublish {
                        session: Arc::clone(&session),
                        library: Arc::clone(&library),
                    };
                    let events = if let Ok(mut eng) = controller_engine.lock() {
                        eng.pump(&mut bus);
                        eng.take_events()
                    } else {
                        Vec::new()
                    };
                    emit_events(&app, events);
                    thread::sleep(PUMP_INTERVAL);
                }
            }));
        }

        Self { stop, threads }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

impl Drop for ControllerHost {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn resolve_shipped_mappings(app: &AppHandle) -> PathBuf {
    if let Ok(resource) = app.path().resource_dir() {
        let candidate = resource.join("mappings");
        if candidate.is_dir() {
            return candidate;
        }
    }
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        let candidate = dir.join("mappings");
        if candidate.is_dir() {
            return candidate;
        }
        if !dir.pop() {
            break;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../mappings")
}

#[tauri::command]
pub fn controller_list_mappings(
    controller_engine: State<'_, SharedController>,
) -> Result<Vec<MappingInfo>, String> {
    controller_engine
        .lock()
        .map_err(|e| e.to_string())?
        .list_mappings()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_list_devices(
    controller_engine: State<'_, SharedController>,
) -> Result<Vec<DeviceInfo>, String> {
    controller_engine
        .lock()
        .map_err(|e| e.to_string())?
        .list_devices()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_pending_offers(
    controller_engine: State<'_, SharedController>,
) -> Result<Vec<ControllerEvent>, String> {
    Ok(controller_engine
        .lock()
        .map_err(|e| e.to_string())?
        .pending_offers())
}

#[tauri::command]
pub fn controller_enable_mapping(
    controller_engine: State<'_, SharedController>,
    mapping_id: String,
    port_name: Option<String>,
) -> Result<(), String> {
    controller_engine
        .lock()
        .map_err(|e| e.to_string())?
        .enable_mapping(&mapping_id, port_name.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_disable_mapping(
    controller_engine: State<'_, SharedController>,
    mapping_id: String,
) -> Result<(), String> {
    controller_engine
        .lock()
        .map_err(|e| e.to_string())?
        .disable_mapping(&mapping_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_update_mapping(
    controller_engine: State<'_, SharedController>,
    mapping_id: String,
) -> Result<(), String> {
    controller_engine
        .lock()
        .map_err(|e| e.to_string())?
        .update_mapping(&mapping_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_update_all_mappings(
    controller_engine: State<'_, SharedController>,
) -> Result<(), String> {
    controller_engine
        .lock()
        .map_err(|e| e.to_string())?
        .update_all_mappings()
        .map_err(|e| e.to_string())
}