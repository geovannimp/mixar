//! Engine + typed event bus without EngineSession.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_error;
use engine_api::Event;
use engine_api::Play;
use engine_core::new_event_bus;
use engine_core::spawn_engine_worker;
use engine_core::Engine;
use engine_core::EngineConfig;
use std::sync::{Arc, Mutex};

fn null_config() -> EngineConfig {
    EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    }
}

#[test]
fn engine_publish_evt_without_session() {
    let mut engine = Engine::new(null_config()).unwrap();
    let bus = new_event_bus();
    engine.set_buses(bus.clone());
    let mut rx = engine.subscribe_evt_all().unwrap();
    engine
        .publish_evt(Event::Error(engine_api::Error {
            message: "missing".into(),
            track_id: None,
            deck: None,
        }))
        .unwrap();
    let error = recv_error(&mut rx);
    assert_eq!(error.message, "missing");
}

#[test]
fn spawn_worker_play_empty_deck_emits_error() {
    let mut engine = Engine::new(null_config()).unwrap();
    let bus = new_event_bus();
    engine.set_buses(bus.clone());
    let engine = Arc::new(Mutex::new(Some(engine)));
    let _worker = spawn_engine_worker(Arc::clone(&engine)).unwrap();

    let mut rx = bus.subscribe_evt().unwrap();
    bus.publish_cmd(Event::Play(Play { deck: 0 })).unwrap();

    // `recv_error` owns the deadline/poll/disconnect handling; the previous hand
    // rolled loop used `remaining.max(1ms)` and `.expect("Error evt")`, which
    // panicked on the first timeout instead of waiting out the deadline.
    let error = recv_error(&mut rx);
    assert!(
        !error.message.trim().is_empty(),
        "play on an empty deck must explain the failure"
    );
}
