//! Engine + EngineBuses without EngineSession.

use engine_api::{encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
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

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let event = rx
            .recv_timeout(remaining.max(Duration::from_millis(1)))
            .expect("evt bus alive")
            .expect("Error evt");
        if *event.kind() == Kind::Error {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("timeout waiting for Error evt, last was {:?}", event.kind());
        }
    }
}
