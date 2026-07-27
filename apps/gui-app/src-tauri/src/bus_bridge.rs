//! Tauri bridge for engine cmd/evt omnibus (MessagePack wire bytes).

use engine_api::{decode_wire, encode_wire, WireMessage};
use engine_core::EngineSession;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

pub const ENGINE_BUS_EVENT: &str = "engine://bus";

pub type SharedSession = Arc<Mutex<Option<Arc<EngineSession>>>>;

pub struct EvtForwarder {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EvtForwarder {
    pub fn start(app: AppHandle, session: Arc<EngineSession>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let rx = session.subscribe_evt_all().expect("evt bus subscribe");
        let thread = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(ev)) => {
                        let wire = encode_wire(&WireMessage {
                            origin: ev.origin().clone(),
                            kind: ev.kind().clone(),
                            revision: session.revision(),
                            body: ev.payload().as_ref().to_vec(),
                        })
                        .ok();
                        if let Some(data) = wire {
                            if let Err(err) = app.emit(ENGINE_BUS_EVENT, data) {
                                log::warn!("failed to emit {ENGINE_BUS_EVENT}: {err}");
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(_) => break,
                }
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

impl Drop for EvtForwarder {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn new_shared_session() -> SharedSession {
    Arc::new(Mutex::new(None))
}

pub fn install_session(holder: &SharedSession, session: Arc<EngineSession>) {
    *holder.lock().expect("shared session lock") = Some(session);
}

pub fn clear_session(holder: &SharedSession) {
    *holder.lock().expect("shared session lock") = None;
}

#[tauri::command]
pub fn engine_publish(session: State<'_, SharedSession>, payload: Vec<u8>) -> Result<(), String> {
    let guard = session.lock().map_err(|e| e.to_string())?;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Engine session not running.".to_string())?;
    let msg = decode_wire(&payload).map_err(|e| e.to_string())?;
    session
        .publish_cmd(msg.origin, msg.kind, msg.body)
        .map_err(|e| e.to_string())
}
