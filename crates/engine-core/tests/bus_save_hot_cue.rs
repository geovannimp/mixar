//! SaveHotCue on engine bus snaps then persists via library evt.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_updated_where;
use common::wait_engine_hot_cue_count;
use common::write_two_second_wav;
use engine_api::Event;
use engine_api::HotCuePadPress;
use engine_api::SaveDeckHotCue;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library::LibraryConfig;
use library::LibrarySession;
use library::NewCollection;
use library::WritableLibrary;
use library_api::decode_evt_body;
use library_api::EvtBody;
use library_api::Kind as LibKind;
use library_core::AudioSource;
use library_core::FileAudioSource;
use library_core::Library;
use library_core::TrackMetadata;
use std::time::Duration;
use std::time::Instant;

fn recv_lib_kind(sub: &library::EvtReceiver, kind: LibKind) -> library::Evt {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = match sub.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(Some(event)) => event,
            // `recv_timeout` returns Ok(None) on timeout; keep waiting for the
            // deadline instead of panicking on the first miss.
            Ok(None) => continue,
            Err(error) => panic!("library evt bus: {error}"),
        };
        if *event.kind() == kind {
            return (*event).clone();
        }
    }
    panic!("timeout waiting for library evt kind {kind:?}");
}

/// Buses attached, but no library worker → SaveHotCue never yields HotCuesChanged.
/// Engine memory must stay empty until that ack (audit #5).
#[test]
fn save_hot_cue_does_not_commit_memory_before_library_ack() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_two_second_wav(&wav);

    let mut library = library::LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = library::LibraryBuses::new();
    library.set_buses(buses.clone());
    let library = std::sync::Arc::new(std::sync::Mutex::new(library));
    // Same cmd bus the engine publishes to; no worker drains it.

    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new_with_library_bus(config, library.clone(), buses.cmd_bus())
        .expect("engine session");
    session.with_engine(|engine| engine.start()).expect("start");

    let track_id = {
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.list_collection_tracks(&folder.id).unwrap();
        tracks[0].id().clone()
    };

    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            engine.seek_deck(0, 620)?;
            engine.save_deck_hot_cue(0, 0)?;
            Ok(())
        })
        .expect("save without worker");

    // Let the control thread tick (subscribed to library evt) with nothing to apply.
    std::thread::sleep(Duration::from_millis(80));
    session
        .with_engine(|engine| {
            let snap = engine.deck_snapshot(0).expect("snapshot");
            assert!(
                snap.hot_cues.is_empty(),
                "engine memory must not commit hot cues before library HotCuesChanged"
            );
            Ok(())
        })
        .expect("still empty after control ticks");
}

