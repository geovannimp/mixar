//! Tauri glue for [`controller::ControllerEngine`].

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use controller::{ActionPublish, ControllerEngine, DeviceInfo, MappingInfo};
use engine_api::{encode_cmd_body, CmdBody, Kind, Origin};
use library_api::{EvtBody as LibraryEvtBody, Kind as LibraryKind, Origin as LibraryOrigin};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::bus_bridge::SharedSession;
use crate::library_bus::SharedLibrarySession;

pub const CONTROLLER_EVENT: &str = "controller://event";

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

pub struct ControllerHost {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ControllerHost {
    pub fn start(
        app: AppHandle,
        engine: SharedController,
        session: SharedSession,
        library: SharedLibrarySession,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                {
                    let mut bus = HostPublish {
                        session: Arc::clone(&session),
                        library: Arc::clone(&library),
                    };
                    if let Ok(mut eng) = engine.lock() {
                        if let Err(err) = eng.poll_devices() {
                            log::debug!("controller poll_devices: {err}");
                        }
                        eng.pump(&mut bus);
                        let events = eng.take_events();
                        for ev in events {
                            if let Err(err) = app.emit(CONTROLLER_EVENT, &ev) {
                                log::warn!("failed to emit {CONTROLLER_EVENT}: {err}");
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
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

pub fn open_engine(app_data: &Path, shipped: &Path) -> Result<ControllerEngine, String> {
    let mut engine = ControllerEngine::open(app_data.join("mappings"), shipped);
    engine.ensure_seeded().map_err(|e| e.to_string())?;
    Ok(engine)
}

#[tauri::command]
pub fn controller_list_mappings(
    engine: State<'_, SharedController>,
) -> Result<Vec<MappingInfo>, String> {
    engine
        .lock()
        .map_err(|e| e.to_string())?
        .list_mappings()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_list_devices(
    engine: State<'_, SharedController>,
) -> Result<Vec<DeviceInfo>, String> {
    engine
        .lock()
        .map_err(|e| e.to_string())?
        .list_devices()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_enable_mapping(
    engine: State<'_, SharedController>,
    mapping_id: String,
    port_name: Option<String>,
) -> Result<(), String> {
    engine
        .lock()
        .map_err(|e| e.to_string())?
        .enable_mapping(&mapping_id, port_name.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_disable_mapping(
    engine: State<'_, SharedController>,
    mapping_id: String,
) -> Result<(), String> {
    engine
        .lock()
        .map_err(|e| e.to_string())?
        .disable_mapping(&mapping_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_update_mapping(
    engine: State<'_, SharedController>,
    mapping_id: String,
) -> Result<(), String> {
    engine
        .lock()
        .map_err(|e| e.to_string())?
        .update_mapping(&mapping_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn controller_update_all_mappings(
    engine: State<'_, SharedController>,
) -> Result<(), String> {
    engine
        .lock()
        .map_err(|e| e.to_string())?
        .update_all_mappings()
        .map_err(|e| e.to_string())
}
