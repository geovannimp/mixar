//! Bus startup: sink registration order, forwarder failure rollback, and the
//! one-owner-per-target rule across the host transports.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers

use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use bus_api::Command;
use bus_api::Event;
use bus_api::Focus;
use bus_api::Load;
use bus_api::Navigate;
use bus_api::Play;
use host_flutter::api::bus::BusTransport;
use host_flutter::api::bus::EventSink;
use host_flutter::api::library::LibraryTransport;
use host_flutter::testing;

/// A Dart sink stand-in: records what the forwarder handed it.
struct RecordingSink {
    tx: Mutex<mpsc::Sender<Event>>,
}

impl RecordingSink {
    fn new() -> (Arc<Self>, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel();
        (Arc::new(Self { tx: Mutex::new(tx) }), rx)
    }
}

impl EventSink for RecordingSink {
    fn send(&self, event: Event) -> bool {
        self.tx.lock().expect("sink lock").send(event).is_ok()
    }
}

fn recv(rx: &mpsc::Receiver<Event>) -> Option<Event> {
    rx.recv_timeout(Duration::from_secs(2)).ok()
}

#[test]
fn a_sink_registered_before_the_forwarder_started_gets_the_next_event_once() {
    let bus = BusTransport::open().unwrap();
    let (sink, events) = RecordingSink::new();
    bus.subscribe_test_sink(sink).expect("sink");

    bus.publish_evt(Event::Notice(bus_api::Notice {
        message: "hello".into(),
    }))
    .expect("publish");

    assert_eq!(
        recv(&events),
        Some(Event::Notice(bus_api::Notice {
            message: "hello".into()
        })),
        "the join point is the subscription itself: nothing before it, everything after"
    );
    assert_eq!(recv(&events), None, "one event is delivered exactly once");
}

#[test]
fn a_failed_forwarder_start_rolls_back_only_that_sink() {
    let bus = BusTransport::open().unwrap();

    // The forwarder cannot start: the sink inserted for it must go away again,
    // and nothing else may be disturbed by the attempt.
    bus.fail_next_forwarder_start();
    let (rejected, rejected_events) = RecordingSink::new();
    let failed = bus
        .subscribe_test_sink(rejected)
        .expect_err("the seam forces a forwarder start failure");
    assert!(failed.contains("forwarder start failed"), "{failed}");

    // The flag is consumed: two later sinks share one forwarder.
    let (first, first_events) = RecordingSink::new();
    let (second, second_events) = RecordingSink::new();
    bus.subscribe_test_sink(first).expect("first sink");
    bus.subscribe_test_sink(second).expect("second sink");

    bus.publish_evt(Event::Notice(bus_api::Notice {
        message: "after".into(),
    }))
    .expect("publish");

    assert!(
        recv(&first_events).is_some(),
        "the first live sink receives"
    );
    assert!(
        recv(&second_events).is_some(),
        "a second sink is not removed by the other's registration"
    );
    assert_eq!(
        recv(&rejected_events),
        None,
        "the rolled-back sink never receives"
    );
}

#[test]
fn navigate_focus_and_load_are_events() {
    let bus = BusTransport::open().unwrap();
    let mut rx = testing::subscribe_evt(&bus).unwrap();

    for event in [
        Event::Navigate(Navigate { delta: 1 }),
        Event::Focus(Focus { delta: -1 }),
        Event::Load(Load { deck: 0 }),
    ] {
        bus.publish_evt(event)
            .expect("the UI publishes these itself");
    }

    let mut seen = Vec::new();
    while seen.len() < 3 {
        let event = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("evt bus alive")
            .expect("ui event");
        seen.push(event.as_ref().clone());
    }
    assert_eq!(
        seen,
        vec![
            Event::Navigate(Navigate { delta: 1 }),
            Event::Focus(Focus { delta: -1 }),
            Event::Load(Load { deck: 0 }),
        ]
    );
}

#[test]
fn publishing_before_an_owner_registers_is_refused_visibly() {
    let bus = BusTransport::open().unwrap();
    let error = testing::publish_cmd(&bus, Command::Play(Play { deck: 0 }))
        .expect_err("no engine worker yet");
    assert!(
        error.contains("no command handler is registered"),
        "the refusal names the missing owner: {error}"
    );
}

#[test]
fn a_second_engine_worker_on_the_same_bus_fails_to_start() {
    let bus = BusTransport::open().unwrap();
    let library = LibraryTransport::open_in_memory(&bus).unwrap();
    let config = engine_core::EngineConfig {
        backend: "null".into(),
        ..engine_core::EngineConfig::default()
    };
    let first = testing::start_engine(&bus, &library, config.clone()).expect("first engine");
    let error = testing::start_engine(&bus, &library, config)
        .err()
        .expect("the engine inbox is already owned");
    assert!(
        error.contains("command target already has a handler"),
        "a duplicate worker must fail visibly, got: {error}"
    );
    drop(first);
}
