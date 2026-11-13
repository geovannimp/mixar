//! Rekordbox adapter tests: read a synthetic, SQLCipher-encrypted `master.db`.
#![cfg(feature = "rekordbox")]
#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers

mod support;

use std::path::PathBuf;

use library::{LibraryConfig, LibraryManager};
use library_adapters::rekordbox::RekordboxLibrary;
use library_core::{
    CollectionType, CueSource, Library, LibraryError, Migratable, MigrateOptions, TrackCue, TrackId,
};
use rusqlite::{params, Connection};

/// Passphrase the fixture is encrypted with. Deliberately not the constant the
/// adapter ships with: every test goes through the explicit-key entry point.
const KEY: &str = "test-passphrase";

/// Name of the adapter's catch-all playlist; referenced, never retyped, so an
/// importer rename cannot leave these assertions looking for a stale name.
const CATCH_ALL_NAME: &str = "Rekordbox";

struct Fixture {
    _dir: tempfile::TempDir,
    db_path: PathBuf,
    present: PathBuf,
    missing: PathBuf,
}

fn build_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().to_path_buf();
    let present = folder.join("ivy.mp3");
    std::fs::write(&present, b"").unwrap();
    let missing = folder.join("nikes.flac");

    let db_path = folder.join("master.db");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(&format!("PRAGMA key = '{KEY}';"))
        .unwrap();
    conn.execute_batch(
        "CREATE TABLE djmdArtist (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdAlbum (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdGenre (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdKey (ID TEXT PRIMARY KEY, ScaleName TEXT);
        CREATE TABLE djmdColor (ID TEXT PRIMARY KEY, ColorCode INTEGER, Commnt TEXT);
        CREATE TABLE djmdContent (
            ID TEXT PRIMARY KEY,
            Title TEXT,
            ArtistID TEXT,
            AlbumID TEXT,
            GenreID TEXT,
            KeyID TEXT,
            ColorID TEXT,
            BPM INTEGER,
            Length INTEGER,
            SampleRate INTEGER,
            BitRate INTEGER,
            Rating INTEGER,
            Commnt TEXT,
            ReleaseYear INTEGER,
            DiscNo INTEGER,
            TrackNo INTEGER,
            ISRC TEXT,
            FolderPath TEXT,
            FileNameL TEXT,
            FileNameS TEXT
        );
        CREATE TABLE djmdCue (
            ID TEXT PRIMARY KEY,
            ContentID TEXT,
            InMsec INTEGER,
            OutMsec INTEGER,
            Kind INTEGER,
            Color INTEGER,
            Comment TEXT
        );
        CREATE TABLE djmdPlaylist (
            ID TEXT PRIMARY KEY,
            Name TEXT,
            Attribute INTEGER,
            ParentID TEXT,
            Seq INTEGER
        );
        CREATE TABLE djmdSongPlaylist (
            ID TEXT PRIMARY KEY,
            PlaylistID TEXT,
            ContentID TEXT,
            TrackNo INTEGER
        );",
    )
    .unwrap();

    conn.execute_batch(
        "INSERT INTO djmdArtist (ID, Name) VALUES ('a1', 'Frank Ocean');
        INSERT INTO djmdAlbum (ID, Name) VALUES ('al1', 'Blonde');
        INSERT INTO djmdGenre (ID, Name) VALUES ('g1', 'R&B');
        INSERT INTO djmdKey (ID, ScaleName) VALUES ('k1', '8A'), ('k2', 'F#m');
        -- Real libraries leave ColorCode NULL and name the palette in Commnt.
        INSERT INTO djmdColor (ID, ColorCode, Commnt) VALUES ('c1', NULL, 'Red');",
    )
    .unwrap();

    // t1 exists on disk; t2 does not. t2's BPM is stored scaled by 100.
    conn.execute(
        "INSERT INTO djmdContent (
            ID, Title, ArtistID, AlbumID, GenreID, KeyID, ColorID, BPM, Length, SampleRate,
            BitRate, Rating, Commnt, ReleaseYear, DiscNo, TrackNo, ISRC, FolderPath, FileNameL
         ) VALUES (
            't1', 'Ivy', 'a1', 'al1', 'g1', 'k1', 'c1', 80, 250, 44100,
            320, 4, '  ', 2016, 1, 2, 'USABC1234567', ?1, 'ivy.mp3'
         )",
        params![folder.to_string_lossy()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO djmdContent (
            ID, Title, ArtistID, KeyID, BPM, Length, FolderPath, FileNameL
         ) VALUES ('t2', 'Nikes', 'a1', 'k2', 12800, 300, ?1, 'nikes.flac')",
        params![folder.join("nikes.flac").to_string_lossy()],
    )
    .unwrap();

    // Cue A with no loop, hot loop C (7500 ms at 80 BPM = 10 beats), and a
    // memory loop.
    conn.execute_batch(
        "INSERT INTO djmdCue (ID, ContentID, InMsec, OutMsec, Kind, Color, Comment)
         VALUES ('q1', 't1', 10000, -1, 1, 2, 'Intro'),
                ('q2', 't1', 60000, 67500, 3, 4, 'Drop'),
                ('q3', 't1', 20000, 25000, 0, -1, NULL),
                ('q4', 't1', 30000, -1, 5, 255, NULL);",
    )
    .unwrap();

    // A folder holding one playlist, plus a smart playlist, both with members.
    conn.execute_batch(
        "INSERT INTO djmdPlaylist (ID, Name, Attribute, ParentID, Seq)
         VALUES ('p1', 'House', 1, '', 1),
                ('p2', 'Warmup', 0, 'p1', 1),
                ('p3', 'Smart', 4, '', 2);
         INSERT INTO djmdSongPlaylist (ID, PlaylistID, ContentID, TrackNo)
         VALUES ('s1', 'p2', 't2', 1),
                ('s2', 'p2', 't1', 2),
                ('s3', 'p3', 't1', 1);",
    )
    .unwrap();

    drop(conn);

    Fixture {
        _dir: dir,
        db_path,
        present,
        missing,
    }
}

