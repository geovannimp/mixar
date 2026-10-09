//! Mixxx adapter tests: read a synthetic `mixxxdb.sqlite`.

use std::path::{Path, PathBuf};

use library::{LibraryConfig, LibraryManager};
use library_adapters::MixxxLibrary;
use library_core::{CollectionType, Library, Migratable, MigrateOptions, TrackId};
use rusqlite::{params, Connection};

struct Fixture {
    _dir: tempfile::TempDir,
    db_path: PathBuf,
    folder: PathBuf,
    present: PathBuf,
}

fn build_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().to_path_buf();
    let present = folder.join("present.mp3");
    std::fs::write(&present, b"").unwrap();
    let missing = folder.join("missing.flac");

    let db_path = folder.join("mixxxdb.sqlite");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE track_locations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            location varchar(512) UNIQUE,
            filename varchar(512),
            directory varchar(512),
            filesize INTEGER,
            fs_deleted INTEGER,
            needs_verification INTEGER
        );
        CREATE TABLE library (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            artist varchar(64),
            title varchar(64),
            album varchar(64),
            genre varchar(64),
            duration FLOAT,
            bitrate INTEGER,
            samplerate INTEGER,
            bpm FLOAT,
            channels INTEGER,
            replaygain FLOAT,
            key varchar(16),
            key_id INTEGER,
            location INTEGER,
            mixxx_deleted INTEGER DEFAULT 0
        );
        CREATE TABLE Playlists (
            id INTEGER PRIMARY KEY,
            name varchar(48),
            position INTEGER,
            hidden INTEGER DEFAULT 0 NOT NULL,
            date_created datetime,
            date_modified datetime
        );
        CREATE TABLE PlaylistTracks (
            id INTEGER PRIMARY KEY,
            playlist_id INTEGER,
            track_id INTEGER,
            position INTEGER
        );
        CREATE TABLE crates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name varchar(48) UNIQUE NOT NULL,
            count INTEGER DEFAULT 0,
            show INTEGER DEFAULT 1
        );
        CREATE TABLE crate_tracks (
            crate_id INTEGER NOT NULL,
            track_id INTEGER NOT NULL,
            UNIQUE (crate_id, track_id)
        );
        CREATE TABLE directories (
            id integer primary key,
            directory varchar(512) unique,
            source integer default 0,
            type integer default 0
        );",
    )
    .unwrap();

    let insert_location = |conn: &Connection, id: i64, path: &Path| {
        conn.execute(
            "INSERT INTO track_locations (id, location, filename, directory, filesize, fs_deleted, needs_verification)
             VALUES (?1, ?2, ?3, ?4, 0, 0, 0)",
            params![
                id,
                path.to_string_lossy(),
                path.file_name().unwrap().to_string_lossy(),
                path.parent().unwrap().to_string_lossy(),
            ],
        )
        .unwrap();
    };
    insert_location(&conn, 1, &present);
    insert_location(&conn, 2, &missing);

    conn.execute(
        "INSERT INTO library (id, artist, title, album, genre, duration, bitrate, samplerate, bpm, channels, replaygain, key, location, mixxx_deleted)
         VALUES (1, 'Artist A', 'Present', 'Album', 'House', 210.5, 320, 44100, 128.0, 2, 1.5, '8A', 1, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO library (id, artist, title, album, genre, duration, bitrate, samplerate, bpm, channels, replaygain, key, location, mixxx_deleted)
         VALUES (2, NULL, 'Missing', NULL, NULL, 0.0, 0, 0, 0.0, 0, NULL, 'C', 2, 0)",
        [],
    )
    .unwrap();
    // Deleted-from-library track must be ignored even if a playlist references it.
    conn.execute(
        "INSERT INTO library (id, artist, title, bpm, key, location, mixxx_deleted)
         VALUES (3, 'Artist B', 'Hidden', 0.0, NULL, 2, 1)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO Playlists (id, name, position, hidden) VALUES (1, 'Warmup', 0, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO Playlists (id, name, position, hidden) VALUES (2, 'AutoDJ', 1, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO PlaylistTracks (playlist_id, track_id, position) VALUES (1, 2, 0), (1, 1, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO PlaylistTracks (playlist_id, track_id, position) VALUES (2, 1, 0)",
        [],
    )
    .unwrap();

    conn.execute("INSERT INTO crates (id, name) VALUES (1, 'Favs')", [])
        .unwrap();
    conn.execute(
        "INSERT INTO crate_tracks (crate_id, track_id) VALUES (1, 1)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO directories (id, directory) VALUES (1, ?1)",
        params![folder.to_string_lossy()],
    )
    .unwrap();

    drop(conn);

    Fixture {
        _dir: dir,
        db_path,
        folder,
        present,
    }
}

#[test]
fn lists_visible_collections_and_skips_hidden() {
    let fixture = build_fixture();
    let library = MixxxLibrary::open(&fixture.db_path).unwrap();
    assert_eq!(library.name(), "mixxx");

    let collections = library.list_collections().unwrap();
    let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Warmup"));
    assert!(names.contains(&"Favs"));
    assert!(
        !names.contains(&"AutoDJ"),
        "hidden AutoDJ playlist must not be listed"
    );

    let playlist = collections
        .iter()
        .find(|c| c.name == "Warmup")
        .expect("Warmup playlist");
    assert_eq!(playlist.collection_type(), CollectionType::Playlist);
    assert!(playlist.sortable());

    let crate_collection = collections.iter().find(|c| c.name == "Favs").unwrap();
    assert!(!crate_collection.sortable());

    assert_eq!(library.preview().folder_count, 1);
    assert_eq!(library.preview().playlist_count, 1);
    assert_eq!(library.preview().crate_count, 1);
    assert_eq!(library.preview().track_count, 2);
    assert_eq!(library.preview().missing_file_count, 1);
}

#[test]
fn playlist_entries_are_position_ordered() {
    let fixture = build_fixture();
    let library = MixxxLibrary::open(&fixture.db_path).unwrap();
    let playlist = library
        .list_collections()
        .unwrap()
        .into_iter()
        .find(|c| c.name == "Warmup")
        .unwrap();

    let entries = library.list_collection_entries(&playlist.id).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].track_id.as_str(), "mixxx:track:2");
    assert_eq!(entries[1].track_id.as_str(), "mixxx:track:1");
    assert_eq!(entries[0].position, Some(0));
    assert_eq!(entries[1].position, Some(1));
}

#[test]
fn camelot_key_normalized_to_musical() {
    let fixture = build_fixture();
    let library = MixxxLibrary::open(&fixture.db_path).unwrap();

    let present = library
        .get_track(&TrackId::new("mixxx:track:1"))
        .unwrap()
        .unwrap();
    assert_eq!(present.metadata().key.as_deref(), Some("Am"));
    assert_eq!(present.metadata().bpm, Some(128.0));
    // Mixxx stores replaygain as a linear ratio (1.0 = 0 dB).
    let gain = present.metadata().replaygain_track_gain_db.unwrap();
    assert!((gain - 20.0 * 1.5f64.log10()).abs() < 1e-6);
    assert_eq!(present.metadata().duration_ms, Some(210_500));
    assert_eq!(present.file().unwrap().path(), fixture.present.as_path());

    // Missing file track is still listed; already-musical key passes through.
    let missing = library
        .get_track(&TrackId::new("mixxx:track:2"))
        .unwrap()
        .unwrap();
    assert_eq!(missing.metadata().key.as_deref(), Some("C"));
    // bpm 0 in Mixxx means "undefined".
    assert_eq!(missing.metadata().bpm, None);

    // A track with mixxx_deleted = 1 is not loaded.
    assert!(library
        .get_track(&TrackId::new("mixxx:track:3"))
        .unwrap()
        .is_none());
    let _ = &fixture.folder;
}

#[test]
fn migrate_imports_tracks_lists_and_missing_files() {
    let fixture = build_fixture();
    let mixxx = MixxxLibrary::open(&fixture.db_path).unwrap();
    let mut target = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();

    let report = mixxx
        .migrate(&mut target, &MigrateOptions::default())
        .unwrap();
    assert_eq!(report.tracks_added, 2);
    assert_eq!(report.tracks_missing_files, 1);
    assert_eq!(report.folders_imported, 1);
    assert_eq!(report.playlists_imported, 1);
    assert_eq!(report.crates_imported, 1);
    assert_eq!(report.failed, 0, "errors: {:?}", report.errors);

    let collections = target.list_collections().unwrap();
    let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Warmup"));
    assert!(names.contains(&"Favs"));
    assert!(
        !names.contains(&"AutoDJ"),
        "hidden playlist must not migrate"
    );

    // Every imported track is reachable via the catch-all "Mixxx" collection.
    let mixxx = collections
        .iter()
        .find(|c| c.name == "Mixxx")
        .expect("catch-all Mixxx collection");
    let mixxx_tracks = target.list_collection_tracks(&mixxx.id).unwrap();
    assert_eq!(mixxx_tracks.len(), 2);
    assert!(mixxx_tracks
        .iter()
        .any(|track| track.metadata().title.as_deref() == Some("Missing")));
    assert!(mixxx_tracks
        .iter()
        .any(|track| track.metadata().title.as_deref() == Some("Present")));

    let warmup = collections.iter().find(|c| c.name == "Warmup").unwrap();
    let tracks = target.list_collection_tracks(&warmup.id).unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].metadata().title.as_deref(), Some("Missing"));
    assert_eq!(tracks[1].metadata().title.as_deref(), Some("Present"));
    // The missing-file track is present in the target pool (path recorded).
    assert!(target.get_track(tracks[0].id()).unwrap().is_some());

    let entries = target.list_collection_entries(&warmup.id).unwrap();
    assert_eq!(entries[0].position, Some(0));
    assert_eq!(entries[1].position, Some(1));

    // Crate membership is a set (no positions) and the folder tracks a real dir.
    let favs = collections.iter().find(|c| c.name == "Favs").unwrap();
    let crate_entries = target.list_collection_entries(&favs.id).unwrap();
    assert_eq!(crate_entries.len(), 1);
    assert_eq!(crate_entries[0].position, None);

    let folder = collections
        .iter()
        .find(|c| c.collection_type() == CollectionType::Folder)
        .expect("watched directory imported as a folder");
    let folder_tracks = target.list_collection_tracks(&folder.id).unwrap();
    assert!(folder_tracks
        .iter()
        .any(|track| track.metadata().title.as_deref() == Some("Present")));
}

