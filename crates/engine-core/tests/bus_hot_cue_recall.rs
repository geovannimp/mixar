//! Integration: trigger hot cue + recall saved loop on the bus.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::RecallSavedLoop;
use bus_api::TriggerHotCue;
use common::recv_updated;
use common::source_with_bpm;
use engine_core::EngineConfig;
use engine_core::EngineSession;

fn null_session_loaded() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(0, source_with_bpm("hotcue.wav", 120.0))?;
            Ok(())
        })
        .expect("load");
    session
}

#[test]
fn trigger_hot_cue_seeks_and_plays() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::TriggerHotCue(TriggerHotCue {
            deck: 0,
            position_ms: 500,
        }))
        .expect("trigger");

    let update = recv_updated(&mut evt);
    assert!(update.playing);
    let pos = update.position_ms.expect("position");
    assert!((pos - 500).abs() <= 1);
}

#[test]
fn recall_saved_loop_activates_and_plays() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::RecallSavedLoop(RecallSavedLoop {
            deck: 0,
            in_ms: 0,
            out_ms: 1000,
        }))
        .expect("recall");

    let update = recv_updated(&mut evt);
    assert!(update.playing);
    let region = update.active_loop.expect("loop");
    assert_eq!(region.in_ms, 0);
    assert_eq!(region.out_ms, 1000);
    assert!(region.active);
}
