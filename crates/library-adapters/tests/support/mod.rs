//! Shared harness for the Rekordbox adapter tests.
//!
//! Both the synthetic fixture suite and the opt-in real-library smoke test use
//! [`migrate_twice`], so the invariants checked against a real `master.db` are
//! the same ones the fixture suite already covers.

use std::path::Path;
use std::time::SystemTime;

use library::{LibraryConfig, LibraryManager};
use library_adapters::rekordbox::RekordboxLibrary;
use library_core::{Library, Migratable, MigrateOptions, MigrateReport};

/// Name of the adapter's catch-all playlist.
///
/// The fixture suite and the opt-in real-library smoke test both locate it, so
/// the name is declared once instead of retyped per assertion.
pub const CATCH_ALL_NAME: &str = "Rekordbox";

/// Cheap fingerprint that catches any write: length, mtime, and the first and
/// last 4 KiB (a rewritten page changes one of them without reading a
/// multi-hundred-megabyte database twice).
type Fingerprint = (u64, Option<SystemTime>, Vec<u8>, Vec<u8>);

fn fingerprint(path: &Path) -> Fingerprint {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path).unwrap();
    let metadata = file.metadata().unwrap();
    let len = metadata.len();

    let mut head = vec![0u8; 4096];
    let read = file.read(&mut head).unwrap();
    head.truncate(read);

    let tail_start = len.saturating_sub(4096);
    file.seek(SeekFrom::Start(tail_start)).unwrap();
    let mut tail = Vec::new();
    file.read_to_end(&mut tail).unwrap();

    (len, metadata.modified().ok(), head, tail)
}

/// Migrate `library` into a fresh library twice and assert what must hold for
/// **any** Rekordbox database — synthetic fixture or a real install:
///
/// - the first run imports every loaded track exactly once and never fails;
/// - the catch-all playlist holds all of them;
/// - the second run adds no track and no list, dedupes every list it created,
///   and re-upserts the cues instead of duplicating them;
/// - the source database file is byte-for-byte untouched.
///
/// Specific counts stay with the caller; this only checks the relation between
/// the report and the library's own contents.
pub fn migrate_twice(library: &RekordboxLibrary, source: &Path) -> (MigrateReport, MigrateReport) {
    let before = fingerprint(source);
    let preview = library.preview();

    let dir = tempfile::tempdir().unwrap();
    let mut manager =
        LibraryManager::open(dir.path().join("library.db"), LibraryConfig::default()).unwrap();

    let first = library
        .migrate(&mut manager, &MigrateOptions::default())
        .unwrap();
    assert_eq!(first.failed, 0, "errors: {:?}", first.errors);
    assert_eq!(
        first.tracks_added + first.tracks_updated,
        preview.track_count,
        "every loaded track reaches the target"
    );
    assert!(
        first.hot_cues_imported <= preview.hot_cue_count,
        "hot cues: imported {} of {}",
        first.hot_cues_imported,
        preview.hot_cue_count
    );
    assert!(
        first.loops_imported <= preview.loop_count,
        "loops: imported {} of {}",
        first.loops_imported,
        preview.loop_count
    );

    let collections = manager.list_collections().unwrap();
    let catch_all = collections
        .iter()
        .find(|collection| collection.name == CATCH_ALL_NAME && !collection.sortable())
        .expect("catch-all playlist");
    assert!(!catch_all.sortable(), "catch-all is an unsortable set");
    assert_eq!(
        manager
            .list_collection_entries(&catch_all.id)
            .unwrap()
            .len(),
        preview.track_count,
        "catch-all holds every imported track"
    );

    let second = library
        .migrate(&mut manager, &MigrateOptions::default())
        .unwrap();
    assert_eq!(second.failed, 0, "errors: {:?}", second.errors);
    assert_eq!(second.tracks_added, 0, "a re-import adds no track");
    assert_eq!(second.tracks_updated, preview.track_count);
    assert_eq!(second.playlists_imported, 0, "a re-import adds no list");
    assert_eq!(
        second.playlists_imported + second.collections_skipped,
        first.playlists_imported + first.collections_skipped,
        "every source list is accounted for on the second run"
    );
    assert_eq!(
        second.hot_cues_imported, first.hot_cues_imported,
        "cues upsert by slot instead of duplicating"
    );
    assert_eq!(
        manager
            .list_collection_entries(&catch_all.id)
            .unwrap()
            .len(),
        preview.track_count,
        "the catch-all does not grow on a re-import"
    );

    assert_eq!(
        fingerprint(source),
        before,
        "the source database is untouched"
    );
    (first, second)
}
