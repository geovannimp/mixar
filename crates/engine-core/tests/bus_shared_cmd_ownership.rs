//! One cmd lane carries the engine's, the library worker's and the controller's
//! commands, and each command has exactly one owner.
//!
//! A library command therefore never reaches the engine inbox — and publishing
//! one with no library worker registered is refused by the lane itself, naming
//! the missing owner, instead of being queued for nobody. Neither case turns a
//! valid command into an engine `Error` event.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers

mod common;

use std::time::Duration;

use bus_api::AnalyzeTrack;
use bus_api::Command;
use bus_api::Event;
use bus_api::RefreshTrack;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use std::sync::Arc;

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
        events.push(Arc::unwrap_or_clone(event));
    }
    events
}

#[test]
fn library_commands_are_ignored_by_the_engine() {
    let session = null_session();
    let mut rx = session.subscribe_evt_all("test").unwrap();

    for command in [
        Command::RefreshTrack(RefreshTrack {
            track_id: "t1".into(),
        }),
        Command::AnalyzeTrack(AnalyzeTrack {
            track_id: "t1".into(),
            force: false,
        }),
    ] {
        // No library worker in this session, so the lane refuses it and hands
        // the command back; the engine is not the fallback owner. The refusal
        // survives the engine's `anyhow` wrapper with its payload intact, so
        // match the variant instead of the message text.
        let error = session.publish_cmd(command).expect_err("no library owner");
        let refusal = error
            .downcast_ref::<event_bus::CommandPublishError<Command>>()
            .unwrap_or_else(|| panic!("the refusal keeps its type: {error}"));
        assert_eq!(refusal.reason, event_bus::BusError::NoCommandHandler);
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
    let mut rx = session.subscribe_evt_all("test").unwrap();

    // The library command has no owner here; the engine must still be responsive.
    session
        .publish_cmd(Command::RefreshTrack(RefreshTrack {
            track_id: "t1".into(),
        }))
        .expect_err("no library owner");
    session
        .publish_cmd(Command::SetCrossfader(bus_api::SetCrossfader {
            position: 0.42,
            soft_takeover: false,
        }))
        .expect("publish");

    let snapshot = common::recv_snapshot(&mut rx);
    assert!((snapshot.status.crossfader - 0.42).abs() < 1e-4);
}
