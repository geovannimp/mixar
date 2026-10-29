//! Integration: channel-strip cmds (filter / gain / headphone cue) → DeckUpdated.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_updated;
use engine_api::Event;
use engine_api::SetFilter;
use engine_api::SetGainTrim;
use engine_api::SetHeadphoneCue;
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

#[test]
fn set_filter_publishes_updated_with_filter_norm() {
    let session = null_session();
    let mut evt = session.subscribe_evt_all().expect("sub");
    session
        .publish_cmd(Event::SetFilter(SetFilter {
            deck: 0,
            filter: 0.75,
            soft_takeover: false,
        }))
        .expect("publish");
    let update = recv_updated(&mut evt);
    assert!((update.filter - 0.75).abs() < 0.01);
}

#[test]
fn set_gain_trim_and_headphone_cue_roundtrip() {
    let session = null_session();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetGainTrim(SetGainTrim {
            deck: 0,
            gain_trim: 0.6,
            soft_takeover: false,
        }))
        .expect("gain");
    let gain_update = recv_updated(&mut evt);
    assert!((gain_update.gain_trim - 0.6).abs() < 0.01);

    session
        .publish_cmd(Event::SetHeadphoneCue(SetHeadphoneCue {
            deck: 0,
            enabled: true,
        }))
        .expect("cue");
    let cue_update = recv_updated(&mut evt);
    assert!(cue_update.headphone_cue);
}
