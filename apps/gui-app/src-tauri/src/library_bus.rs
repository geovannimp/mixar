//! Tauri bridge for library cmd/evt omnibus (MessagePack wire bytes).

use library::{Evt, LibrarySession};
use library_api::{decode_wire, encode_wire, WireMessage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

pub const LIBRARY_BUS_EVENT: &str = "library://bus";

pub type SharedLibrarySession = Arc<LibrarySession>;

pub struct LibraryEvtForwarder {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LibraryEvtForwarder {
    pub fn start(app: AppHandle, session: SharedLibrarySession) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let rx = session.subscribe_evt_all().expect("library evt bus subscribe");
        let thread = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let first = match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(ev)) => ev,
                    Ok(None) => continue,
                    Err(_) => break,
                };

                let mut batch: Vec<Arc<Evt>> = vec![first];
                loop {
                    match rx.recv() {
                        Ok(Some(ev)) => batch.push(ev),
                        Ok(None) => break,
                        Err(_) => return,
                    }
                }

                for ev in batch {
                    let Ok(data) = encode_wire(&WireMessage {
                        origin: ev.origin().clone(),
                        kind: ev.kind().clone(),
                        revision: session.revision(),
                        action_timestamp_ms: 0,
                        body: ev.payload().as_ref().to_vec(),
                    }) else {
                        continue;
                    };
                    if let Err(err) = app.emit(LIBRARY_BUS_EVENT, data) {
                        log::warn!("failed to emit {LIBRARY_BUS_EVENT}: {err}");
                    }
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

impl Drop for LibraryEvtForwarder {
    fn drop(&mut self) {
        self.stop();
    }
}

#[tauri::command]
pub fn library_publish(
    session: State<'_, SharedLibrarySession>,
    payload: Vec<u8>,
) -> Result<(), String> {
    let msg = decode_wire(&payload).map_err(|e| e.to_string())?;
    session
        .publish_cmd(msg.origin, msg.kind, msg.body)
        .map_err(|e| e.to_string())
}
