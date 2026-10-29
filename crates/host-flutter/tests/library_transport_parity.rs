//! LibraryTransport parity: add folder, resolve paths, artwork metadata, bus cmds.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use host_flutter::api::library::{LibraryTransport, SamplerPlayMode};
use library_api::{decode_evt_body, EvtBody, Kind};
use lofty::config::WriteOptions;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::{Tag, TagType};

#[test]
fn create_sampler_bank_trims_names_and_maps_play_modes() {
    let transport = LibraryTransport::open_in_memory().unwrap();

    let err = transport
        .create_sampler_bank("   ".into(), None)
        .unwrap_err();
    assert!(err.contains("empty"), "unexpected error: {err}");

    let bank = transport
        .create_sampler_bank("  Pads  ".into(), Some(SamplerPlayMode::Hold))
        .unwrap();
    assert_eq!(bank.name, "Pads");
    assert_eq!(bank.play_mode, Some(SamplerPlayMode::Hold));

    let banks = transport.list_sampler_banks().unwrap();
    assert_eq!(banks.len(), 1);
    assert_eq!(banks[0].id, bank.id);
}

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
        Picture::unchecked(artwork.to_vec())
            .pic_type(PictureType::CoverFront)
            .mime_type(MimeType::Png)
            .build(),
    );
    tagged
        .save_to_path(path, WriteOptions::default())
        .expect("save artwork tag");

    // Fail loudly when the container dropped the picture: a silently
    // unsupported container must not read as "this track has no artwork".
    let reread = Probe::open(path).unwrap().read().unwrap();
    assert_eq!(
        reread.primary_tag().expect("id3 tag").pictures().len(),
        1,
        "lofty must read back the picture it wrote"
    );
}

/// 300×200 PNG cover — larger than the UI thumbnail edge, so it gets resized.
fn cover_png() -> Vec<u8> {
    let rgb = image::RgbImage::from_fn(300, 200, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 32])
    });
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("encode cover");
    bytes
}

#[test]
fn add_folder_resolve_and_track_artwork_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().join("track_a.wav");
    write_minimal_wav(&bare);
    let cover = cover_png();
    let with_art = dir.path().join("track_cover.wav");
    write_wav_with_artwork(&with_art, &cover);

    let transport = LibraryTransport::open_in_memory().unwrap();
    let added = transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned(), true, None)
        .unwrap();
    assert_eq!(added.added, 2);
    assert_eq!(added.collection.track_count, Some(2));

    let collections = transport.list_collections().unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].track_count, Some(2));

    let tracks = transport
        .list_collection_entries(collections[0].id.clone())
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

    // Artwork comes back as a UI thumbnail, never as the full-resolution cover.
    let fetched = transport.get_track(cover_row.id.clone()).unwrap().unwrap();
    let artwork = fetched.artwork.expect("cover artwork");
    assert_ne!(artwork, cover, "the full-size cover must not be shipped");
    let decoded = image::load_from_memory(&artwork).expect("artwork is an image");
    assert!(decoded.width().max(decoded.height()) <= library::ARTWORK_UI_EDGE_PX);
}

#[test]
fn delete_collection_removes_orphaned_tracks() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("keep.wav"));

    let transport = LibraryTransport::open_in_memory().unwrap();
    let added = transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned(), true, None)
        .unwrap();
    let collection_id = added.collection.id.clone();
    let track_id = transport
        .list_collection_entries(collection_id.clone())
        .unwrap()[0]
        .id
        .clone();

    transport.delete_collection(collection_id.clone()).unwrap();

    assert!(transport.list_collections().unwrap().is_empty());
    assert!(
        transport.get_track(track_id).unwrap().is_none(),
        "the collection was the only reference to the track"
    );
    assert!(
        transport.delete_collection(collection_id).is_err(),
        "second delete reports the missing collection"
    );
}

#[test]
fn add_folder_collection_skips_nested_when_not_recursive() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("root.wav"));
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    write_minimal_wav(&nested.join("child.wav"));

    let shallow = LibraryTransport::open_in_memory().unwrap();
    let added = shallow
        .add_folder_collection(dir.path().to_string_lossy().into_owned(), false, None)
        .unwrap();
    assert_eq!(added.added, 1);

    let deep = LibraryTransport::open_in_memory().unwrap();
    let added = deep
        .add_folder_collection(dir.path().to_string_lossy().into_owned(), true, None)
        .unwrap();
    assert_eq!(added.added, 2);
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

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let event = loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let event = rx
            .recv_timeout(remaining)
            .expect("evt bus alive")
            .expect("Error evt");
        // Analyze publishes TrackProgress before failure; wait for the Error.
        if event.kind() == &Kind::Error {
            break event;
        }
        assert_eq!(
            event.kind(),
            &Kind::TrackProgress,
            "unexpected evt before Error: {:?}",
            event.kind()
        );
    };
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
        .add_folder_collection(dir.path().to_string_lossy().into_owned(), true, None)
        .unwrap();
    let tracks = transport
        .list_collection_entries(added.collection.id.clone())
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