#[test]
fn migrate_is_idempotent_for_lists() {
    let fixture = build_fixture();
    let mixxx = MixxxLibrary::open(&fixture.db_path).unwrap();
    let mut target = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();

    let first = mixxx
        .migrate(&mut target, &MigrateOptions::default())
        .unwrap();
    assert_eq!(first.playlists_imported, 1);
    assert_eq!(first.crates_imported, 1);

    let second = mixxx
        .migrate(&mut target, &MigrateOptions::default())
        .unwrap();
    assert_eq!(second.playlists_imported, 0);
    assert_eq!(second.crates_imported, 0);
    assert_eq!(
        second.folders_imported, 0,
        "an already-watched directory is not re-created"
    );
    assert_eq!(second.collections_skipped, 3);
    assert_eq!(second.tracks_added, 0);
    assert_eq!(second.tracks_updated, 2);

    let playlists = target
        .list_collections()
        .unwrap()
        .into_iter()
        .filter(|c| c.collection_type() == CollectionType::Playlist)
        .collect::<Vec<_>>();
    assert_eq!(playlists.len(), 3, "no duplicate playlists/crates");
    assert_eq!(
        playlists.iter().filter(|c| c.name == "Mixxx").count(),
        1,
        "the catch-all is reused, not recreated"
    );

    // Re-running keeps the catch-all complete without duplicating entries.
    let mixxx = playlists.iter().find(|c| c.name == "Mixxx").unwrap();
    assert_eq!(target.list_collection_tracks(&mixxx.id).unwrap().len(), 2);
}

