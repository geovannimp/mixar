//! LibrarySession analyze cmd → TrackAnalyzed evt.

#![cfg(feature = "analysis")]

use library::{LibraryConfig, LibrarySession, NewCollection, WritableLibrary};
use library_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use library_core::Library;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

fn write_analysis_wav(path: &Path) {
    let sample_rate = 48_000u32;
    let duration_secs = 3u32;
    let sample_count = (sample_rate * duration_secs) as usize;
    let mut pcm = Vec::with_capacity(sample_count * 2);
    for index in 0..sample_count {
        let time = index as f32 / sample_rate as f32;
        let sample =
            (0.25 * (2.0 * std::f32::consts::PI * 440.0 * time).sin() * i16::MAX as f32) as i16;
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
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

#[test]
fn analyze_track_cmd_emits_track_analyzed_evt() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_analysis_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let tracks = lib.get_collection_tracks(&folder.id).unwrap();
        assert_eq!(tracks.len(), 1);
        tracks[0].id().as_str().to_string()
    };

    let rx = session.subscribe_evt_all().unwrap();
    let body = encode_cmd_body(&CmdBody::AnalyzeTrack {
        track_id: track_id.clone(),
        force: false,
    })
    .unwrap();
    session
        .publish_cmd(Origin::Library, Kind::AnalyzeTrack, body)
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("evt bus alive")
        .expect("TrackAnalyzed evt");
    assert_eq!(event.kind(), &Kind::TrackAnalyzed);
    assert_eq!(event.origin(), &Origin::Track(track_id.clone()));
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::TrackAnalyzed { track } => {
            assert_eq!(track.id, track_id);
            assert!(!track.path.is_empty());
        }
        other => panic!("unexpected evt body: {other:?}"),
    }
}
