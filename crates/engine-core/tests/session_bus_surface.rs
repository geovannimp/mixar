//! `EngineSession`'s public bus surface: refusals keep their type, events are shared.
//!
//! The migration changed two things an embedder can observe: `publish_cmd` now
//! reports a refused command instead of dropping it, and `subscribe_evt_all`
//! requires a label and yields `Subscriber<Arc<Event>>`. Both are covered here
//! at the session boundary, where hosts actually call them.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers

use std::sync::Arc;
use std::time::Duration;

use bus_api::Command;
use bus_api::Event;
use bus_api::RefreshTrack;
use bus_api::SetMasterVolume;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use event_bus::BusError;
use event_bus::CommandPublishError;

fn null_session() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
}

#[test]
fn publish_cmd_reports_a_refused_command_with_its_payload() {
    let session = null_session();
    // This session has no library worker, so the library target has no owner.
    let error = session
        .publish_cmd(Command::RefreshTrack(RefreshTrack {
            track_id: "t1".into(),
        }))
        .expect_err("no library owner is registered");
    let refusal = error
        .downcast_ref::<CommandPublishError<Command>>()
        .unwrap_or_else(|| panic!("the refusal keeps its type: {error}"));
    assert_eq!(refusal.reason, BusError::NoCommandHandler);
    assert!(
        matches!(refusal.command, Command::RefreshTrack(_)),
        "the unsent command comes back with its payload"
    );

    // The engine target is unaffected: one refused lane is not a dead bus.
    session
        .publish_cmd(Command::SetMasterVolume(SetMasterVolume {
            volume: 0.5,
            soft_takeover: false,
        }))
        .expect("the engine owns its inbox");
}

#[test]
fn subscribers_share_one_event_allocation() {
    let session = null_session();
    let mut first = session.subscribe_evt_all("first").expect("subscriber");
    let mut second = session.subscribe_evt_all("second").expect("subscriber");

    session
        .publish_cmd(Command::SetMasterVolume(SetMasterVolume {
            volume: 0.42,
            soft_takeover: false,
        }))
        .expect("the engine owns its inbox");

    let from_first = first
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("an event");
    let from_second = second
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("an event");

    assert!(
        Arc::ptr_eq(&from_first, &from_second),
        "every observer receives the same payload, not a copy"
    );
    assert_eq!(from_first.as_ref(), from_second.as_ref());
    assert!(
        !matches!(from_first.as_ref(), Event::Error(_)),
        "a valid command must not surface as an engine error"
    );
}
