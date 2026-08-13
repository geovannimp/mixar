//! LibraryTransport parity: add folder, resolve paths, artwork.

use std::io::Write;
use std::path::Path;

use host_flutter::api::library::LibraryTransport;

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
