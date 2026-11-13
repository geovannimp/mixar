//! LibrarySession deck performance cmds → per-track change evts.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use bus_api::Event;
use bus_api::SaveBeatGrid;
use bus_api::SaveTrackHotCue;
use bus_api::SaveTrackLoop;
use bus_api::AUTO_LOOP_SLOT;
use library::{LibraryConfig, LibrarySession, NewCollection, WritableLibrary};
use library_core::Library;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

fn write_tiny_wav(path: &Path) {
    let sample_rate = 48_000u32;
    let sample_count = sample_rate as usize; // 1s
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

fn import_one_track(session: &LibrarySession, dir: &Path) -> String {
    let library = session.library();
    let mut lib = library.lock().unwrap();
    let folder = lib.add_collection(&NewCollection::folder(dir)).unwrap();
    lib.sync_collection(Some(&folder.id)).unwrap();
    let tracks = lib.list_collection_tracks(&folder.id).unwrap();
    assert_eq!(tracks.len(), 1);
    tracks[0].id().as_str().to_string()
}

#[test]
fn save_hot_cue_cmd_emits_hot_cues_changed_for_track() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_one_track(&session, dir.path());

    let mut rx = session.subscribe_evt().unwrap();
    session
        .publish_cmd(Event::SaveTrackHotCue(SaveTrackHotCue {
            track_id: track_id.clone(),
            slot: 3,
            position_ms: 12_500,
            loop_length_beats: None,
            color: None,
            label: None,
        }))
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("HotCuesChanged evt");
    match event {
        Event::HotCuesChanged(changed) => {
            assert_eq!(changed.track_id, track_id);
            assert_eq!(changed.hot_cues.len(), 1);
            assert_eq!(changed.hot_cues[0].slot, 3);
            assert_eq!(changed.hot_cues[0].position_ms, 12_500);
        }
        other => panic!("unexpected evt: {other:?}"),
    }
}

#[test]
fn save_loop_cmd_emits_loops_changed_for_track() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_one_track(&session, dir.path());

    let mut rx = session.subscribe_evt().unwrap();
    session
        .publish_cmd(Event::SaveTrackLoop(SaveTrackLoop {
            track_id: track_id.clone(),
            slot: 1,
            in_ms: 1000,
            out_ms: 5000,
            label: None,
            color: None,
        }))
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("LoopsChanged evt");
    match event {
        Event::LoopsChanged(changed) => {
            assert_eq!(changed.track_id, track_id);
            assert_eq!(changed.loops.len(), 1);
            assert_eq!(changed.loops[0].slot, 1);
            assert_eq!(changed.loops[0].in_ms, 1000);
            assert_eq!(changed.loops[0].out_ms, 5000);
        }
        other => panic!("unexpected evt: {other:?}"),
    }
}

#[test]
fn save_beat_grid_cmd_emits_beat_grid_changed_for_track() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_one_track(&session, dir.path());

    let mut rx = session.subscribe_evt().unwrap();
    session
        .publish_cmd(Event::SaveBeatGrid(SaveBeatGrid {
            track_id: track_id.clone(),
            bpm: 128.0,
            first_beat_secs: 0.5,
            replace: false,
        }))
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("BeatGridChanged evt");
    match event {
        Event::BeatGridChanged(changed) => {
            assert_eq!(changed.track_id, track_id);
            let segment = changed.beat_grid.segments.first().expect("segment");
            assert_eq!(segment.bpm, 128.0);
            assert_eq!(segment.start_time_ms, 500);
        }
        other => panic!("unexpected evt: {other:?}"),
    }
}

#[test]
fn auto_loop_cmd_allocates_and_emits_loops_changed() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_one_track(&session, dir.path());

    let mut rx = session.subscribe_evt().unwrap();
    session
        .publish_cmd(Event::SaveTrackLoop(SaveTrackLoop {
            track_id: track_id.clone(),
            slot: AUTO_LOOP_SLOT,
            in_ms: 1000,
            out_ms: 3000,
            label: None,
            color: None,
        }))
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("LoopsChanged evt");
    match event {
        Event::LoopsChanged(changed) => {
            assert_eq!(changed.track_id, track_id);
            assert_eq!(changed.loops.len(), 1);
            assert_eq!(changed.loops[0].slot, 0);
            assert_eq!(changed.loops[0].in_ms, 1000);
            assert_eq!(changed.loops[0].out_ms, 3000);
        }
        other => panic!("unexpected evt: {other:?}"),
    }
}

#[test]
fn auto_loop_cmd_updates_overlapping_auto_loop() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_one_track(&session, dir.path());
    let mut rx = session.subscribe_evt().unwrap();

    for out_ms in [3000, 5000] {
        session
            .publish_cmd(Event::SaveTrackLoop(SaveTrackLoop {
                track_id: track_id.clone(),
                slot: AUTO_LOOP_SLOT,
                in_ms: 1000,
                out_ms,
                label: None,
                color: None,
            }))
            .unwrap();
        let event = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("evt bus alive")
            .expect("LoopsChanged evt");
        match event {
            Event::LoopsChanged(changed) => {
                assert_eq!(changed.loops.len(), 1);
                assert_eq!(changed.loops[0].slot, 0);
                assert_eq!(changed.loops[0].out_ms, out_ms);
            }
            other => panic!("unexpected evt: {other:?}"),
        }
    }
}

#[test]
fn auto_loop_cmd_does_not_touch_imported_loop() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_one_track(&session, dir.path());
    {
        let library = session.library();
        let library = library.lock().unwrap();
        library
            .save_track_loop(
                &library_core::TrackId::new(track_id.clone()),
                0,
                1000,
                3000,
                Some("Imported".into()),
                None,
            )
            .unwrap();
    }

    let mut rx = session.subscribe_evt().unwrap();
    session
        .publish_cmd(Event::SaveTrackLoop(SaveTrackLoop {
            track_id: track_id.clone(),
            slot: AUTO_LOOP_SLOT,
            in_ms: 1000,
            out_ms: 3000,
            label: None,
            color: None,
        }))
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("LoopsChanged evt");
    match event {
        Event::LoopsChanged(changed) => {
            assert_eq!(changed.loops.len(), 2);
            let imported = changed.loops.iter().find(|loop_| loop_.slot == 0).unwrap();
            assert_eq!(imported.label.as_deref(), Some("Imported"));
            let auto = changed.loops.iter().find(|loop_| loop_.slot == 1).unwrap();
            assert_eq!((auto.in_ms, auto.out_ms), (1000, 3000));
        }
        other => panic!("unexpected evt: {other:?}"),
    }
}