fn open(fixture: &Fixture) -> RekordboxLibrary {
    RekordboxLibrary::open_keyed(&fixture.db_path, KEY).unwrap()
}

#[test]
fn preview_counts_tracks_lists_and_cues() {
    let fixture = build_fixture();
    let library = open(&fixture);
    assert_eq!(library.name(), "rekordbox");

    let preview = library.preview();
    assert_eq!(preview.track_count, 2);
    assert_eq!(preview.missing_file_count, 1);
    assert_eq!(preview.playlist_count, 1);
    assert_eq!(preview.smart_playlist_count, 1);
    assert_eq!(preview.folder_count, 1);
    assert_eq!(preview.hot_cue_count, 3);
    assert_eq!(preview.loop_count, 1);
}

#[test]
fn tracks_map_lookups_and_normalise_values() {
    let fixture = build_fixture();
    let library = open(&fixture);

    let track = library
        .get_track(&TrackId::new("rekordbox:track:t1"))
        .unwrap()
        .unwrap();
    let metadata = track.metadata();
    assert_eq!(metadata.title.as_deref(), Some("Ivy"));
    assert_eq!(metadata.artist.as_deref(), Some("Frank Ocean"));
    assert_eq!(metadata.album.as_deref(), Some("Blonde"));
    assert_eq!(metadata.genre.as_deref(), Some("R&B"));
    // Camelot written by Rekordbox is normalised to musical notation.
    assert_eq!(metadata.key.as_deref(), Some("Am"));
    assert_eq!(metadata.bpm, Some(80.0));
    assert_eq!(metadata.duration_ms, Some(250_000));
    assert_eq!(metadata.sample_rate, Some(44_100));
    assert_eq!(metadata.bitrate_kbps, Some(320));
    assert_eq!(metadata.rating, Some(4));
    // A whitespace-only comment is not a comment.
    assert_eq!(metadata.comment, None);
    assert_eq!(metadata.year, Some(2016));
    assert_eq!(metadata.disc_number, Some(1));
    assert_eq!(metadata.track_number, Some(2));
    assert_eq!(metadata.isrc.as_deref(), Some("USABC1234567"));
    assert_eq!(metadata.color_label, Some(library_core::ColorLabel::Red));
    assert_eq!(track.file().unwrap().path(), fixture.present.as_path());

    // The absent file is still a track, with its BPM un-scaled.
    let absent = library
        .get_track(&TrackId::new("rekordbox:track:t2"))
        .unwrap()
        .unwrap();
    assert_eq!(absent.metadata().bpm, Some(128.0));
    assert_eq!(absent.file().unwrap().path(), fixture.missing.as_path());
    assert_eq!(absent.metadata().title.as_deref(), Some("Nikes"));
}

