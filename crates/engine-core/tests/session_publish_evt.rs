//! Host-style egress via `EngineSession::publish_evt`.

mod common;

use common::recv_evt;
use engine_api::Event;
use engine_api::Notice;
use engine_core::EngineConfig;
use engine_core::EngineSession;

/// The host-enriched `Notice` published with `publish_evt` reaches a subscriber
/// with its payload intact.
#[test]
fn publish_evt_delivers_host_enriched_notice() {
    let session = EngineSession::new(EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    })
    .expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_evt(Event::Notice(Notice {
            message: "host-enriched".into(),
        }))
        .expect("publish_evt");

    let event = recv_evt(&mut evt, |event| matches!(event, Event::Notice(_)));
    let Event::Notice(notice) = event else {
        panic!("expected Notice");
    };
    assert_eq!(notice.message, "host-enriched");
}
