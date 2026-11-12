//! Integration: loop activation persists through the library bus.
//!
//! The end-to-end assertions live here (engine + library worker + SQLite) so the
//! library crate does not need a dev-dependency back on engine-core.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers

mod common;

use bus_api::BeginLoopRoll;
use bus_api::Event;
use bus_api::ExitLoop;
use bus_api::LoopIn;
use bus_api::LoopOut;
use bus_api::LoopsChanged;
use bus_api::SetAutoLoop;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use event_bus::Subscriber;
use library::LibraryConfig;
use library::LibrarySession;
use library::NewCollection;
use library::WritableLibrary;
use library_core::AudioSource;
use library_core::FileAudioSource;
use library_core::Library;
use library_core::TrackId;
use library_core::TrackMetadata;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

fn import_track(session: &LibrarySession, dir: &Path) -> String {
    let library = session.library();
    let mut lib = library.lock().unwrap();
    let folder = lib.add_collection(&NewCollection::folder(dir)).unwrap();
    lib.sync_collection(Some(&folder.id)).unwrap();
    let tracks = lib.list_collection_tracks(&folder.id).unwrap();
    assert_eq!(tracks.len(), 1);
    tracks[0].id().as_str().to_string()
}

fn recv_loops_within(rx: &mut Subscriber<Event>, timeout: Duration) -> Option<LoopsChanged> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match rx.recv_timeout(remaining) {
            Ok(Some(Event::LoopsChanged(changed))) => return Some(changed),
            Ok(Some(_)) => {}
            Ok(None) => return None,
            Err(error) => panic!("evt bus disconnected: {error}"),
        }
    }
}

fn recv_loops(rx: &mut Subscriber<Event>) -> LoopsChanged {
    recv_loops_within(rx, Duration::from_secs(5)).expect("LoopsChanged within deadline")
}

fn engine_with_track() -> (EngineSession, LibrarySession, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    common::write_two_second_wav(&wav);

    let library = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_track(&library, dir.path());
    let source = AudioSource::File(FileAudioSource::new(
        TrackId::new(track_id.clone()),
        wav.clone(),
        TrackMetadata {
            bpm: Some(120.0),
            ..TrackMetadata::default()
        },
    ));
    let engine = EngineSession::new_with_library_bus(
        EngineConfig {
            backend: "null".to_string(),
            ..EngineConfig::default()
        },
        library.library(),
        library.bus(),
    )
    .unwrap();
    engine
        .with_engine(|engine| {
            engine.start()?;
            engine.load_track(0, source)?;
            Ok(())
        })
        .unwrap();
    (engine, library, track_id, dir)
}

#[test]
fn auto_loop_activation_persists_a_loop() {
    let (engine, library, _track_id, _dir) = engine_with_track();
    let mut rx = library.subscribe_evt().unwrap();

    engine
        .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: 0,
            beats: 2.0,
        }))
        .unwrap();

    let changed = recv_loops(&mut rx);
    assert_eq!(changed.loops.len(), 1);
    assert_eq!(changed.loops[0].slot, 0);
    assert_eq!((changed.loops[0].in_ms, changed.loops[0].out_ms), (0, 1000));
}

#[test]
fn auto_loop_resize_updates_the_same_loop() {
    let (engine, library, _track_id, _dir) = engine_with_track();
    let mut rx = library.subscribe_evt().unwrap();

    engine
        .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: 0,
            beats: 1.0,
        }))
        .unwrap();
    let _ = recv_loops(&mut rx);

    engine
        .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: 0,
            beats: 2.0,
        }))
        .unwrap();
    let changed = recv_loops(&mut rx);
    assert_eq!(changed.loops.len(), 1);
    assert_eq!((changed.loops[0].in_ms, changed.loops[0].out_ms), (0, 1000));
}

#[test]
fn manual_loop_is_saved_but_roll_and_exit_are_not() {
    let (engine, library, _track_id, _dir) = engine_with_track();
    let mut rx = library.subscribe_evt().unwrap();

    engine
        .publish_cmd(Event::LoopIn(LoopIn {
            deck: 0,
            position_ms: 0,
        }))
        .unwrap();
    engine
        .publish_cmd(Event::LoopOut(LoopOut {
            deck: 0,
            position_ms: 1000,
        }))
        .unwrap();
    let changed = recv_loops(&mut rx);
    assert_eq!(changed.loops.len(), 1);
    assert_eq!((changed.loops[0].in_ms, changed.loops[0].out_ms), (0, 1000));

    engine
        .publish_cmd(Event::BeginLoopRoll(BeginLoopRoll {
            deck: 0,
            beats: 1.0,
        }))
        .unwrap();
    assert!(
        recv_loops_within(&mut rx, Duration::from_millis(200)).is_none(),
        "loop roll must not persist a loop"
    );

    engine
        .publish_cmd(Event::ExitLoop(ExitLoop { deck: 0 }))
        .unwrap();
    assert!(
        recv_loops_within(&mut rx, Duration::from_millis(200)).is_none(),
        "exit must not persist a loop"
    );
}
