//! Engine + EngineBuses without EngineSession.

mod common;

use common::recv_evt_kind;
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{spawn_engine_worker, Engine, EngineBuses, EngineConfig};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn null_config() -> EngineConfig {
    EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    }
}

#[test]
fn engine_publish_evt_without_session() {
    let mut engine = Engine::new(null_config()).unwrap();
    let buses = EngineBuses::new();
    engine.set_buses(buses.clone());
    let rx = engine.subscribe_evt_all().unwrap();
    engine
        .publish_evt(
            Origin::Engine,
            Kind::Error,
            EvtBody::Error {
                message: "missing".into(),
            },
        )
        .unwrap();
    let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
    assert_eq!(ev.kind(), &Kind::Error);
}

#[test]
fn spawn_worker_play_empty_deck_emits_error() {
    let mut engine = Engine::new(null_config()).unwrap();
    let buses = EngineBuses::new();
    engine.set_buses(buses.clone());
    let engine = Arc::new(Mutex::new(Some(engine)));
    let _worker = spawn_engine_worker(Arc::clone(&engine)).unwrap();

    let rx = buses.subscribe_evt_all().unwrap();
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    buses
        .publish_cmd(Origin::Deck(0), Kind::Play, body)
        .unwrap();

    // `recv_evt_kind` handles `Ok(None)` timeouts correctly; the previous hand
    // rolled loop used `remaining.max(1ms)` and `.expect("Error evt")`, which
    // panicked on the first timeout instead of waiting out the deadline.
    let event = recv_evt_kind(&rx, Kind::Error);
    let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("Error body") else {
        panic!("expected Error body");
    };
    assert!(
        !message.trim().is_empty(),
        "play on an empty deck must explain the failure"
    );
}
