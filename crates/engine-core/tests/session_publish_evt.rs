//! Host-style egress via `EngineSession::publish_evt`.

mod common;

use common::recv_evt_kind;
use engine_api::{decode_evt_body, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use omnibus::Filter;

#[test]
fn publish_evt_delivers_body_and_bumps_revision() {
    let session = EngineSession::new(EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    })
    .expect("session");
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    assert_eq!(session.revision(), 0);

    session
        .publish_evt(
            Origin::Mixer,
            Kind::Notice,
            EvtBody::Notice {
                message: "host-enriched".into(),
            },
        )
        .expect("publish_evt");

    let event = recv_evt_kind(&evt, Kind::Notice);
    assert_eq!(*event.origin(), Origin::Mixer);
    let EvtBody::Notice { message } = decode_evt_body(event.payload()).expect("decode") else {
        panic!("expected Notice body");
    };
    assert_eq!(message, "host-enriched");
    assert_eq!(session.revision(), 1);
}
