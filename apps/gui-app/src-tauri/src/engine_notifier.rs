//! Background AppState sync for track-ended while the engine is running.
//! Position/levels stream on `engine://bus` from the control thread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use engine_core::TransportEvent;
use tauri::AppHandle;

use crate::engine_controller::publish_deck;
use crate::{SharedAppState, NUM_DECKS};

const NOTIFIER_INTERVAL: Duration = Duration::from_millis(33);

pub struct EngineNotifier {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EngineNotifier {
    pub fn start(app: AppHandle, state: SharedAppState) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let thread = thread::spawn(move || notifier_loop(app, state, stop_flag));
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

impl Drop for EngineNotifier {
    fn drop(&mut self) {
        self.stop();
    }
}

fn notifier_loop(app: AppHandle, shared_state: SharedAppState, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        let mut track_ended_decks: Vec<usize> = Vec::new();

        {
            let mut state = match shared_state.lock() {
                Ok(state) => state,
                Err(_) => {
                    thread::sleep(NOTIFIER_INTERVAL);
                    continue;
                }
            };

            if state.session.is_some() {
                let session = state.session.as_ref().unwrap();
                let transport_events = session
                    .with_engine(|engine| Ok(engine.drain_transport_events()))
                    .unwrap_or_default();

                for event in transport_events {
                    let TransportEvent::TrackEnded { deck_id } = event;
                    if deck_id < NUM_DECKS {
                        state.decks[deck_id].playing = false;
                        track_ended_decks.push(deck_id);
                    }
                }
            }
        }

        for deck_id in track_ended_decks {
            let mut state = match shared_state.lock() {
                Ok(state) => state,
                Err(_) => continue,
            };
            publish_deck(&app, &mut state, deck_id);
        }

        thread::sleep(NOTIFIER_INTERVAL);
    }
}
