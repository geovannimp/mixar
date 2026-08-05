//! Tauri bridge for library cmd/evt omnibus (MessagePack wire bytes).

use library::{Evt, LibrarySession};
use library_api::{decode_evt_body, decode_wire, encode_wire, EvtBody, Kind, WireMessage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::controller_host::SharedController;
use crate::SharedAppState;

pub const LIBRARY_BUS_EVENT: &str = "library://bus";

pub type SharedLibrarySession = Arc<LibrarySession>;

pub struct LibraryEvtForwarder {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn hot_cue_slots(cues: &[library_api::HotCue]) -> [Option<i32>; 8] {
    let mut slots = [None; 8];
    for cue in cues {
        let idx = cue.slot as usize;
        if idx < 8 {
            slots[idx] = Some(cue.position_ms);
        }
    }
    slots
}

/// Push library cue positions into the controller so pads Trigger instead of Save.
fn mirror_hot_cues_to_controller(app: &AppHandle, track_id: &str, cues: &[library_api::HotCue]) {
    let Some(app_state) = app.try_state::<SharedAppState>() else {
        return;
    };
    let Some(controller) = app.try_state::<SharedController>() else {
        return;
    };
    let app_state = Arc::clone(&app_state);
    let controller = Arc::clone(&controller);
    let Ok(state) = app_state.lock() else {
        return;
    };
    let deck_ids: Vec<u16> = state
        .decks
        .iter()
        .enumerate()
        .filter_map(|(i, deck)| {
            (deck.track_id.as_deref() == Some(track_id)).then_some(i as u16)
        })
        .collect();
    drop(state);
    if deck_ids.is_empty() {
        return;
    }
    let slots = hot_cue_slots(cues);
    let Ok(mut eng) = controller.lock() else {
        return;
    };
    for deck in deck_ids {
        eng.set_deck_hot_cues(deck, slots);
    }
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
                    if *ev.kind() == Kind::HotCuesChanged {
                        if let Ok(EvtBody::HotCuesChanged { track_id, hot_cues }) =
                            decode_evt_body(ev.payload())
                        {
                            mirror_hot_cues_to_controller(&app, &track_id, &hot_cues);
                        }
                    }
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