#[test]
fn lists_flatten_folder_paths_and_keep_entry_order() {
    let fixture = build_fixture();
    let library = open(&fixture);

    let collections = library.list_collections().unwrap();
    let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["House / Warmup", "Smart"]);

    let warmup = collections
        .iter()
        .find(|c| c.name == "House / Warmup")
        .unwrap();
    assert_eq!(warmup.collection_type(), CollectionType::Playlist);
    assert!(warmup.sortable());

    let entries = library.list_collection_entries(&warmup.id).unwrap();
    let ids: Vec<&str> = entries
        .iter()
        .map(|entry| entry.track_id.as_str())
        .collect();
    assert_eq!(ids, vec!["rekordbox:track:t2", "rekordbox:track:t1"]);
    assert_eq!(entries[0].position, Some(0));
    assert_eq!(entries[1].position, Some(1));
}

#[test]
fn hot_cues_and_loops_are_grouped_per_track() {
    let fixture = build_fixture();
    let library = open(&fixture);
    let cues = library.cues_for(&TrackId::new("rekordbox:track:t1"));

    assert_eq!(cues.hot_cues.len(), 3);
    let intro = &cues.hot_cues[0];
    assert_eq!(intro.slot, 0, "Kind 1 is cue A");
    assert_eq!(intro.position_ms, 10_000);
    assert_eq!(intro.loop_length_beats, None);
    assert_eq!(intro.label.as_deref(), Some("Intro"));
    assert_eq!(intro.color.as_deref(), Some("#FF0000"));

    let drop = &cues.hot_cues[1];
    assert_eq!(drop.slot, 2, "Kind 3 is cue C");
    assert_eq!(drop.loop_length_beats, Some(10));
    assert_eq!(drop.color.as_deref(), Some("#FFFF00"));

    // 255 is Rekordbox 7's "no colour": never a guessed hue.
    let uncoloured = &cues.hot_cues[2];
    assert_eq!(uncoloured.slot, 4, "Kind 5 is cue E");
    assert_eq!(uncoloured.color, None);
    assert_eq!(uncoloured.label, None);

    assert_eq!(cues.loops.len(), 1);
    assert_eq!(cues.loops[0].slot, 0);
    assert_eq!(cues.loops[0].in_ms, 20_000);
    assert_eq!(cues.loops[0].out_ms, 25_000);

    // The only memory cue is also the track's main cue.
    assert_eq!(cues.main_cue_ms, Some(20_000));

    // A track with no cue rows yields empty sets rather than an error.
    let none = library.cues_for(&TrackId::new("rekordbox:track:t2"));
    assert!(none.hot_cues.is_empty() && none.loops.is_empty());
    assert_eq!(none.main_cue_ms, None);
}

