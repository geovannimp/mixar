//! LibrarySession deck performance cmds → per-track change evts.

use library::{LibraryConfig, LibrarySession, NewCollection, WritableLibrary};
use library_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
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
    let tracks = lib.get_collection_tracks(&folder.id).unwrap();
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

    let rx = session.subscribe_evt_track(track_id.clone()).unwrap();
    let body = encode_cmd_body(&CmdBody::SaveHotCue {
        track_id: track_id.clone(),
        slot: 3,
        position_ms: 12_500,
        loop_length_beats: None,
        color: None,
        label: None,
    })
    .unwrap();
    session
        .publish_cmd(Origin::Library, Kind::SaveHotCue, body)
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("HotCuesChanged evt");
    assert_eq!(event.kind(), &Kind::HotCuesChanged);
    assert_eq!(event.origin(), &Origin::Track(track_id.clone()));
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::HotCuesChanged {
            track_id: tid,
            hot_cues,
        } => {
            assert_eq!(tid, track_id);
            assert_eq!(hot_cues.len(), 1);
            assert_eq!(hot_cues[0].slot, 3);
            assert_eq!(hot_cues[0].position_ms, 12_500);
        }
        other => panic!("unexpected evt body: {other:?}"),
    }
}

#[test]
fn save_loop_cmd_emits_loops_changed_for_track() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_tiny_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = import_one_track(&session, dir.path());

    let rx = session.subscribe_evt_track(track_id.clone()).unwrap();
    let body = encode_cmd_body(&CmdBody::SaveLoop {
        track_id: track_id.clone(),
        slot: 1,
        in_ms: 1000,
        out_ms: 5000,
        label: None,
        color: None,
    })
    .unwrap();
    session
        .publish_cmd(Origin::Library, Kind::SaveLoop, body)
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("evt bus alive")
        .expect("LoopsChanged evt");
    assert_eq!(event.kind(), &Kind::LoopsChanged);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::LoopsChanged {
            track_id: tid,
            loops,
        } => {
            assert_eq!(tid, track_id);
            assert_eq!(loops.len(), 1);
            assert_eq!(loops[0].slot, 1);
            assert_eq!(loops[0].in_ms, 1000);
            assert_eq!(loops[0].out_ms, 5000);
        }
        other => panic!("unexpected evt body: {other:?}"),
    }
}
