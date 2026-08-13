//! LibraryManager + LibraryBuses without LibrarySession.

use library::{spawn_library_worker, LibraryBuses, LibraryConfig, LibraryManager};
use library_api::{encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test]
fn manager_publish_evt_without_session() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = LibraryBuses::new();
    lib.set_buses(buses.clone());
    let rx = lib.subscribe_evt_all().unwrap();
    lib.publish_evt(
        Origin::LibraryNavigation,
        Kind::Navigate,
        EvtBody::Navigate { delta: 1 },
    )
    .unwrap();
    let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
    assert_eq!(ev.kind(), &Kind::Navigate);
}

#[test]
fn spawn_worker_refresh_missing_track_emits_error() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = LibraryBuses::new();
    lib.set_buses(buses.clone());
    let library = Arc::new(Mutex::new(lib));
    let _worker = spawn_library_worker(Arc::clone(&library)).unwrap();

    let rx = buses.subscribe_evt_all().unwrap();
    let body = encode_cmd_body(&CmdBody::RefreshTrack {
        track_id: "missing-track-id".into(),
    })
    .unwrap();
    buses
        .publish_cmd(Origin::Library, Kind::RefreshTrack, body)
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("evt bus alive")
        .expect("Error evt");
    assert_eq!(event.kind(), &Kind::Error);
}
