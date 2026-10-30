//! LibrarySession UI navigation evt → subscribers (no worker cmd).

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use bus_api::Event;
use bus_api::Navigate;
use library::{LibraryConfig, LibrarySession};
use std::time::Duration;

#[test]
fn publish_ui_nav_evt_reaches_subscriber() {
    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let mut rx = session.subscribe_evt().unwrap();
    session
        .publish_evt(Event::Navigate(Navigate { delta: 1 }))
        .unwrap();
    let event = rx
        .recv_timeout(Duration::from_secs(1))
        .expect("evt bus alive")
        .expect("Navigate evt");
    assert_eq!(event, Event::Navigate(Navigate { delta: 1 }));
}
