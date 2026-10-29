//! One cmd bus is shared by the engine, the library worker and the controller.
//!
//! A command addressed to another owner must be ignored, not rejected: an
//! `Error` there turns a perfectly valid command (loading a deck publishes
//! `RefreshTrack` for the deck's track) into a UI toast.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers

mod common;

use std::time::Duration;

use bus_api::AnalyzeTrack;
use bus_api::Event;
use bus_api::RefreshTrack;
use engine_core::EngineConfig;
use engine_core::EngineSession;

fn null_session() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
}

/// Collect the events the engine publishes until it goes quiet.
fn drain_engine_evts(rx: &mut common::TestReceiver) -> Vec<Event> {
    let mut events = Vec::new();
    while let Ok(Some(event)) = rx.recv_timeout(Duration::from_millis(100)) {
        events.push(event);
    }
    events
}

#[test]
fn library_commands_are_ignored_by_the_engine() {
    let session = null_session();
    let mut rx = session.subscribe_evt_all().unwrap();

    for event in [
        Event::RefreshTrack(RefreshTrack {
            track_id: "t1".into(),
        }),
        Event::AnalyzeTrack(AnalyzeTrack {
            track_id: "t1".into(),
            force: false,
        }),
    ] {
        session.publish_cmd(event).expect("publish");
    }

    let events = drain_engine_evts(&mut rx);
    let errors: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Error(error) => Some(error.message.clone()),
            _ => None,
        })
        .collect();
    assert!(
        errors.is_empty(),
        "another owner's command must not surface as an engine error: {errors:?}"
    );
}

#[test]
fn an_engine_command_still_runs_after_a_library_command() {
    let session = null_session();
    let mut rx = session.subscribe_evt_all().unwrap();

    // The library command is drained first; the engine must still be responsive.
    session
        .publish_cmd(Event::RefreshTrack(RefreshTrack {
            track_id: "t1".into(),
        }))
        .expect("publish");
    session
        .publish_cmd(Event::SetCrossfader(bus_api::SetCrossfader {
            position: 0.42,
            soft_takeover: false,
        }))
        .expect("publish");

    let snapshot = common::recv_snapshot(&mut rx);
    assert!((snapshot.status.crossfader - 0.42).abs() < 1e-4);
}
