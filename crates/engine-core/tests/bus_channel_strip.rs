//! Integration: channel-strip cmds (filter / gain / headphone cue) → DeckUpdated.

mod common;

use common::recv_evt_kind;
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use omnibus::Filter;

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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    let body = encode_cmd_body(&CmdBody::SetFilter {
        filter: 0.75,
        soft_takeover: false,
    })
    .unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::SetFilter, body)
        .expect("publish");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { filter, .. } =
        decode_evt_body(event.payload()).expect("decode evt body")
    else {
        panic!("expected DeckUpdated");
    };
    assert!((filter - 0.75).abs() < 0.01);
}

#[test]
fn set_gain_trim_and_headphone_cue_roundtrip() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetGainTrim,
            encode_cmd_body(&CmdBody::SetGainTrim {
                gain_trim: 0.6,
                soft_takeover: false,
            })
            .unwrap(),
        )
        .expect("gain");
    let gain_evt = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { gain_trim, .. } =
        decode_evt_body(gain_evt.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!((gain_trim - 0.6).abs() < 0.01);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetHeadphoneCue,
            encode_cmd_body(&CmdBody::SetHeadphoneCue { enabled: true }).unwrap(),
        )
        .expect("cue");
    let cue_evt = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { headphone_cue, .. } =
        decode_evt_body(cue_evt.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(headphone_cue);
}
