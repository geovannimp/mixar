//! Integration: trigger hot cue + recall saved loop on the bus.

mod common;

use common::{recv_evt_kind, source_with_bpm};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use omnibus::Filter;

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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::TriggerHotCue,
            encode_cmd_body(&CmdBody::TriggerHotCue { position_ms: 500 }).unwrap(),
        )
        .expect("trigger");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        position_ms,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(playing);
    let pos = position_ms.expect("position");
    assert!((pos - 500).abs() <= 1);
}

#[test]
fn recall_saved_loop_activates_and_plays() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::RecallSavedLoop,
            encode_cmd_body(&CmdBody::RecallSavedLoop {
                in_ms: 0,
                out_ms: 1000,
            })
            .unwrap(),
        )
        .expect("recall");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        active_loop,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(playing);
    let region = active_loop.expect("loop");
    assert_eq!(region.in_ms, 0);
    assert_eq!(region.out_ms, 1000);
    assert!(region.active);
}