#[test]
fn reads_legacy_text_location_schema() {
    // Schema revision < 3 stores the path as text in `library.location` and
    // joins `track_locations` on `location` rather than `id`.
    let dir = tempfile::tempdir().unwrap();
    let present = dir.path().join("legacy.mp3");
    std::fs::write(&present, b"").unwrap();
    let db_path = dir.path().join("mixxxdb.sqlite");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "CREATE TABLE track_locations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            location varchar(512) UNIQUE, filename varchar(512), directory varchar(512),
            filesize INTEGER, fs_deleted INTEGER, needs_verification INTEGER
        );
        CREATE TABLE library (
            id INTEGER PRIMARY KEY AUTOINCREMENT, artist varchar(48), title varchar(48),
            location varchar(512), bpm FLOAT, key varchar(16)
        );",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO track_locations (id, location) VALUES (1, ?1)",
        params![present.to_string_lossy()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO library (id, artist, title, location, bpm, key) VALUES (1, 'A', 'Legacy', ?1, 120.0, 'Am')",
        params![present.to_string_lossy()],
    )
    .unwrap();
    drop(conn);

    let library = MixxxLibrary::open(&db_path).unwrap();
    let track = library
        .get_track(&TrackId::new("mixxx:track:1"))
        .unwrap()
        .expect("legacy track resolves");
    assert_eq!(track.file().unwrap().path(), present.as_path());
    assert_eq!(track.metadata().title.as_deref(), Some("Legacy"));
}
