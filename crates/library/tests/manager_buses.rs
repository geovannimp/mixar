//! LibraryManager + LibraryBuses without LibrarySession.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use bus_api::Event;
use bus_api::Navigate;
use bus_api::RefreshTrack;
use library::{spawn_library_worker, LibraryBuses, LibraryConfig, LibraryManager};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn manager_publish_evt_without_session() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = LibraryBuses::new();
    lib.set_buses(buses.clone());
    let mut rx = lib.subscribe_evt().unwrap();
    lib.publish_evt(Event::Navigate(Navigate { delta: 1 }))
        .unwrap();
    let ev = rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap()
        .expect("Navigate evt");
    assert_eq!(ev, Event::Navigate(Navigate { delta: 1 }));
}

#[test]
fn spawn_worker_refresh_missing_track_emits_error() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = LibraryBuses::new();
    lib.set_buses(buses.clone());
    let library = Arc::new(Mutex::new(lib));
    let _worker = spawn_library_worker(Arc::clone(&library)).unwrap();

    let mut rx = buses.subscribe_evt().unwrap();
    buses
        .publish_cmd(Event::RefreshTrack(RefreshTrack {
            track_id: "missing-track-id".into(),
        }))
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("evt bus alive")
        .expect("Error evt");
    match event {
        Event::Error(error) => assert!(
            error.message.contains("missing-track-id"),
            "unexpected message: {}",
            error.message
        ),
        other => panic!("unexpected evt: {other:?}"),
    }
}