#[test]
fn save_hot_cue_snaps_with_quantize_and_emits_library_evt() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_two_second_wav(&wav);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.list_collection_tracks(&folder.id).unwrap();
        assert_eq!(tracks.len(), 1);
        tracks[0].id().clone()
    };

    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new_with_library_bus(
        config,
        library_session.library(),
        library_session.cmd_bus(),
    )
    .expect("engine session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            engine.set_deck_quantize(0, true)?;
            engine.seek_deck(0, 620)?;
            Ok(())
        })
        .expect("load");

    let rx = library_session
        .subscribe_evt_track(track_id.as_str())
        .unwrap();

    session
        .publish_cmd(Event::SaveDeckHotCue(SaveDeckHotCue { deck: 0, slot: 1 }))
        .expect("save");

    let event = recv_lib_kind(&rx, LibKind::HotCuesChanged);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::HotCuesChanged { hot_cues, .. } => {
            assert_eq!(hot_cues.len(), 1);
            assert_eq!(hot_cues[0].slot, 1);
            assert_eq!(hot_cues[0].position_ms, 500);
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn hot_cue_pad_press_saves_then_triggers() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_two_second_wav(&wav);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.list_collection_tracks(&folder.id).unwrap();
        tracks[0].id().clone()
    };

    let config = EngineConfig {
        backend: "null".to_string(),
        ..EngineConfig::default()
    };
    let session = EngineSession::new_with_library_bus(
        config,
        library_session.library(),
        library_session.cmd_bus(),
    )
    .expect("engine session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            engine.set_deck_quantize(0, true)?;
            engine.seek_deck(0, 620)?;
            Ok(())
        })
        .expect("load");

    let rx = library_session
        .subscribe_evt_track(track_id.as_str())
        .unwrap();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::HotCuePadPress(HotCuePadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }))
        .expect("save press");

    let event = recv_lib_kind(&rx, LibKind::HotCuesChanged);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::HotCuesChanged { hot_cues, .. } => {
            assert_eq!(hot_cues[0].slot, 0);
            assert_eq!(hot_cues[0].position_ms, 500);
        }
        other => panic!("unexpected: {other:?}"),
    }

    wait_engine_hot_cue_count(&session, 0, 1);
    session
        .with_engine(|engine| {
            let snap = engine.deck_snapshot(0).expect("snapshot");
            assert_eq!(snap.track_id.as_deref(), Some(track_id.as_str()));
            assert_eq!(snap.hot_cues.len(), 1);
            assert_eq!(snap.hot_cues[0].slot, 0);
            assert_eq!(snap.hot_cues[0].position_ms, 500);
            Ok(())
        })
        .expect("snapshot cues");

    session
        .with_engine(|engine| {
            engine.seek_deck(0, 0)?;
            Ok(())
        })
        .expect("seek 0");
    let drain_until = Instant::now() + Duration::from_secs(2);
    // The control thread ticks every 33 ms, so the quiet window must exceed that.
    // A 20 ms window could break during a normal gap while the seek-to-0 Updated
    // was still in flight, and the read below would then latch that stale
    // snapshot (playing == false, position 0).
    let quiet = Duration::from_millis(150);
    while Instant::now() < drain_until {
        match evt.recv_timeout(quiet) {
            // Quiet window elapsed: nothing else was queued.
            Ok(None) => break,
            Ok(Some(_)) => {}
            // Do not read a disconnected bus as "quiet".
            Err(error) => panic!("evt bus: {error}"),
        }
    }

    session
        .publish_cmd(Event::HotCuePadPress(HotCuePadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }))
        .expect("trigger press");

    // Match on the trigger's effect (the deck starts playing), not on the event
    // kind alone, so a stale pre-trigger `DeckUpdated` cannot satisfy the
    // assertion. `recv_updated_where` owns the deadline/poll/disconnect handling.
    let update = recv_updated_where(&mut evt, |update| update.playing);
    assert_eq!(update.position_ms, Some(500));
    assert!(update.playing);

    session
        .publish_cmd(Event::HotCuePadPress(HotCuePadPress {
            deck: 0,
            slot: 0,
            shift: true,
        }))
        .expect("delete press");

    let event = recv_lib_kind(&rx, LibKind::HotCuesChanged);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::HotCuesChanged { hot_cues, .. } => {
            assert!(hot_cues.is_empty());
        }
        other => panic!("unexpected: {other:?}"),
    }
    wait_engine_hot_cue_count(&session, 0, 0);
}

#[test]
fn hot_cue_pad_press_triggers_hydrated_cues_after_reload() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_two_second_wav(&wav);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.list_collection_tracks(&folder.id).unwrap();
        tracks[0].id().clone()
    };

    let config = EngineConfig {
        backend: "null".to_string(),
        ..EngineConfig::default()
    };
    let session = EngineSession::new_with_library_bus(
        config.clone(),
        library_session.library(),
        library_session.cmd_bus(),
    )
    .expect("engine session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            engine.set_deck_quantize(0, true)?;
            engine.seek_deck(0, 620)?;
            Ok(())
        })
        .expect("load");

    let rx = library_session
        .subscribe_evt_track(track_id.as_str())
        .unwrap();
    session
        .publish_cmd(Event::SaveDeckHotCue(SaveDeckHotCue { deck: 0, slot: 0 }))
        .expect("save");
    let _ = recv_lib_kind(&rx, LibKind::HotCuesChanged);
    drop(session);

    let session = EngineSession::new_with_library_bus(
        config,
        library_session.library(),
        library_session.cmd_bus(),
    )
    .expect("reload session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            engine.seek_deck(0, 0)?;
            Ok(())
        })
        .expect("reload");

    session
        .with_engine(|engine| {
            let snap = engine.deck_snapshot(0).expect("snapshot");
            assert_eq!(snap.hot_cues.len(), 1);
            assert_eq!(snap.hot_cues[0].position_ms, 500);
            Ok(())
        })
        .expect("hydrated");

    let mut evt = session.subscribe_evt_all().expect("sub");
    session
        .publish_cmd(Event::HotCuePadPress(HotCuePadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }))
        .expect("trigger");

    // Match on the trigger's effect (the deck starts playing), not on the event
    // kind alone, so a stale pre-trigger `DeckUpdated` (playing == false,
    // position 0) buffered on the bus cannot satisfy the assertions below — the
    // same hazard the twin test above documents.
    let update = recv_updated_where(&mut evt, |update| update.playing);
    assert_eq!(update.position_ms, Some(500));
    assert_eq!(update.hot_cues.len(), 1);
    assert_eq!(update.hot_cues[0].position_ms, 500);
}
