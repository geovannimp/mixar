//! LibraryTransport parity: add folder, resolve paths, artwork, analyze/refresh bus.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use host_flutter::api::library::{LibraryTransport, RenderWaveformLaneRequest};
use library_api::{decode_evt_body, EvtBody, Kind};

fn write_minimal_wav(path: &Path) {
    let sample_rate = 8_000u32;
    let sample_count = sample_rate as usize; // 1s mono
    let pcm = vec![0u8; sample_count * 2];
    let data_size = pcm.len() as u32;
    let file_size = 36 + data_size;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&(sample_rate * 2).to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&pcm).unwrap();
}

#[test]
fn add_folder_resolve_and_artwork() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track_a.wav");
    write_minimal_wav(&wav);

    let transport = LibraryTransport::open_in_memory().unwrap();
    let added = transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned())
        .unwrap();
    assert_eq!(added.added, 1);
    assert_eq!(added.collection.track_count, 1);

    let collections = transport.list_collections().unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].track_count, 1);

    let tracks = transport
        .list_collection_tracks(collections[0].id.clone())
        .unwrap();
    assert_eq!(tracks.len(), 1);
    assert!(tracks[0].path.ends_with("track_a.wav"));

    let resolved = transport
        .resolve_tracks_for_paths(vec![wav.to_string_lossy().into_owned()])
        .unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].track.id, tracks[0].id);
    assert!(resolved[0].request_path.ends_with("track_a.wav"));

    let artwork = transport
        .get_track_artwork(Some(tracks[0].id.clone()), None)
        .unwrap();
    assert!(artwork.is_none() || artwork.as_ref().is_some_and(|b| !b.is_empty()));

    let artwork_by_path = transport
        .get_track_artwork(None, Some(wav.to_string_lossy().into_owned()))
        .unwrap();
    assert!(artwork_by_path.is_none() || artwork_by_path.as_ref().is_some_and(|b| !b.is_empty()));
}

#[test]
fn refresh_missing_track_publishes_error_evt() {
    let transport = LibraryTransport::open_in_memory().unwrap();
    let rx = transport.subscribe_evt_all().unwrap();

    transport.refresh_track("missing-track-id".into()).unwrap();
    transport
        .analyze_track("missing-track-id".into(), false)
        .unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("evt bus alive")
        .expect("Error evt");
    assert_eq!(event.kind(), &Kind::Error);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::Error { message, track_id } => {
            assert!(!message.is_empty());
            assert_eq!(track_id.as_deref(), Some("missing-track-id"));
        }
        other => panic!("unexpected body {other:?}"),
    }
}

#[test]
fn refresh_existing_track_emits_track_updated() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track_b.wav");
    write_minimal_wav(&wav);

    let transport = LibraryTransport::open_in_memory().unwrap();
    let added = transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned())
        .unwrap();
    let tracks = transport
        .list_collection_tracks(added.collection.id.clone())
        .unwrap();
    assert_eq!(tracks.len(), 1);

    let rx = transport.subscribe_evt_all().unwrap();
    transport.refresh_track(tracks[0].id.clone()).unwrap();

    let event = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("evt bus alive")
        .expect("TrackUpdated evt");
    assert_eq!(event.kind(), &Kind::TrackUpdated);
    match decode_evt_body(event.payload()).unwrap() {
        EvtBody::TrackUpdated { track } => {
            assert_eq!(track.id, tracks[0].id);
            assert!(track.path.ends_with("track_b.wav"));
        }
        other => panic!("unexpected body {other:?}"),
    }
}

#[test]
fn render_waveform_lane_requires_path_or_track_id() {
    let transport = LibraryTransport::open_in_memory().unwrap();
    let err = transport
        .render_waveform_lane(RenderWaveformLaneRequest {
            track_id: None,
            path: None,
            width: 64,
            height: 32,
            position_ms: 0,
            visible_ms: 1_000,
            buffer_ratio: 0.0,
            include_detail: false,
            include_beat_grid: false,
            eq_low_db: 0.0,
            eq_mid_db: 0.0,
            eq_high_db: 0.0,
        })
        .unwrap_err();
    assert!(err.contains("path or track_id"));
}

#[test]
fn render_waveform_lane_returns_wfr1_packed_frame() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track_w.wav");
    write_minimal_wav(&wav);

    let transport = LibraryTransport::open_in_memory().unwrap();
    let added = transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned())
        .unwrap();
    let tracks = transport
        .list_collection_tracks(added.collection.id.clone())
        .unwrap();
    assert_eq!(tracks.len(), 1);

    let width = 64u32;
    let height = 32u32;
    let packed = transport
        .render_waveform_lane(RenderWaveformLaneRequest {
            track_id: Some(tracks[0].id.clone()),
            path: None,
            width,
            height,
            position_ms: 0,
            visible_ms: 1_000,
            buffer_ratio: 0.0,
            include_detail: false,
            include_beat_grid: false,
            eq_low_db: 0.0,
            eq_mid_db: 0.0,
            eq_high_db: 0.0,
        })
        .unwrap();

    assert!(packed.starts_with(b"WFR1"));
    assert!(packed.len() >= 28);
    assert_eq!(u32::from_le_bytes(packed[4..8].try_into().unwrap()), width);
    assert_eq!(
        u32::from_le_bytes(packed[8..12].try_into().unwrap()),
        height
    );
}
