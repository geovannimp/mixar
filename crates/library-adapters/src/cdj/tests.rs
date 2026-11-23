//! Unit tests for the CDJ exporter orchestration.

use super::*;
use fallible_iterator::FallibleIterator;
use library_core::{
    AudioSource, Collection, CollectionConfig, CollectionEntry, CollectionId, ExportArtwork,
    ExportCue, ExportOptions, ExportSource, FileAudioSource, Library, TrackExportData, TrackId,
    TrackMetadata,
};
use rekordcrate::pdb::io::Database;
use rekordcrate::pdb::DatabaseType;

fn cue(hot_slot: Option<u8>, position_ms: u32, loop_end_ms: Option<u32>) -> ExportCue {
    ExportCue {
        hot_cue_slot: hot_slot,
        position_ms,
        loop_end_ms,
        loop_length_beats: None,
        label: None,
        color: None,
    }
}

#[test]
fn cues_split_into_hot_and_memory_lists_in_slot_order() {
    let cues = vec![
        cue(Some(3), 30_000, None),
        cue(None, 90_000, Some(96_000)),
        cue(Some(1), 10_000, None),
        cue(None, 5_000, None),
    ];
    let (hot, memory) = split_cues(&cues);
    assert_eq!(
        hot.iter().map(|cue| cue.hot_cue_slot).collect::<Vec<_>>(),
        vec![Some(1), Some(3)]
    );
    assert_eq!(
        memory.iter().map(|cue| cue.position_ms).collect::<Vec<_>>(),
        vec![5_000, 90_000]
    );
}

#[test]
fn relative_paths_lose_their_leading_slash() {
    assert_eq!(
        relative_path("/PIONEER/USBANLZ/P051/0001D603/ANLZ0000.DAT"),
        PathBuf::from("PIONEER/USBANLZ/P051/0001D603/ANLZ0000.DAT")
    );
    assert_eq!(
        relative_path("Contents/A/B.mp3"),
        PathBuf::from("Contents/A/B.mp3")
    );
}

#[test]
fn analysis_dir_matches_the_device_hash() {
    assert_eq!(
        CdjExporter::analysis_dir("/Contents/ARTISTTEST1/ALBUMTEST1/TITLETEST1.mp3"),
        "/PIONEER/USBANLZ/P051/0001D603"
    );
}

/// Library stub exposing file tracks plus one shared cover, for the artwork run.
struct ArtSource {
    tracks: Vec<(TrackId, PathBuf)>,
    collection: CollectionId,
    artwork: Option<Vec<u8>>,
}

impl ArtSource {
    fn new(dir: &Path, files: &[&str], artwork: Option<Vec<u8>>) -> Self {
        let tracks = files
            .iter()
            .map(|name| {
                let path = dir.join(name);
                std::fs::write(&path, b"audio bytes").expect("write fixture file");
                (TrackId::new(*name), path)
            })
            .collect();
        Self {
            tracks,
            collection: CollectionId::new("pl-1"),
            artwork,
        }
    }

    fn options(&self) -> ExportOptions {
        ExportOptions {
            playlists: vec![self.collection.clone()],
            folders: Vec::new(),
            refresh_files: false,
        }
    }
}

impl Library for ArtSource {
    fn name(&self) -> &'static str {
        "art-source"
    }

    fn get_track(&self, id: &TrackId) -> Result<Option<AudioSource>> {
        Ok(self
            .tracks
            .iter()
            .find(|(track_id, _)| track_id == id)
            .map(|(track_id, path)| {
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    path.clone(),
                    TrackMetadata::default(),
                ))
            }))
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        Ok(vec![Collection {
            id: self.collection.clone(),
            name: "Warmup".to_string(),
            config: CollectionConfig::Playlist { sortable: true },
        }])
    }

    fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>> {
        Ok(self
            .list_collections()?
            .into_iter()
            .find(|collection| &collection.id == id))
    }

    fn list_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<AudioSource>> {
        if collection_id != &self.collection {
            return Ok(Vec::new());
        }
        self.tracks
            .iter()
            .map(|(track_id, path)| {
                Ok(AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    path.clone(),
                    TrackMetadata::default(),
                )))
            })
            .collect()
    }

    fn list_collection_entries(
        &self,
        _collection_id: &CollectionId,
    ) -> Result<Vec<CollectionEntry>> {
        Ok(Vec::new())
    }
}

impl ExportSource for ArtSource {
    fn export_data(&self, _id: &TrackId) -> Result<Option<TrackExportData>> {
        Ok(Some(TrackExportData {
            artwork: self.artwork.clone().map(|jpeg| ExportArtwork { jpeg }),
            ..TrackExportData::default()
        }))
    }
}

