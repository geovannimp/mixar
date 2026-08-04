//! LibrarySession UI navigation evt → subscribers (no worker cmd).

use library::{LibraryConfig, LibrarySession};
use library_api::{EvtBody, Kind, Origin};
use std::time::Duration;

#[test]
fn publish_ui_nav_evt_reaches_subscriber() {
    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let rx = session.subscribe_evt_all().unwrap();
    session
        .publish_evt(
            Origin::LibraryNavigation,
            Kind::NavigateNext,
            EvtBody::Empty,
        )
        .unwrap();
    let event = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("evt bus alive")
        .expect("NavigateNext evt");
    assert_eq!(event.origin(), &Origin::LibraryNavigation);
    assert_eq!(event.kind(), &Kind::NavigateNext);
}
