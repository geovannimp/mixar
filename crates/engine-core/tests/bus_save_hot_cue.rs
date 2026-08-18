//! SaveHotCue on engine bus snaps then persists via library evt.

use engine_api::{
    decode_evt_body as decode_engine_evt, encode_cmd_body, CmdBody, EvtBody as EngineEvtBody, Kind,
    Origin,
};
use engine_core::{EngineConfig, EngineSession};
use library::{LibraryConfig, LibrarySession, NewCollection, WritableLibrary};
use library_api::{decode_evt_body, EvtBody, Kind as LibKind};
use library_core::{AudioSource, FileAudioSource, Library, TrackMetadata};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

fn write_tiny_wav(path: &Path) {
    let sample_rate = 48_000u32;
    let sample_count = (sample_rate * 2) as usize;
    let pcm = vec![0u8; sample_count * 2];
    let data_size = pcm.len() as u32;
    let file_size = 36 + data_size;
    let byte_rate = sample_rate * 2;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&pcm).unwrap();
}

fn recv_lib_kind(sub: &library::EvtReceiver, kind: LibKind) -> library::Evt {
    let deadline = Instant::now() + Duration::from_secs(5);
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
    panic!("timeout waiting for library evt kind {kind:?}");
}

#[test]
fn save_hot_cue_snaps_with_quantize_and_emits_library_evt() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.get_collection_tracks(&folder.id).unwrap();
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
        .publish_cmd(
            Origin::Deck(0),
            Kind::SaveHotCue,
            encode_cmd_body(&CmdBody::SaveHotCue { slot: 1 }).unwrap(),
        )
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
    write_tiny_wav(&wav);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.get_collection_tracks(&folder.id).unwrap();
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
    let evt = session
        .evt_bus()
        .subscribe(omnibus::Filter::Any, omnibus::Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::HotCuePadPress,
            encode_cmd_body(&CmdBody::HotCuePadPress {
                slot: 0,
                shift: false,
            })
            .unwrap(),
        )
        .expect("save press");

    let event = recv_lib_kind(&rx, LibKind::HotCuesChanged);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::HotCuesChanged { hot_cues, .. } => {
            assert_eq!(hot_cues[0].slot, 0);
            assert_eq!(hot_cues[0].position_ms, 500);
        }
        other => panic!("unexpected: {other:?}"),
    }

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
    let drain_until = Instant::now() + Duration::from_millis(500);
    while Instant::now() < drain_until {
        if evt
            .recv_timeout(Duration::from_millis(20))
            .ok()
            .flatten()
            .is_none()
        {
            break;
        }
    }

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::HotCuePadPress,
            encode_cmd_body(&CmdBody::HotCuePadPress {
                slot: 0,
                shift: false,
            })
            .unwrap(),
        )
        .expect("trigger press");

    let event = loop {
        let event = evt
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("recv")
            .expect("event");
        if *event.kind() == Kind::Updated {
            break event;
        }
    };
    let EngineEvtBody::DeckUpdated {
        position_ms,
        playing,
        ..
    } = decode_engine_evt(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(position_ms, Some(500));
    assert!(playing);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::HotCuePadPress,
            encode_cmd_body(&CmdBody::HotCuePadPress {
                slot: 0,
                shift: true,
            })
            .unwrap(),
        )
        .expect("delete press");

    let event = recv_lib_kind(&rx, LibKind::HotCuesChanged);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::HotCuesChanged { hot_cues, .. } => {
            assert!(hot_cues.is_empty());
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn hot_cue_pad_press_triggers_hydrated_cues_after_reload() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.get_collection_tracks(&folder.id).unwrap();
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
        .publish_cmd(
            Origin::Deck(0),
            Kind::SaveHotCue,
            encode_cmd_body(&CmdBody::SaveHotCue { slot: 0 }).unwrap(),
        )
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

    let evt = session
        .evt_bus()
        .subscribe(omnibus::Filter::Any, omnibus::Filter::Any)
        .expect("sub");
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::HotCuePadPress,
            encode_cmd_body(&CmdBody::HotCuePadPress {
                slot: 0,
                shift: false,
            })
            .unwrap(),
        )
        .expect("trigger");

    let event = loop {
        let event = evt
            .recv_timeout(Duration::from_secs(2))
            .expect("recv")
            .expect("event");
        if *event.kind() == Kind::Updated {
            break event;
        }
    };
    let EngineEvtBody::DeckUpdated {
        position_ms,
        playing,
        hot_cues,
        ..
    } = decode_engine_evt(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(position_ms, Some(500));
    assert!(playing);
    assert_eq!(hot_cues.len(), 1);
    assert_eq!(hot_cues[0].position_ms, 500);
}
