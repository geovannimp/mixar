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
        assert_eq!(lib.get_collection_tracks(&collection.id).unwrap().len(), 1);
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
