//! Mixxx import through the host `LibraryTransport` bridge.

use std::path::{Path, PathBuf};

use host_flutter::api::library::LibraryTransport;
use rusqlite::{params, Connection};

fn build_mixxx_db(dir: &Path) -> (PathBuf, PathBuf) {
    let present = dir.join("present.mp3");
    std::fs::write(&present, b"").unwrap();
    let missing = dir.join("missing.flac");

    let db_path = dir.join("mixxxdb.sqlite");
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
            artist varchar(64), title varchar(64), album varchar(64), genre varchar(64),
            duration FLOAT, bitrate INTEGER, samplerate INTEGER, bpm FLOAT, channels INTEGER,
            replaygain FLOAT, key varchar(16), location INTEGER, mixxx_deleted INTEGER DEFAULT 0
        );
        CREATE TABLE Playlists (
            id INTEGER PRIMARY KEY, name varchar(48), position INTEGER,
            hidden INTEGER DEFAULT 0 NOT NULL, date_created datetime, date_modified datetime
        );
        CREATE TABLE PlaylistTracks (
            id INTEGER PRIMARY KEY, playlist_id INTEGER, track_id INTEGER, position INTEGER
        );
        CREATE TABLE crates (
            id INTEGER PRIMARY KEY AUTOINCREMENT, name varchar(48) UNIQUE NOT NULL,
            count INTEGER DEFAULT 0, show INTEGER DEFAULT 1
        );
        CREATE TABLE crate_tracks (
            crate_id INTEGER NOT NULL, track_id INTEGER NOT NULL, UNIQUE (crate_id, track_id)
        );
        CREATE TABLE directories (
            id integer primary key, directory varchar(512) unique,
            source integer default 0, type integer default 0
        );",
    )
    .unwrap();

    for (id, path) in [(1i64, &present), (2i64, &missing)] {
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
    }
    conn.execute(
        "INSERT INTO library (id, artist, title, bpm, key, duration, location, mixxx_deleted)
         VALUES (1, 'Artist A', 'Present', 128.0, '8A', 210.5, 1, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO library (id, artist, title, bpm, key, duration, location, mixxx_deleted)
         VALUES (2, NULL, 'Missing', 0.0, 'C', 0.0, 2, 0)",
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
    conn.execute("INSERT INTO crates (id, name) VALUES (1, 'Favs')", [])
        .unwrap();
    conn.execute(
        "INSERT INTO crate_tracks (crate_id, track_id) VALUES (1, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO directories (id, directory) VALUES (1, ?1)",
        params![dir.to_string_lossy()],
    )
    .unwrap();

    (db_path, present)
}

#[test]
fn import_mixxx_library_through_transport() {
    let dir = tempfile::tempdir().unwrap();
    let (db_path, _present) = build_mixxx_db(dir.path());

    let transport = LibraryTransport::open_in_memory().unwrap();

    let preview =
        host_flutter::api::library::mixxx_import_preview(db_path.to_string_lossy().into_owned())
            .unwrap();
    assert_eq!(preview.track_count, 2);
    assert_eq!(preview.missing_file_count, 1);
    assert_eq!(preview.playlist_count, 1);
    assert_eq!(preview.crate_count, 1);
    assert_eq!(preview.folder_count, 1);

    let report = transport
        .import_mixxx_library(db_path.to_string_lossy().into_owned())
        .unwrap();
    assert_eq!(report.tracks_added, 2);
    assert_eq!(report.tracks_missing_files, 1);
    assert_eq!(report.playlists_imported, 2);
    assert_eq!(report.crates_imported, 1);
    assert_eq!(report.failed, 0, "errors: {:?}", report.errors);

    let collections = transport.list_collections().unwrap();
    let names: Vec<&str> = collections.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Warmup"));
    assert!(names.contains(&"Favs"));
    assert!(!names.contains(&"AutoDJ"));
}
