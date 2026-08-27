//! Engine evt subscription + periodic tick for performance history recording.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use engine_api::{decode_evt_body, DeckSnapshot, EvtBody};
use engine_core::EngineBuses;
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
    pub fn start(buses: &EngineBuses, library: Arc<Mutex<LibraryManager>>) -> Result<Self, String> {
        let rx = buses.subscribe_evt_all().map_err(|e| e.to_string())?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let handle = std::thread::Builder::new()
            .name("history-worker".into())
            .spawn(move || run_loop(rx, library, shutdown_flag))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            shutdown,
            handle: Some(handle),
        })
    }
}

fn run_loop(
    rx: engine_core::EvtReceiver,
    library: Arc<Mutex<LibraryManager>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut last_tick = Instant::now();
    while !shutdown.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Some(ev)) => {
                if let Ok(body) = decode_evt_body(ev.payload()) {
                    apply_evt(&library, &body);
                }
            }
            Ok(None) => {}
            Err(_) => break,
        }
        if last_tick.elapsed() >= TICK_INTERVAL {
            last_tick = Instant::now();
            if let Ok(mut lib) = library.lock() {
                let _ = lib.history_tick();
            }
        }
    }
}

fn apply_evt(library: &Arc<Mutex<LibraryManager>>, body: &EvtBody) {
    let Ok(mut lib) = library.lock() else {
        return;
    };
    match body {
        EvtBody::DeckUpdated {
            id,
            track,
            track_id,
            title,
            artist,
            bpm,
            key,
            playing,
            volume,
            duration_ms,
            ..
        } => {
            let _ = lib.history_on_deck_updated(
                *id as usize,
                DeckPlaySnapshot {
                    playing: *playing,
                    volume: *volume,
                    track_id: track_id.clone(),
                    track_path: track.clone(),
                    title: title.clone(),
                    artist: artist.clone(),
                    album: None,
                    bpm: *bpm,
                    key: key.clone(),
                    duration_ms: *duration_ms,
                },
            );
        }
        EvtBody::EngineStatus { status } => {
            let _ = lib.history_on_crossfader(status.crossfader);
            for deck in &status.decks {
                let _ = lib.history_on_deck_updated(deck.id as usize, deck_snapshot(deck));
            }
        }
        _ => {}
    }
}

fn deck_snapshot(deck: &DeckSnapshot) -> DeckPlaySnapshot {
    DeckPlaySnapshot {
        playing: deck.playing,
        volume: deck.volume,
        track_id: deck.track_id.clone(),
        track_path: deck.track.clone(),
        title: deck.title.clone(),
        artist: deck.artist.clone(),
        album: None,
        bpm: deck.bpm,
        key: deck.key.clone(),
        duration_ms: deck.duration_ms,
    }
}
