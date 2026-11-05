//! Rekordbox import through the host `LibraryTransport` bridge.
//!
//! The fixture is a real SQLCipher-encrypted database keyed with the passphrase
//! the adapter ships, so this exercises the same `open` path production uses.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::path::{Path, PathBuf};

use library_adapters::rekordbox::DATABASE_KEY;
use rusqlite::{params, Connection};

fn build_master_db(dir: &Path) -> PathBuf {
    // One file exists, one does not: the import must count the difference.
    std::fs::write(dir.join("ivy.mp3"), b"").unwrap();

    let db_path = dir.join("master.db");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(&format!("PRAGMA key = '{DATABASE_KEY}';"))
        .unwrap();
    conn.execute_batch(
        "CREATE TABLE djmdArtist (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdAlbum (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdGenre (ID TEXT PRIMARY KEY, Name TEXT);
        CREATE TABLE djmdKey (ID TEXT PRIMARY KEY, ScaleName TEXT);
        CREATE TABLE djmdColor (ID TEXT PRIMARY KEY, ColorCode INTEGER, Commnt TEXT);
        CREATE TABLE djmdContent (
            ID TEXT PRIMARY KEY, Title TEXT, ArtistID TEXT, AlbumID TEXT, GenreID TEXT,
            KeyID TEXT, ColorID TEXT, BPM INTEGER, Length INTEGER, SampleRate INTEGER,
            BitRate INTEGER, Rating INTEGER, Commnt TEXT, ReleaseYear INTEGER,
            DiscNo INTEGER, TrackNo INTEGER, ISRC TEXT, FolderPath TEXT, FileNameL TEXT,
            FileNameS TEXT
        );
        CREATE TABLE djmdCue (
            ID TEXT PRIMARY KEY, ContentID TEXT, InMsec INTEGER, OutMsec INTEGER,
            Kind INTEGER, Color INTEGER, Comment TEXT
        );
        CREATE TABLE djmdPlaylist (
            ID TEXT PRIMARY KEY, Name TEXT, Attribute INTEGER, ParentID TEXT, Seq INTEGER
        );
        CREATE TABLE djmdSongPlaylist (
            ID TEXT PRIMARY KEY, PlaylistID TEXT, ContentID TEXT, TrackNo INTEGER
        );
        INSERT INTO djmdArtist (ID, Name) VALUES ('a1', 'Frank Ocean');
        INSERT INTO djmdKey (ID, ScaleName) VALUES ('k1', '8A');
        INSERT INTO djmdPlaylist (ID, Name, Attribute, ParentID)
            VALUES ('p1', 'Warmup', 0, '');",
    )
    .unwrap();

    for (id, title, file) in [("t1", "Ivy", "ivy.mp3"), ("t2", "Nikes", "nikes.flac")] {
        // `FolderPath` holds the full file path in Rekordbox 6/7.
        conn.execute(
            "INSERT INTO djmdContent (ID, Title, ArtistID, KeyID, BPM, Length, FolderPath, FileNameL)
             VALUES (?1, ?2, 'a1', 'k1', 128, 250, ?3, ?4)",
            params![id, title, dir.join(file).to_string_lossy(), file],
        )
        .unwrap();
    }
    conn.execute_batch(
        "INSERT INTO djmdSongPlaylist (ID, PlaylistID, ContentID, TrackNo)
         VALUES ('s1', 'p1', 't2', 1), ('s2', 'p1', 't1', 2);
         INSERT INTO djmdCue (ID, ContentID, InMsec, OutMsec, Kind, Color, Comment)
         VALUES ('q1', 't1', 10000, -1, 1, 2, 'Intro');",
    )
    .unwrap();

    db_path
}

#[test]
fn import_rekordbox_library_through_transport() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = build_master_db(dir.path());

    let transport = host_flutter::testing::open_library_in_memory().unwrap();

    let preview = host_flutter::api::library::rekordbox_import_preview(
        db_path.to_string_lossy().into_owned(),
    )
    .unwrap();
    assert_eq!(preview.track_count, 2);
    assert_eq!(preview.missing_file_count, 1);
    assert_eq!(preview.playlist_count, 1);
    assert_eq!(preview.hot_cue_count, 1);
    assert_eq!(preview.loop_count, 0);

    let report = transport
        .import_rekordbox_library(db_path.to_string_lossy().into_owned())
        .unwrap();
    assert_eq!(report.tracks_added, 2);
    assert_eq!(report.tracks_missing_files, 1);
    assert_eq!(report.playlists_imported, 1);
    assert_eq!(report.hot_cues_imported, 1);
    assert_eq!(report.loops_imported, 0);
    assert_eq!(report.failed, 0, "errors: {:?}", report.errors);

    let collections = transport.list_collections().unwrap();
    let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Warmup"), "{names:?}");
    assert!(names.contains(&"Rekordbox"), "{names:?}");

    let warmup = collections.iter().find(|c| c.name == "Warmup").unwrap();
    let entries = transport
        .list_collection_entries(warmup.id.clone())
        .unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].title.as_deref(), Some("Nikes"));
}

#[test]
fn preview_a_rekordbox_5_database_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("datafile.edb");
    std::fs::write(&path, b"not sqlite").unwrap();

    let error =
        host_flutter::api::library::rekordbox_import_preview(path.to_string_lossy().into_owned())
            .unwrap_err();
    assert!(error.contains("Rekordbox 5"), "{error}");
}