#[test]
fn migration_imports_tracks_lists_and_cues_then_repeats_idempotently() {
    let fixture = build_fixture();
    let library = open(&fixture);

    let target_dir = tempfile::tempdir().unwrap();
    let mut manager = LibraryManager::open(
        target_dir.path().join("library.db"),
        LibraryConfig::default(),
    )
    .unwrap();
    let first = library
        .migrate(&mut manager, &MigrateOptions::default())
        .unwrap();
    assert_eq!(first.tracks_added, 2);
    assert_eq!(first.tracks_updated, 0);
    assert_eq!(first.tracks_missing_files, 1);
    assert_eq!(first.playlists_imported, 2);
    assert_eq!(first.collections_skipped, 0);
    assert_eq!(first.hot_cues_imported, 3);
    assert_eq!(first.loops_imported, 1);
    assert_eq!(first.failed, 0, "{:?}", first.errors);

    let collections = manager.list_collections().unwrap();
    let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&CATCH_ALL_NAME), "{names:?}");
    assert!(names.contains(&"House / Warmup"), "{names:?}");
    assert!(names.contains(&"Smart"), "{names:?}");

    let catch_all = collections
        .iter()
        .find(|c| c.name == CATCH_ALL_NAME)
        .unwrap();
    assert!(!catch_all.sortable(), "catch-all is an unsortable set");
    assert_eq!(
        manager
            .list_collection_entries(&catch_all.id)
            .unwrap()
            .len(),
        2
    );

    let warmup = collections
        .iter()
        .find(|c| c.name == "House / Warmup")
        .unwrap();
    let entries = manager.list_collection_entries(&warmup.id).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        manager
            .get_track(&entries[0].track_id)
            .unwrap()
            .unwrap()
            .metadata()
            .title
            .as_deref(),
        Some("Nikes")
    );

    // Cues landed on the imported track and are readable through the manager.
    let imported = manager
        .list_collection_tracks(&catch_all.id)
        .unwrap()
        .into_iter()
        .find(|source| source.metadata().title.as_deref() == Some("Ivy"))
        .unwrap();
    let imported_id = imported.id().clone();
    let hot_cues = manager.list_track_hot_cues(&imported_id).unwrap();
    assert_eq!(hot_cues.len(), 3);
    assert_eq!(hot_cues[0].slot_index, 0);
    assert_eq!(hot_cues[0].label.as_deref(), Some("Intro"));
    assert_eq!(hot_cues[1].loop_length_beats, Some(10));
    let loops = manager.list_track_loops(&imported_id).unwrap();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].out_ms, 25_000);
    assert_eq!(
        manager.track_cue(&imported_id).unwrap(),
        Some(TrackCue {
            position_ms: 20_000,
            source: CueSource::Import,
        }),
        "the memory cue lands as the track cue"
    );

    // The generic invariants (re-import idempotency, catch-all contents, source
    // left untouched) run here so the opt-in real-library test exercises the
    // same code path.
    let (_, second) = support::migrate_twice(&library, &fixture.db_path);
    assert_eq!(second.tracks_updated, 2);
    assert_eq!(second.collections_skipped, 2);
}

/// A re-import must not overwrite a cue the user set on the deck.
#[test]
fn migration_keeps_an_existing_manual_cue() {
    let fixture = build_fixture();
    let library = open(&fixture);
    let target_dir = tempfile::tempdir().unwrap();
    let mut manager = LibraryManager::open(
        target_dir.path().join("library.db"),
        LibraryConfig::default(),
    )
    .unwrap();
    library
        .migrate(&mut manager, &MigrateOptions::default())
        .unwrap();

    let catch_all = manager
        .list_collections()
        .unwrap()
        .into_iter()
        .find(|collection| collection.name == CATCH_ALL_NAME)
        .unwrap();
    let imported_id = manager
        .list_collection_tracks(&catch_all.id)
        .unwrap()
        .into_iter()
        .find(|source| source.metadata().title.as_deref() == Some("Ivy"))
        .unwrap()
        .id()
        .clone();
    manager.save_track_cue(&imported_id, 250).unwrap();

    library
        .migrate(&mut manager, &MigrateOptions::default())
        .unwrap();

    assert_eq!(
        manager.track_cue(&imported_id).unwrap(),
        Some(TrackCue {
            position_ms: 250,
            source: CueSource::Manual,
        })
    );
}

