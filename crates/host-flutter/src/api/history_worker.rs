//! Engine evt subscription + periodic tick for performance history recording.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::bridge::BridgeResult;
use engine_api::DeckSnapshot;
use engine_api::DeckUpdated;
use engine_api::Event;
use event_bus::EventBus;
use event_bus::Subscriber;
use library::{DeckPlaySnapshot, LibraryManager};

const TICK_INTERVAL: Duration = Duration::from_millis(200);

#[flutter_rust_bridge::frb(ignore)]
pub(crate) struct HistoryWorker {
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for HistoryWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl HistoryWorker {
    pub fn start(
        buses: &EventBus<Event>,
        library: Arc<Mutex<LibraryManager>>,
    ) -> Result<Self, String> {
        let rx = buses.subscribe_evt().bridge()?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let handle = std::thread::Builder::new()
            .name("history-worker".into())
            .spawn(move || run_loop(rx, library, shutdown_flag))
            .bridge()?;
        Ok(Self {
            shutdown,
            handle: Some(handle),
        })
    }
}

fn run_loop(
    mut rx: Subscriber<Event>,
    library: Arc<Mutex<LibraryManager>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut last_tick = Instant::now();
    while !shutdown.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Some(event)) => apply_evt(&library, &event),
            Ok(None) => {}
            Err(_) => break,
        }
        if last_tick.elapsed() >= TICK_INTERVAL {
            last_tick = Instant::now();
            match library.lock() {
                Ok(mut lib) => {
                    if let Err(err) = lib.history_tick() {
                        eprintln!("history worker: history_tick failed: {err}");
                    }
                }
                Err(_) => eprintln!("history worker: library lock poisoned"),
            }
        }
    }
}

fn apply_evt(library: &Arc<Mutex<LibraryManager>>, event: &Event) {
    let Ok(mut lib) = library.lock() else {
        eprintln!("history worker: library lock poisoned");
        return;
    };
    match event {
        Event::DeckUpdated(update) => {
            if let Err(err) =
                lib.history_on_deck_updated(usize::from(update.deck), update_snapshot(update))
            {
                eprintln!("history worker: history_on_deck_updated failed: {err}");
            }
        }
        Event::EngineSnapshot(snapshot) => {
            if let Err(err) = lib.history_on_crossfader(snapshot.status.crossfader) {
                eprintln!("history worker: history_on_crossfader failed: {err}");
            }
            for deck in &snapshot.status.decks {
                if let Err(err) = lib.history_on_deck_updated(deck.id as usize, deck_snapshot(deck))
                {
                    eprintln!("history worker: history_on_deck_updated failed: {err}");
                }
            }
        }
        _ => {}
    }
}

fn update_snapshot(update: &DeckUpdated) -> DeckPlaySnapshot {
    DeckPlaySnapshot {
        playing: update.playing,
        volume: update.volume,
        crossfader_assign: update.crossfader_assign,
        track_id: update.track_id.clone(),
        track_path: update.track.clone(),
        title: update.title.clone(),
        artist: update.artist.clone(),
        album: update.album.clone(),
        isrc: update.isrc.clone(),
        bpm: update.bpm,
        key: update.key.clone(),
        duration_ms: update.duration_ms,
    }
}

fn deck_snapshot(deck: &DeckSnapshot) -> DeckPlaySnapshot {
    DeckPlaySnapshot {
        playing: deck.playing,
        volume: deck.volume,
        crossfader_assign: deck.crossfader_assign,
        track_id: deck.track_id.clone(),
        track_path: deck.track.clone(),
        title: deck.title.clone(),
        artist: deck.artist.clone(),
        album: deck.album.clone(),
        bpm: deck.bpm,
        key: deck.key.clone(),
        isrc: deck.isrc.clone(),
        duration_ms: deck.duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drives_history(event: &Event) -> bool {
        matches!(event, Event::DeckUpdated(_) | Event::EngineSnapshot(_))
    }

    #[test]
    fn non_deck_events_do_not_drive_history() {
        // Sampler / PFL are commands or DeckUpdated.headphone_cue — never history inputs.
        assert!(!drives_history(&Event::DeckPosition(
            engine_api::DeckPosition {
                deck: 0,
                position_ms: 0,
                slip_shadow_position_ms: None,
            }
        )));
        assert!(!drives_history(&Event::DeckLevels(
            engine_api::DeckLevels {
                deck: None,
                peak_l: 0.0,
                peak_r: 0.0,
                peak_hold_l: 0.0,
                peak_hold_r: 0.0,
            }
        )));
        assert!(!drives_history(&Event::Notice(engine_api::Notice {
            message: "hi".into(),
        })));
    }
}
