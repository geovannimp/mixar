//! Tauri bridge for engine cmd/evt omnibus (MessagePack wire bytes).

use engine_api::{decode_wire, encode_wire, Kind, Origin, WireMessage};
use engine_core::{EngineSession, Evt};
use std::collections::HashMap;
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

fn is_high_rate(kind: &Kind) -> bool {
    matches!(kind, Kind::Position | Kind::Levels)
}

impl EvtForwarder {
    pub fn start(app: AppHandle, session: Arc<EngineSession>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let rx = session.subscribe_evt_all().expect("evt bus subscribe");
        let thread = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let first = match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(ev)) => ev,
                    Ok(None) => continue,
                    Err(_) => break,
                };

                // Drain the queue: keep every discrete evt, coalesce high-rate by origin+kind.
                // Prevents Position/Levels from starving Pause/Updated when emit is slow.
                let mut discrete: Vec<Arc<Evt>> = Vec::new();
                let mut high_rate: HashMap<(Origin, Kind), Arc<Evt>> = HashMap::new();
                let mut push = |ev: Arc<Evt>| {
                    if is_high_rate(ev.kind()) {
                        high_rate.insert((ev.origin().clone(), ev.kind().clone()), ev);
                    } else {
                        discrete.push(ev);
                    }
                };
                push(first);
                loop {
                    match rx.recv() {
                        Ok(Some(ev)) => push(ev),
                        Ok(None) => break,
                        Err(_) => return,
                    }
                }

                for ev in discrete.into_iter().chain(high_rate.into_values()) {
                    let Ok(data) = encode_wire(&WireMessage {
                        origin: ev.origin().clone(),
                        kind: ev.kind().clone(),
                        revision: session.revision(),
                        action_timestamp_ms: 0,
                        body: ev.payload().as_ref().to_vec(),
                    }) else {
                        continue;
                    };
                    if let Err(err) = app.emit(ENGINE_BUS_EVENT, data) {
                        log::warn!("failed to emit {ENGINE_BUS_EVENT}: {err}");
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
pub fn engine_publish(
    session: State<'_, SharedSession>,
    app_state: State<'_, crate::SharedAppState>,
    payload: Vec<u8>,
) -> Result<(), String> {
    let msg = decode_wire(&payload).map_err(|e| e.to_string())?;
    // ponytail: AppState still owns track metadata until load migrates; clear on Unload so
    // leftover library invokes don't see a ghost track.
    if matches!((&msg.origin, &msg.kind), (Origin::Deck(_), Kind::Unload)) {
        let Origin::Deck(deck_id) = msg.origin else {
            unreachable!()
        };
        let deck_id = deck_id as usize;
        if deck_id < crate::NUM_DECKS {
            let mut state = app_state.lock().map_err(|e| e.to_string())?;
            crate::clear_deck_info(&mut state.decks[deck_id]);
        }
    }
    let guard = session.lock().map_err(|e| e.to_string())?;
    let session = guard
        .as_ref()
        .ok_or_else(|| "Engine session not running.".to_string())?;
    session
        .publish_cmd(msg.origin, msg.kind, msg.body)
        .map_err(|e| e.to_string())
}
