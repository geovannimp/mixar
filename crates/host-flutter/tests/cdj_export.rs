//! CDJ USB export through the host `LibraryTransport` bridge.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::io::Write;
use std::path::Path;

/// One second of silence as a WAV, so the folder scan registers a real track.
fn write_minimal_wav(path: &Path) {
    let sample_rate = 8_000u32;
    let sample_count = sample_rate as usize;
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
fn export_cdj_usb_through_transport() {
    let media = tempfile::tempdir().unwrap();
    write_minimal_wav(&media.path().join("warmup.wav"));

    let transport = host_flutter::testing::open_library_in_memory().unwrap();
    let folder = transport
        .add_folder_collection(
            media.path().to_string_lossy().into_owned(),
            true,
            Some("Sets".to_string()),
        )
        .unwrap();
    assert_eq!(folder.collection.track_count, 1);
    let playlist = transport
        .add_playlist_collection("Warmup".to_string(), true)
        .unwrap();

    // A destination that does not exist yet: the exporter creates it.
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("cdj-usb");
    let destination_arg = destination.to_string_lossy().into_owned();

    let report = transport
        .export_cdj_usb(
            destination_arg.clone(),
            vec![playlist.id.clone()],
            vec![folder.collection.id.clone()],
            false,
        )
        .unwrap();

    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    // The selected playlist plus the folder collection's own playlist.
    assert_eq!(report.playlists, 2);
    assert_eq!(report.folders, 1);
    assert_eq!(report.tracks, 1);
    assert!(
        destination.join("PIONEER/rekordbox/export.pdb").is_file(),
        "device database missing under {destination_arg}"
    );
}
