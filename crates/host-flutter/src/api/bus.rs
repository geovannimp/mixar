//! The one event stream Flutter subscribes to.
//!
//! Every engine, library and controller event travels the same typed bus, so the
//! Flutter side opens exactly one stream instead of three. Fan-out to multiple
//! Dart sinks happens here: `BusTransport` owns the only `subscribe_evt`
//! subscription and forwards each event to every live sink, dropping a sink on
//! the first send error (the Dart end went away).

use bus_api::Command;
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

/// What the fan-out needs from one Dart stream.
///
/// Production sinks are `StreamSink<Event>`. The `test-utils` build lets a test
/// register its own sink, so the startup sequence (register, then start the
/// forwarder) can be exercised without a live Dart port.
#[cfg(any(test, feature = "test-utils"))]
pub trait EventSink: Send + Sync + 'static {
    /// Send one event; `false` means that stream is gone.
    fn send(&self, event: Event) -> bool;
}

#[cfg(not(any(test, feature = "test-utils")))]
trait EventSink: Send + Sync + 'static {
    fn send(&self, event: Event) -> bool;
}

impl EventSink for StreamSink<Event> {
    fn send(&self, event: Event) -> bool {
        self.add(event).is_ok()
    }
}

struct EvtForwarder {
    shutdown: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// Process-wide bus plus the Dart-facing fan-out.
#[frb(opaque)]
pub struct BusTransport {
    bus: EventBus<Command, Event>,
    sinks: Arc<Mutex<HashMap<u64, Arc<dyn EventSink>>>>,
    next_sink: Arc<Mutex<u64>>,
    chrome: DeckChrome,
    forwarder: Mutex<Option<EvtForwarder>>,
    /// Test seam: make the next forwarder start fail, to prove the sink rollback.
    #[cfg(any(test, feature = "test-utils"))]
    fail_next_forwarder: AtomicBool,
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
            #[cfg(any(test, feature = "test-utils"))]
            fail_next_forwarder: AtomicBool::new(false),
        })
    }

    /// Subscribe to every event. Several sinks may be live at once.
    ///
    /// The sink is registered before a newly created forwarder is started, so
    /// there is no window where the Rust subscription is live and the Dart
    /// stream is not: anything published after this returns is delivered once.
    pub fn subscribe_events(&self, sink: StreamSink<Event>) -> Result<(), String> {
        self.subscribe_sink(Arc::new(sink)).map(|_id| ())
    }

    /// Register one sink, then make sure a forwarder is running for it.
    ///
    /// The two steps cannot be reordered: a forwarder that started first could
    /// consume an event with no sink registered, and a sink registered last
    /// would miss events in between.
    fn subscribe_sink(&self, sink: Arc<dyn EventSink>) -> Result<u64, String> {
        let mut slot = self
            .forwarder
            .lock()
            .map_err(|_| "bus forwarder poisoned".to_string())?;
        let id = {
            let mut next = self
                .next_sink
                .lock()
                .map_err(|_| "bus sink counter poisoned".to_string())?;
            *next += 1;
            *next
        };
        self.sinks
            .lock()
            .map_err(|_| "bus sink registry poisoned".to_string())?
            .insert(id, sink);
        // An existing forwarder is already reading, so insertion above is the
        // join point. A new one must start with this sink already in place.
        if let Err(err) = self.start_forwarder(&mut slot) {
            // Roll back only the sink this call inserted.
            if let Ok(mut sinks) = self.sinks.lock() {
                sinks.remove(&id);
            }
            return Err(err);
        }
        Ok(id)
    }

    /// Test seam: register a sink without a Dart port.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn subscribe_test_sink(&self, sink: Arc<dyn EventSink>) -> Result<u64, String> {
        self.subscribe_sink(sink)
    }

    /// Test seam: the next forwarder start fails, as a broken Dart port would.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn fail_next_forwarder_start(&self) {
        self.fail_next_forwarder.store(true, Ordering::Relaxed);
    }

    /// Publish a host-originated event (UI library navigation, today).
    pub fn publish_evt(&self, event: Event) -> Result<(), String> {
        self.bus.publish_evt(event).map(|_report| ()).bridge()
    }

    /// Subscribe to every event without a Dart sink (tests only).
    #[cfg(any(test, feature = "test-utils"))]
    pub(crate) fn subscribe_evt(
        &self,
        label: &'static str,
    ) -> Result<event_bus::Subscriber<Arc<Event>>, String> {
        self.bus.subscribe_evt(label).bridge()
    }

    /// Clone of the bus for the engine/library/controller transports.
    pub(crate) fn bus(&self) -> EventBus<Command, Event> {
        self.bus.clone()
    }

    /// Sampler pad chrome the engine transport owns and keeps current.
    pub(crate) fn deck_chrome(&self) -> DeckChrome {
        Arc::clone(&self.chrome)
    }

    /// Start the single forwarder thread, once.
    ///
    /// The caller holds the forwarder guard, so this must not lock it again.
    fn start_forwarder(&self, slot: &mut Option<EvtForwarder>) -> Result<(), String> {
        if slot.is_some() {
            return Ok(());
        }
        #[cfg(any(test, feature = "test-utils"))]
        if self.fail_next_forwarder.swap(false, Ordering::Relaxed) {
            return Err("test: forwarder start failed".to_string());
        }
        // Unfiltered: Flutter renders every event, and `DeckUpdated` gates the UI.
        let rx = self.bus.subscribe_evt("flutter").bridge()?;
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
    mut rx: event_bus::Subscriber<Arc<Event>>,
    sinks: Arc<Mutex<HashMap<u64, Arc<dyn EventSink>>>>,
    chrome: DeckChrome,
    shutdown: Arc<AtomicBool>,
) {
    while !shutdown.load(Ordering::Relaxed) {
        let event = match rx.recv_timeout(READ_TIMEOUT) {
            Ok(Some(event)) => event,
            Ok(None) => continue,
            Err(_) => break,
        };
        // Own the event only where chrome decoration and FRB encoding need it.
        let event = with_deck_chrome(Arc::unwrap_or_clone(event), &chrome);
        // Fan out with the registry unlocked: `add` encodes and hands the event
        // to the Dart port, and holding the lock across it would stall a
        // concurrent `subscribe_events`. Take the sinks out, send, put the ones
        // that accepted it back; a failed send means that Dart stream is gone.
        let taken: Vec<(u64, Arc<dyn EventSink>)> = {
            let Ok(mut sinks) = sinks.lock() else {
                break;
            };
            sinks.drain().collect()
        };
        let mut delivered = Vec::with_capacity(taken.len());
        for (id, sink) in taken {
            if sink.send(event.clone()) {
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