#[test]
fn a_wrong_key_is_a_typed_error_not_a_panic() {
    let fixture = build_fixture();
    let error = RekordboxLibrary::open_keyed(&fixture.db_path, "not-the-key").unwrap_err();
    let message = error.to_string();
    assert!(message.contains("could not be decrypted"), "{message}");
    assert!(
        message.contains(library_adapters::rekordbox::KEY_ENV),
        "the error names the override: {message}"
    );
    assert!(!message.contains(fixture.db_path.to_string_lossy().as_ref()));
}

#[test]
fn a_plain_sqlite_database_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("other.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE library (id INTEGER PRIMARY KEY);")
        .unwrap();
    drop(conn);

    match RekordboxLibrary::open_keyed(&path, KEY) {
        Err(LibraryError::Unsupported(message)) => {
            assert!(message.contains("Rekordbox 6/7"), "{message}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn a_rekordbox_5_datafile_is_unsupported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("datafile.edb");
    std::fs::write(&path, b"not a sqlite database").unwrap();

    match RekordboxLibrary::open_keyed(&path, KEY) {
        Err(LibraryError::Unsupported(message)) => {
            assert!(message.contains("Rekordbox 5"), "{message}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn a_library_without_cue_rows_still_imports() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("master.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(&format!("PRAGMA key = '{KEY}';"))
        .unwrap();
    conn.execute_batch(
        "CREATE TABLE djmdContent (
            ID TEXT PRIMARY KEY, Title TEXT, ArtistID TEXT, AlbumID TEXT, GenreID TEXT,
            KeyID TEXT, ColorID TEXT, BPM INTEGER, Length INTEGER, SampleRate INTEGER,
            BitRate INTEGER, Rating INTEGER, Commnt TEXT, ReleaseYear INTEGER,
            DiscNo INTEGER, TrackNo INTEGER, ISRC TEXT, FolderPath TEXT, FileNameL TEXT,
            FileNameS TEXT
         );
         CREATE TABLE djmdPlaylist (ID TEXT PRIMARY KEY, Name TEXT, Attribute INTEGER, ParentID TEXT);
         CREATE TABLE djmdSongPlaylist (ID TEXT PRIMARY KEY, PlaylistID TEXT, ContentID TEXT, TrackNo INTEGER);
         -- Only the short file name is filled in: the adapter falls back to it.
         INSERT INTO djmdContent (ID, Title, FolderPath, FileNameS)
             VALUES ('t1', 'Solo', ?1, 'solo.mp3');
         INSERT INTO djmdPlaylist (ID, Name, Attribute, ParentID) VALUES ('p1', 'All', 0, '');",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO djmdSongPlaylist (ID, PlaylistID, ContentID, TrackNo) VALUES ('s1', 'p1', 't1', 1)",
        [],
    )
    .unwrap();
    drop(conn);

    let library = RekordboxLibrary::open_keyed(&path, KEY).unwrap();
    assert_eq!(library.preview().track_count, 1);
    assert_eq!(library.preview().hot_cue_count, 0);
    assert!(library
        .cues_for(&TrackId::new("rekordbox:track:t1"))
        .hot_cues
        .is_empty());
    // No djmdKey / djmdColor tables: the lookups stay empty instead of failing.
    let track = library
        .get_track(&TrackId::new("rekordbox:track:t1"))
        .unwrap()
        .unwrap();
    assert_eq!(track.metadata().key, None);
    assert_eq!(track.metadata().color_label, None);
    assert_eq!(
        track.file().unwrap().path().file_name().unwrap(),
        "solo.mp3"
    );
}
