//! Integration test: cmd bus Play → evt bus response.
//!
//! Task 2 stub: control thread publishes `Kind::Error` with `"no handler"`.
//! Task 3 replaces the stub with real play/pause dispatch.

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use omnibus::Filter;
use std::time::Duration;

#[test]
fn play_on_empty_deck_publishes_no_handler_error() {
    let session = EngineSession::new(EngineConfig::default()).expect("session");
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::Play, body)
        .expect("publish");
    let event = evt
        .recv_timeout(Duration::from_secs(1))
        .expect("recv")
        .expect("event");
    assert_eq!(*event.origin(), Origin::Deck(0));
    assert_eq!(*event.kind(), Kind::Error);
    let evt_body = decode_evt_body(event.payload()).expect("decode evt body");
    assert_eq!(
        evt_body,
        EvtBody::Error {
            message: "no handler".into(),
        }
    );
}
