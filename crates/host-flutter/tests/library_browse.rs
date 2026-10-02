//! Browse smoke: open library DB, list collections and tracks.

use std::io::Write;
use std::path::Path;

use host_flutter::api::library::LibraryTransport;
use library::{LibraryConfig, LibraryManager, NewCollection, WritableLibrary};
use library_core::Library;

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
fn list_collections_and_tracks_from_disk_db() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track_a.wav");
    write_minimal_wav(&wav);

    let db = dir.path().join("library.db");
    {
        let mut lib = LibraryManager::open(&db, LibraryConfig::default()).unwrap();
        let collection = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&collection.id)).unwrap();
        assert_eq!(lib.list_collection_tracks(&collection.id).unwrap().len(), 1);
    }

    let transport = LibraryTransport::open(db.to_string_lossy().into_owned()).unwrap();
    let collections = transport.list_collections().unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].track_count, 1);

    let tracks = transport
        .list_collection_entries(collections[0].id.clone())
        .unwrap();
    assert_eq!(tracks.len(), 1);
    assert!(tracks[0].path.ends_with("track_a.wav"));
}

#[test]
fn open_in_memory_lists_empty() {
    let transport = LibraryTransport::open_in_memory().unwrap();
    assert!(transport.list_collections().unwrap().is_empty());
}

#[test]
fn analyze_collection_queues_targets_onto_bus() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("a.wav"));
    write_minimal_wav(&dir.path().join("b.wav"));

    let db = dir.path().join("library.db");
    let transport = LibraryTransport::open(db.to_string_lossy().into_owned()).unwrap();
    let added = transport
        .add_folder_collection(
            dir.path().to_string_lossy().into_owned(),
            true,
            Some("Batch".into()),
        )
        .unwrap();
    assert_eq!(added.added, 2);

    // Synchronous fan-out: one AnalyzeTrack per target, no stems requested.
    let result = transport
        .analyze_collection(added.collection.id.clone(), false, false)
        .unwrap();
    assert_eq!(result.queued_track_ids.len(), 2);
    assert!(result.stem_track_ids.is_empty());

    // Force re-includes every file track regardless of worker progress.
    let forced = transport
        .analyze_collection(added.collection.id.clone(), true, false)
        .unwrap();
    assert_eq!(forced.queued_track_ids.len(), 2);
}
