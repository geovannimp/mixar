//! Background position/transport notifier (~30 Hz) while the engine is running.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use engine_core::TransportEvent;
use tauri::AppHandle;

use crate::engine_controller::publish_deck;
use crate::engine_events::{emit_levels, emit_position};
use crate::{SharedAppState, NUM_DECKS};

const NOTIFIER_INTERVAL: Duration = Duration::from_millis(33);
const PEAK_HOLD_DECAY_PER_TICK: f32 = 0.04;

struct PeakHoldState {
    hold_l: [f32; NUM_DECKS],
    hold_r: [f32; NUM_DECKS],
}

impl PeakHoldState {
    fn new() -> Self {
        Self {
            hold_l: [0.0; NUM_DECKS],
            hold_r: [0.0; NUM_DECKS],
        }
    }

    fn update(&mut self, deck_id: usize, peak_l: f32, peak_r: f32) -> (f32, f32) {
        if deck_id >= NUM_DECKS {
            return (0.0, 0.0);
        }
        Self::ballistics(&mut self.hold_l[deck_id], peak_l);
        Self::ballistics(&mut self.hold_r[deck_id], peak_r);
        (self.hold_l[deck_id], self.hold_r[deck_id])
    }

    fn ballistics(hold: &mut f32, peak: f32) {
        if peak >= *hold {
            *hold = peak;
        } else {
            *hold = (*hold - PEAK_HOLD_DECAY_PER_TICK).max(0.0);
        }
    }
}

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
    let mut peak_hold = PeakHoldState::new();

    while !stop.load(Ordering::Relaxed) {
        let mut positions: Vec<(usize, f64)> = Vec::new();
        let mut levels: Vec<(usize, f32, f32)> = Vec::new();
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
                let (snapshot, level_snapshot) = session
                    .with_engine(|engine| {
                        Ok((
                            engine.deck_playback_snapshot(),
                            engine.deck_level_snapshot(),
                        ))
                    })
                    .unwrap_or((Vec::new(), Vec::new()));
                levels = level_snapshot;

                for event in transport_events {
                    match event {
                        TransportEvent::TrackEnded { deck_id } => {
                            if deck_id < NUM_DECKS {
                                state.decks[deck_id].playing = false;
                                track_ended_decks.push(deck_id);
                            }
                        }
                    }
                }

                for (deck_id, position, _duration) in snapshot {
                    if deck_id < NUM_DECKS && state.decks[deck_id].playing {
                        positions.push((deck_id, position));
                    }
                }
            }
        }

        for (deck_id, position_secs) in positions {
            emit_position(&app, deck_id, position_secs);
        }

        for (deck_id, peak_l, peak_r) in levels {
            let (peak_hold_l, peak_hold_r) = peak_hold.update(deck_id, peak_l, peak_r);
            emit_levels(&app, deck_id, peak_l, peak_r, peak_hold_l, peak_hold_r);
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