/// Open the database this run wrote, as the parser sees it.
fn open_export_db(destination: &Path) -> Database<std::fs::File> {
    let path = destination.join(PDB_PATH);
    Database::open_non_persistent(
        std::fs::File::open(&path).expect("open export.pdb"),
        DatabaseType::Plain,
    )
    .expect("parse export.pdb")
}

fn artwork_rows(destination: &Path) -> Vec<serde_json::Value> {
    let mut db = open_export_db(destination);
    let mut rows = Vec::new();
    let mut iter = db
        .iter_rows::<rekordcrate::pdb::Artwork>()
        .expect("artwork rows");
    while let Some(row) = iter.next().expect("artwork row") {
        rows.push(serde_json::to_value(&*row).expect("artwork as json"));
    }
    rows
}

fn track_artwork_ids(destination: &Path) -> Vec<u64> {
    let mut db = open_export_db(destination);
    let mut ids = Vec::new();
    let mut iter = db
        .iter_rows::<rekordcrate::pdb::Track>()
        .expect("track rows");
    while let Some(row) = iter.next().expect("track row") {
        let json = serde_json::to_value(&*row).expect("track as json");
        ids.push(json["artwork_id"].as_u64().expect("artwork_id"));
    }
    ids
}

#[test]
fn one_cover_is_written_and_shared_by_every_track() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = ArtSource::new(
        dir.path(),
        &["one.wav", "two.wav"],
        Some(b"jpeg-bytes".to_vec()),
    );
    let drive = dir.path().join("drive");

    let report = CdjExporter
        .export(&source, &drive, &source.options())
        .expect("export");

    assert!(report.errors.is_empty(), "{:?}", report.errors);
    // The cover is written once and counted in the copied bytes.
    let artwork = drive.join("PIONEER/Artwork/00001/a1.jpg");
    assert_eq!(
        std::fs::read(&artwork).expect("artwork file"),
        b"jpeg-bytes"
    );
    assert_eq!(
        report.bytes_copied,
        ("audio bytes".len() * 2 + "jpeg-bytes".len()) as u64
    );

    let rows = artwork_rows(&drive);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["id"], serde_json::json!(1));
    assert_eq!(rows[0]["path"], "/PIONEER/Artwork/00001/a1.jpg");
    assert_eq!(track_artwork_ids(&drive), vec![1, 1]);

    // A forced rewrite still writes and counts the shared cover once.
    let refreshed = CdjExporter
        .export(
            &source,
            &drive,
            &ExportOptions {
                refresh_files: true,
                ..source.options()
            },
        )
        .expect("export again with refresh");
    assert_eq!(
        refreshed.bytes_copied,
        ("audio bytes".len() * 2 + "jpeg-bytes".len()) as u64
    );

    // A re-run reuses the file (nothing copied for artwork) and keeps one row.
    let again = CdjExporter
        .export(&source, &drive, &source.options())
        .expect("export again");
    assert_eq!(again.bytes_copied, 0);
    assert_eq!(artwork_rows(&drive).len(), 1);
}

#[test]
fn a_track_without_artwork_keeps_id_zero() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = ArtSource::new(dir.path(), &["one.wav"], None);
    let drive = dir.path().join("drive");

    CdjExporter
        .export(&source, &drive, &source.options())
        .expect("export");

    assert!(artwork_rows(&drive).is_empty());
    assert_eq!(track_artwork_ids(&drive), vec![0]);
    assert!(!drive.join("PIONEER/Artwork").exists());
}

#[test]
fn different_covers_get_their_own_ids() {
    let dir = tempfile::tempdir().expect("temp dir");
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    std::fs::create_dir_all(&first).expect("first dir");
    std::fs::create_dir_all(&second).expect("second dir");

    // Two sources, each with its own cover, exported into one drive in turn.
    let one = ArtSource::new(&first, &["one.wav"], Some(b"cover-one".to_vec()));
    let two = ArtSource::new(&second, &["two.wav"], Some(b"cover-two".to_vec()));
    let drive = dir.path().join("drive");

    CdjExporter
        .export(&one, &drive, &one.options())
        .expect("first");
    CdjExporter
        .export(&two, &drive, &two.options())
        .expect("second");

    // The second run merges into the drive, so its cover takes the next free id
    // instead of overwriting the first one's file or row.
    let rows = artwork_rows(&drive);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"].as_u64(), Some(1));
    assert_eq!(rows[1]["id"].as_u64(), Some(2));
    assert_eq!(
        std::fs::read(drive.join("PIONEER/Artwork/00001/a1.jpg")).expect("first cover file"),
        b"cover-one"
    );
    assert_eq!(
        std::fs::read(drive.join("PIONEER/Artwork/00002/a1.jpg")).expect("second cover file"),
        b"cover-two"
    );
    assert_eq!(
        track_artwork_ids(&drive),
        vec![1, 2],
        "each of the drive's tracks points at its own cover"
    );
}
