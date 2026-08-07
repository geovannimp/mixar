//! Integration: channel-strip cmds (filter / gain / headphone cue) → DeckUpdated.

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use omnibus::Filter;
use std::time::{Duration, Instant};

fn recv_evt_kind(
    sub: &omnibus::BusReceiver<Origin, Kind, std::sync::Arc<[u8]>>,
    kind: Kind,
) -> omnibus::Event<Origin, Kind, std::sync::Arc<[u8]>> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = sub
            .recv_timeout(remaining.min(Duration::from_millis(50)))
            .expect("recv")
            .expect("event");
        if *event.kind() == kind {
            return (*event).clone();
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

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
fn set_filter_publishes_updated_with_filter_db() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    let body = encode_cmd_body(&CmdBody::SetFilter { filter_db: 4.5, soft_takeover: false }).unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::SetFilter, body)
        .expect("publish");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { filter_db, .. } =
        decode_evt_body(event.payload()).expect("decode evt body")
    else {
        panic!("expected DeckUpdated");
    };
    assert!((filter_db - 4.5).abs() < 0.01);
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
            encode_cmd_body(&CmdBody::SetGainTrim { gain_db: 1.5, soft_takeover: false }).unwrap(),
        )
        .expect("gain");
    let gain_evt = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { gain_trim_db, .. } =
        decode_evt_body(gain_evt.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!((gain_trim_db - 1.5).abs() < f32::EPSILON);

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
