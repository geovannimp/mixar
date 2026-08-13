//! LibraryTransport parity: add folder, resolve paths, artwork metadata, bus cmds.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use host_flutter::api::library::LibraryTransport;
use library_api::{decode_evt_body, EvtBody, Kind};
use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Tag, TagType};

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

fn write_wav_with_artwork(path: &Path, artwork: &[u8]) {
    write_minimal_wav(path);
    let mut tagged = Probe::open(path).unwrap().read().unwrap();
    if tagged.primary_tag().is_none() {
        tagged.insert_tag(Tag::new(TagType::Id3v2));
    }
    let tag = tagged.primary_tag_mut().expect("id3 tag");
    tag.set_picture(
        0,
        Picture::new_unchecked(
            PictureType::CoverFront,
            Some(MimeType::Jpeg),
            None,
            artwork.to_vec(),
        ),
    );
    tagged
        .save_to_path(path, WriteOptions::default())
        .expect("save artwork tag");
}

#[test]
fn add_folder_resolve_and_track_artwork_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().join("track_a.wav");
    write_minimal_wav(&bare);
    let art_bytes: &[u8] = &[0xFF, 0xD8, 0xFF, 0xD9, 0x01, 0x02, 0x03];
    let with_art = dir.path().join("track_cover.wav");
    write_wav_with_artwork(&with_art, art_bytes);

    let transport = LibraryTransport::open_in_memory().unwrap();
    let added = transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned())
        .unwrap();
    assert_eq!(added.added, 2);
    assert_eq!(added.collection.track_count, 2);

    let collections = transport.list_collections().unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].track_count, 2);

    let tracks = transport
        .list_collection_tracks(collections[0].id.clone())
        .unwrap();
    assert_eq!(tracks.len(), 2);

    let bare_row = tracks
        .iter()
        .find(|t| t.path.ends_with("track_a.wav"))
        .expect("bare wav");
    assert!(bare_row.artwork.is_none());

    let cover_row = tracks
        .iter()
        .find(|t| t.path.ends_with("track_cover.wav"))
        .expect("cover wav");
    // Lists stay artwork-free until covers are stored in library.db.
    assert!(cover_row.artwork.is_none());

    let resolved = transport
        .resolve_tracks_for_paths(vec![bare.to_string_lossy().into_owned()])
        .unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].track.id, bare_row.id);

    let full_cover = transport.get_track(cover_row.id.clone()).unwrap().unwrap();
    assert_eq!(full_cover.artwork.as_deref(), Some(art_bytes));
}

#[test]
fn refresh_missing_track_publishes_error_evt() {
    let transport = LibraryTransport::open_in_memory().unwrap();
    let rx = transport.subscribe_evt_all().unwrap();

    transport.refresh_track("missing-track-id".into()).unwrap();

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
fn analyze_missing_track_publishes_error_evt() {
    let transport = LibraryTransport::open_in_memory().unwrap();
    let rx = transport.subscribe_evt_all().unwrap();

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
