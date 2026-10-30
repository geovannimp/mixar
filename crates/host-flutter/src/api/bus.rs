//! The one event stream Flutter subscribes to.
//!
//! Every engine, library and controller event travels the same typed bus, so the
//! Flutter side opens exactly one stream instead of three. Fan-out to multiple
//! Dart sinks happens here: `BusTransport` owns the only `subscribe_evt`
//! subscription and forwards each event to every live sink, dropping a sink on
//! the first send error (the Dart end went away).

use bus_api::DeckUpdated;
use bus_api::Event;
use bus_api::SamplerSlotInfo;
use event_bus::EventBus;
use flutter_rust_bridge::frb;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::bridge::BridgeResult;
use crate::frb_generated::StreamSink;
use engine_core::MAX_DECKS;

/// How long the forwarder waits for an event before re-checking for shutdown.
const READ_TIMEOUT: Duration = Duration::from_millis(200);

/// Ephemeral sampler pad chrome per deck, indexed by deck.
///
/// The engine tracks the audio; the display label/path/track id live in the host
/// because they come from the prepared library source. Flutter still needs them
/// on `DeckUpdated`, so the forwarder stamps them in on the way out.
pub(crate) type DeckChrome = Arc<Mutex<Vec<Vec<SamplerSlotInfo>>>>;

struct EvtForwarder {
    shutdown: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// Process-wide bus plus the Dart-facing fan-out.
#[frb(opaque)]
pub struct BusTransport {
    bus: EventBus<Event>,
    sinks: Arc<Mutex<HashMap<u64, StreamSink<Event>>>>,
    next_sink: Arc<Mutex<u64>>,
    chrome: DeckChrome,
    forwarder: Mutex<Option<EvtForwarder>>,
}

impl Drop for BusTransport {
    fn drop(&mut self) {
        let mut slot = self.forwarder.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(forwarder) = slot.take() {
            forwarder.shutdown.store(true, Ordering::Relaxed);
            let _ = forwarder.handle.join();
        }
    }
}

impl BusTransport {
    /// Create the bus. Engine, library and controller transports are built on top of it.
    pub fn open() -> Result<BusTransport, String> {
        Ok(Self {
            bus: engine_core::new_event_bus(),
            sinks: Arc::new(Mutex::new(HashMap::new())),
            next_sink: Arc::new(Mutex::new(0)),
            chrome: Arc::new(Mutex::new(vec![Vec::new(); MAX_DECKS])),
            forwarder: Mutex::new(None),
        })
    }

    /// Subscribe to every event. Several sinks may be live at once.
    pub fn subscribe_events(&self, sink: StreamSink<Event>) -> Result<(), String> {
        self.ensure_forwarder()?;
        let id = {
            let mut next = self
                .next_sink
                .lock()
                .map_err(|_| "bus sink counter poisoned".to_string())?;
            *next += 1;
            *next
        };
        let mut sinks = self
            .sinks
            .lock()
            .map_err(|_| "bus sink registry poisoned".to_string())?;
        sinks.insert(id, sink);
        Ok(())
    }

    /// Publish a host-originated event (UI library navigation, today).
    pub fn publish_evt(&self, event: Event) -> Result<(), String> {
        self.bus.publish_evt(event).bridge()
    }

    /// Subscribe to every event without a Dart sink (tests only).
    #[cfg(any(test, feature = "test-utils"))]
    pub(crate) fn subscribe_evt(&self) -> Result<event_bus::Subscriber<Event>, String> {
        self.bus.subscribe_evt().bridge()
    }

    /// Clone of the bus for the engine/library/controller transports.
    pub(crate) fn bus(&self) -> EventBus<Event> {
        self.bus.clone()
    }

    /// Sampler pad chrome the engine transport owns and keeps current.
    pub(crate) fn deck_chrome(&self) -> DeckChrome {
        Arc::clone(&self.chrome)
    }

    /// Start the single forwarder thread, once.
    fn ensure_forwarder(&self) -> Result<(), String> {
        let mut slot = self
            .forwarder
            .lock()
            .map_err(|_| "bus forwarder poisoned".to_string())?;
        if slot.is_some() {
            return Ok(());
        }
        let rx = self.bus.subscribe_evt().bridge()?;
        let sinks = Arc::clone(&self.sinks);
        let chrome = Arc::clone(&self.chrome);
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let handle = std::thread::Builder::new()
            .name("bus-evt-forwarder".into())
            .spawn(move || forward_loop(rx, sinks, chrome, shutdown_flag))
            .bridge()?;
        *slot = Some(EvtForwarder { shutdown, handle });
        Ok(())
    }
}

fn forward_loop(
    mut rx: event_bus::Subscriber<Event>,
    sinks: Arc<Mutex<HashMap<u64, StreamSink<Event>>>>,
    chrome: DeckChrome,
    shutdown: Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        let event = match rx.recv_timeout(READ_TIMEOUT) {
            Ok(Some(event)) => event,
            Ok(None) => continue,
            Err(_) => break,
        };
        let event = with_deck_chrome(event, &chrome);
        // Fan out with the registry unlocked: `add` encodes and hands the event
        // to the Dart port, and holding the lock across it would stall a
        // concurrent `subscribe_events`. Take the sinks out, send, put the ones
        // that accepted it back; a failed send means that Dart stream is gone.
        let taken: Vec<(u64, StreamSink<Event>)> = {
            let Ok(mut sinks) = sinks.lock() else {
                break;
            };
            sinks.drain().collect()
        };
        let mut delivered = Vec::with_capacity(taken.len());
        for (id, sink) in taken {
            if sink.add(event.clone()).is_ok() {
                delivered.push((id, sink));
            }
        }
        if let Ok(mut sinks) = sinks.lock() {
            sinks.extend(delivered);
        }
    }
}

fn with_deck_chrome(event: Event, chrome: &DeckChrome) -> Event {
    let Event::DeckUpdated(update) = &event else {
        return event;
    };
    let slots = match chrome.lock() {
        Ok(chrome) => chrome
            .get(usize::from(update.deck))
            .cloned()
            .unwrap_or_default(),
        Err(_) => return event,
    };
    Event::DeckUpdated(DeckUpdated {
        sampler_slots: slots,
        ..update.clone()
    })
}
