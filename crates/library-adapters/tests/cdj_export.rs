//! End-to-end CDJ USB export.
//!
//! A small library (two WAV tracks in a folder collection plus a playlist, a
//! beat grid, a hot cue and a saved loop) is exported into a temporary "drive",
//! then the device database and analysis files are read back with `rekordcrate`
//! — an independent parser of the same formats — to check that what a CDJ/XDJ
//! would read is what we meant to write.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use binrw::BinRead;
use fallible_iterator::FallibleIterator;
use library::LibraryManager;
use library_adapters::CdjExporter;
use library_core::{
    Collection, ExportOptions, ExportReport, Exportable, Library, LibraryConfig, NewCollection,
    TrackId, WritableLibrary,
};

use rekordcrate::anlz::{Content, CueListType, CueType, ANLZ};
use rekordcrate::pdb::io::Database;
use rekordcrate::pdb::{Artwork, DatabaseType, PlaylistEntry, PlaylistTreeNode, Track};

const HOT_CUE_MS: u32 = 12_000;
const LOOP_IN_MS: u32 = 20_000;
const LOOP_OUT_MS: u32 = 24_000;
const BPM: f64 = 128.0;

/// Write a 16-bit mono PCM WAV holding a 440 Hz sine.
fn write_wav(path: &Path, seconds: f32) {
    let sample_rate = 44_100u32;
    let frames = (f64::from(sample_rate) * f64::from(seconds)).round() as usize;
    let mut pcm = Vec::with_capacity(frames * 2);
    for frame in 0..frames {
        let value = (2.0 * std::f32::consts::PI * 440.0 * frame as f32 / sample_rate as f32).sin();
        pcm.extend_from_slice(&((value * 0.5 * f32::from(i16::MAX)) as i16).to_le_bytes());
    }

    let mut file = File::create(path).expect("create wav");
    let data_size = pcm.len() as u32;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
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

/// A 300×200 PNG cover, the shape a real file's tags carry.
fn cover_png() -> Vec<u8> {
    let rgb = image::RgbImage::from_fn(300, 200, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("encode cover");
    bytes
}

/// Append an ID3v2.3 `APIC` cover to a WAV through its `id3 ` chunk, which is
/// how tagged WAVs in a real library look.
fn write_cover_chunk(path: &Path, cover: &[u8]) {
    let mut payload = Vec::new();
    payload.push(0); // ISO-8859-1 text encoding
    payload.extend_from_slice(b"image/png\0");
    payload.push(0x03); // front cover
    payload.push(0); // empty description
    payload.extend_from_slice(cover);

    let mut frames = Vec::new();
    frames.extend_from_slice(b"APIC");
    frames.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frames.extend_from_slice(&[0, 0]); // frame flags
    frames.extend_from_slice(&payload);

    let mut tag = Vec::new();
    tag.extend_from_slice(b"ID3");
    tag.extend_from_slice(&[0x03, 0x00, 0x00]); // v2.3, no flags
    let size = frames.len() as u32;
    tag.extend_from_slice(&[
        ((size >> 21) & 0x7F) as u8,
        ((size >> 14) & 0x7F) as u8,
        ((size >> 7) & 0x7F) as u8,
        (size & 0x7F) as u8,
    ]);
    tag.extend_from_slice(&frames);

    let mut file = File::options().append(true).open(path).expect("reopen wav");
    file.write_all(b"id3 ").unwrap();
    file.write_all(&(tag.len() as u32).to_le_bytes()).unwrap();
    file.write_all(&tag).unwrap();
    if tag.len() % 2 == 1 {
        file.write_all(&[0]).unwrap(); // RIFF chunks are word-aligned
    }
}

struct Fixture {
    #[allow(dead_code)]
    dir: tempfile::TempDir,
    library: LibraryManager,
    folder: Collection,
    playlist: Collection,
}

impl Fixture {
    /// Build a library over `music/Artist/Album/{One,Two}.wav` with a folder
    /// collection, a sortable playlist holding both tracks in reverse scan
    /// order, a beat grid, a hot cue and a saved loop on the first track.
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let album = dir.path().join("music").join("Artist").join("Album");
        fs::create_dir_all(&album).expect("create music dir");
        write_wav(&album.join("One.wav"), 5.0);
        write_cover_chunk(&album.join("One.wav"), &cover_png());
        write_wav(&album.join("Two.wav"), 2.0);

        let mut library =
            LibraryManager::open_in_memory(LibraryConfig::default()).expect("library");
        let folder = library
            .add_collection(&NewCollection::folder(dir.path().join("music")))
            .expect("folder collection");
        library.sync_collection(None).expect("scan folder");
        let playlist = library
            .add_collection(&NewCollection::playlist("Warmup", true))
            .expect("playlist");

        let sources = library
            .list_collection_tracks(&folder.id)
            .expect("folder tracks");
        assert_eq!(sources.len(), 2, "both wavs imported");
        let track_ids: Vec<TrackId> = sources.iter().map(|source| source.id().clone()).collect();
        // Reverse order proves playlist order is preserved (not sorted by path);
        // `None` appends, so the first insert takes position 0.
        for id in track_ids.iter().rev() {
            library
                .add_collection_entry(&playlist.id, id, None)
                .expect("add playlist entry");
            library
                .save_track_beat_grid(id, BPM, 0.0)
                .expect("beat grid");
        }
        library
            .save_track_hot_cue(
                &track_ids[0],
                0,
                HOT_CUE_MS as i32,
                None,
                Some("#FF0000".to_string()),
                Some("Drop".to_string()),
            )
            .expect("hot cue");
        library
            .save_track_loop(
                &track_ids[0],
                0,
                LOOP_IN_MS as i32,
                LOOP_OUT_MS as i32,
                Some("Break".to_string()),
                None,
            )
            .expect("saved loop");

        Self {
            dir,
            library,
            folder,
            playlist,
        }
    }

    /// Folder collection display name (the drive mirrors it as a directory).
    fn folder_name(&self) -> &str {
        &self.folder.name
    }

    fn options(&self) -> ExportOptions {
        ExportOptions {
            playlists: vec![self.playlist.id.clone()],
            folders: vec![self.folder.id.clone()],
            refresh_files: false,
        }
    }

    fn export(&self, destination: &Path, options: &ExportOptions) -> ExportReport {
        CdjExporter
            .export(&self.library, destination, options)
            .expect("export succeeds")
    }

    /// On-drive path of a mirrored folder track, without the leading slash.
    fn device_path(&self, file_name: &str) -> String {
        format!("Contents/{}/Artist/Album/{file_name}", self.folder_name())
    }
}

fn open_pdb(destination: &Path) -> Database<File> {
    let path = destination.join("PIONEER/rekordbox/export.pdb");
    Database::open_non_persistent(
        File::open(&path).expect("open export.pdb"),
        DatabaseType::Plain,
    )
    .expect("parse export.pdb")
}

fn read_anlz(destination: &Path, device_audio_path: &str, extension: &str) -> ANLZ {
    let dir = CdjExporter::analysis_dir(device_audio_path);
    let path = destination.join(format!(
        "{}/ANLZ0000.{extension}",
        dir.trim_start_matches('/')
    ));
    let mut file = File::open(&path).unwrap_or_else(|err| panic!("open {}: {err}", path.display()));
    ANLZ::read(&mut file).expect("parse ANLZ")
}

fn section_names(anlz: &ANLZ) -> Vec<&'static str> {
    anlz.sections
        .iter()
        .map(|section| match &section.content {
            Content::Path(_) => "PPTH",
            Content::VBR(_) => "PVBR",
            Content::BeatGrid(_) => "PQTZ",
            Content::WaveformPreview(_) => "PWAV",
            Content::TinyWaveformPreview(_) => "PWV2",
            Content::WaveformDetail(_) => "PWV3",
            Content::WaveformColorPreview(_) => "PWV4",
            Content::WaveformColorDetail(_) => "PWV5",
            Content::CueList(_) => "PCOB",
            Content::ExtendedCueList(_) => "PCO2",
            _ => "other",
        })
        .collect()
}

#[test]
fn writes_drive_layout_database_and_analysis() {
    let fixture = Fixture::new();
    let destination = fixture.dir.path().join("drive");
    let report = fixture.export(&destination, &fixture.options());

    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert_eq!(report.tracks, 2);
    assert_eq!(report.tracks_reused, 0);
    // The selected playlist plus the folder collection's own playlist.
    assert_eq!(report.playlists, 2);
    assert_eq!(report.folders, 1);
    assert!(report.bytes_copied > 0);

    for file_name in ["One.wav", "Two.wav"] {
        let path = destination.join(fixture.device_path(file_name));
        assert!(path.is_file(), "missing {}", path.display());
    }
    // Folder-track placement wins over the metadata layout, so nothing is
    // duplicated under the artist/album tree.
    assert!(!destination.join("Contents/Artist/Album/One.wav").exists());

    check_database(&destination, &fixture);
    check_artwork(&destination);
    check_dat(&destination, &fixture);
    check_ext(&destination, &fixture);
}

/// A track row as reported by the parser, reduced to what we assert on.
#[derive(Debug)]
struct Row {
    id: u32,
    title: String,
    file_path: String,
    analyze_path: String,
    date_added: String,
    artwork_id: u64,
}

fn check_database(destination: &Path, fixture: &Fixture) {
    let mut db = open_pdb(destination);
    {
        let header = db.get_header();
        assert_eq!(header.page_size, 4096);
        assert_eq!(header.tables.len(), 20);
    }

    let mut rows: Vec<Row> = Vec::new();
    let mut one_json = None;
    {
        let mut tracks = db.iter_rows::<Track>().expect("track rows");
        while let Some(track) = tracks.next().expect("track row") {
            let json = serde_json::to_value(&*track).expect("track as json");
            let strings = &json["offsets"]["inner"];
            let as_string =
                |value: &serde_json::Value| value.as_str().unwrap_or_default().to_string();
            let title = track.offsets.title.to_string();
            if title == "One" {
                one_json = Some(json.clone());
            }
            rows.push(Row {
                id: track.id.0,
                title,
                file_path: track.offsets.file_path.to_string(),
                analyze_path: as_string(&strings["analyze_path"]),
                date_added: as_string(&strings["date_added"]),
                artwork_id: json["artwork_id"].as_u64().unwrap_or_default(),
            });
        }
    }

    assert_eq!(rows.len(), 2);
    let mut titles: Vec<&str> = rows.iter().map(|row| row.title.as_str()).collect();
    titles.sort_unstable();
    assert_eq!(titles, vec!["One", "Two"]);
    for row in &rows {
        assert_eq!(
            row.file_path,
            format!("/{}", fixture.device_path(&format!("{}.wav", row.title)))
        );
        assert!(row.analyze_path.ends_with("/ANLZ0000.DAT"), "{row:?}");
        assert!(!row.date_added.is_empty());
    }

    // Numeric fields are only reachable through the parser's serde view.
    let one = one_json.expect("track One");
    assert_eq!(one["tempo"], serde_json::json!(12_800));
    assert_eq!(one["sample_rate"], serde_json::json!(44_100));
    assert_eq!(one["sample_depth"], serde_json::json!(16));
    assert_eq!(one["duration"], serde_json::json!(5));
    assert_eq!(one["key_id"], serde_json::json!(0));
    assert!(one["file_size"].as_u64().unwrap_or(0) > 44);

    let id_of = |name: &str| {
        rows.iter()
            .find(|row| row.title == name)
            .expect("track row")
            .id
    };

    let mut nodes: Vec<(u32, String)> = Vec::new();
    {
        let mut tree = db.iter_rows::<PlaylistTreeNode>().expect("playlist tree");
        while let Some(node) = tree.next().expect("playlist node") {
            nodes.push((node.id.0, node.name.to_string()));
        }
    }
    let mut names: Vec<&str> = nodes.iter().map(|(_, name)| name.as_str()).collect();
    names.sort_unstable();
    let mut expected = vec!["Warmup", fixture.folder_name()];
    expected.sort_unstable();
    assert_eq!(names, expected);

    let mut entries: Vec<(u32, u32, u32)> = Vec::new();
    {
        let mut rows = db.iter_rows::<PlaylistEntry>().expect("playlist entries");
        while let Some(entry) = rows.next().expect("playlist entry") {
            entries.push((entry.playlist_id.0, entry.entry_index, entry.track_id.0));
        }
    }
    let order = |playlist_name: &str| {
        let (playlist_id, _) = nodes
            .iter()
            .find(|(_, name)| name == playlist_name)
            .expect("playlist node");
        let mut rows: Vec<(u32, u32)> = entries
            .iter()
            .filter(|(id, _, _)| id == playlist_id)
            .map(|(_, entry_index, track_id)| (*entry_index, *track_id))
            .collect();
        rows.sort_unstable();
        rows
    };

    // The playlist was filled in reverse folder order; the folder's own
    // playlist follows the plan order (scan order).
    assert_eq!(order("Warmup"), vec![(1, id_of("Two")), (2, id_of("One"))]);
    assert_eq!(
        order(fixture.folder_name()),
        vec![(1, id_of("One")), (2, id_of("Two"))]
    );

    // Only the covered track references a cover; the bare one keeps id 0.
    let artwork = |title: &str| {
        rows.iter()
            .find(|row| row.title == title)
            .expect("track row")
            .artwork_id
    };
    assert_eq!(artwork("One"), 1);
    assert_eq!(artwork("Two"), 0);

    let mut artworks = db.iter_rows::<Artwork>().expect("artwork rows");
    let mut rows_written = Vec::new();
    while let Some(row) = artworks.next().expect("artwork row") {
        rows_written.push(serde_json::to_value(&*row).expect("artwork as json"));
    }
    assert_eq!(rows_written.len(), 1, "{rows_written:?}");
    assert_eq!(rows_written[0]["id"], serde_json::json!(1));
    assert_eq!(rows_written[0]["path"], "/PIONEER/Artwork/00001/a1.jpg");
}

/// The cover the exporter writes is the embedded one, scaled to the 80 px the
/// device database expects, and written once per distinct cover.
fn check_artwork(destination: &Path) {
    let path = destination.join("PIONEER/Artwork/00001/a1.jpg");
    let bytes = fs::read(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    assert_eq!(bytes[..2], [0xFF, 0xD8], "thumbnails are JPEG");

    let decoded = image::load_from_memory(&bytes).expect("decode thumbnail");
    assert_eq!((decoded.width(), decoded.height()), (80, 53));
}

fn check_dat(destination: &Path, fixture: &Fixture) {
    for (file_name, track_seconds) in [("One.wav", 5u32), ("Two.wav", 2)] {
        let device_audio_path = format!("/{}", fixture.device_path(file_name));
        let dat = read_anlz(destination, &device_audio_path, "DAT");
        assert_eq!(
            section_names(&dat),
            vec!["PPTH", "PVBR", "PQTZ", "PWAV", "PWV2", "PCOB", "PCOB"],
            "sections for {file_name}"
        );

        let mut hot = Vec::new();
        let mut memory = Vec::new();
        for section in &dat.sections {
            match &section.content {
                Content::Path(path) => {
                    assert_eq!(path.path.to_string(), device_audio_path)
                }
                Content::BeatGrid(grid) => {
                    assert!(
                        grid.beats.len() as u32 > track_seconds,
                        "grid covers the track"
                    );
                    assert_eq!(grid.beats[0].time, 0);
                    assert_eq!(grid.beats[0].tempo, (BPM * 100.0) as u16);
                    assert_eq!(grid.beats[0].beat_number, 1);
                    assert_eq!(grid.beats[1].beat_number, 2);
                }
                Content::WaveformPreview(preview) => {
                    assert_eq!(preview.data.len(), 400);
                    assert!(preview.data.iter().any(|column| column.height() > 0));
                }
                Content::TinyWaveformPreview(preview) => assert_eq!(preview.data.len(), 100),
                Content::VBR(_) => {}
                Content::CueList(list) => {
                    let target = match list.list_type {
                        CueListType::HotCues => &mut hot,
                        CueListType::MemoryCues => &mut memory,
                    };
                    for cue in &list.cues {
                        let kind = match &cue.cue_type {
                            CueType::Point => "point",
                            CueType::Loop => "loop",
                        };
                        target.push((cue.hot_cue, cue.time, cue.loop_time, kind));
                    }
                }
                other => panic!("unexpected .DAT section {other:?}"),
            }
        }

        if file_name == "One.wav" {
            assert_eq!(hot, vec![(1, HOT_CUE_MS, 0, "point")]);
            assert_eq!(memory, vec![(0, LOOP_IN_MS, LOOP_OUT_MS, "loop")]);
        } else {
            assert!(hot.is_empty(), "only the first track has cues");
            assert!(memory.is_empty());
        }
    }
}

/// One extended cue as reported by the parser.
struct ExtendedRow {
    hot_cue: u32,
    time: u32,
    loop_time: u32,
    cue_type: &'static str,
    comment: String,
    color_rgb: (u8, u8, u8),
    loop_numerator: u16,
    loop_denominator: u16,
    trailing: usize,
}

fn check_ext(destination: &Path, fixture: &Fixture) {
    for (file_name, track_seconds) in [("One.wav", 5u32), ("Two.wav", 2)] {
        let device_audio_path = format!("/{}", fixture.device_path(file_name));
        let ext = read_anlz(destination, &device_audio_path, "EXT");
        assert_eq!(
            section_names(&ext),
            vec!["PPTH", "PWV3", "PCOB", "PCOB", "PCO2", "PCO2", "PWV5", "PWV4"],
            "sections for {file_name}"
        );

        let expected_detail = (track_seconds * 150) as usize;
        let mut detail = None;
        let mut color_detail = None;
        let mut color_preview = None;
        let mut plain_hot = Vec::new();
        let mut plain_memory = Vec::new();
        let mut extended_hot = Vec::new();
        let mut extended_memory = Vec::new();
        for section in &ext.sections {
            match &section.content {
                Content::Path(path) => assert_eq!(path.path.to_string(), device_audio_path),
                Content::WaveformDetail(data) => detail = Some(data.data.len()),
                Content::WaveformColorDetail(data) => color_detail = Some(data.data.len()),
                Content::WaveformColorPreview(data) => color_preview = Some(data.data.len()),
                // The .EXT repeats the plain cue lists the .DAT carries.
                Content::CueList(list) => {
                    let target = match list.list_type {
                        CueListType::HotCues => &mut plain_hot,
                        CueListType::MemoryCues => &mut plain_memory,
                    };
                    for cue in &list.cues {
                        target.push((cue.hot_cue, cue.time, cue.loop_time));
                    }
                }
                Content::ExtendedCueList(list) => {
                    let target = match list.list_type {
                        CueListType::HotCues => &mut extended_hot,
                        CueListType::MemoryCues => &mut extended_memory,
                    };
                    for cue in &list.cues {
                        target.push(ExtendedRow {
                            hot_cue: cue.hot_cue,
                            time: cue.time,
                            loop_time: cue.loop_time,
                            cue_type: match &cue.cue_type {
                                CueType::Point => "point",
                                CueType::Loop => "loop",
                            },
                            comment: cue.comment.to_string(),
                            color_rgb: cue.hot_cue_color_rgb,
                            loop_numerator: cue.loop_numerator,
                            loop_denominator: cue.loop_denominator,
                            trailing: cue.trailing.len(),
                        });
                    }
                }
                other => panic!("unexpected .EXT section {other:?}"),
            }
        }
        assert_eq!(
            detail,
            Some(expected_detail),
            "detail entries for {file_name}"
        );
        assert_eq!(color_detail, Some(expected_detail));
        assert_eq!(color_preview, Some(1200));

        if file_name == "One.wav" {
            assert_eq!(plain_hot, vec![(1, HOT_CUE_MS, 0)]);
            assert_eq!(plain_memory, vec![(0, LOOP_IN_MS, LOOP_OUT_MS)]);
            assert_eq!(extended_hot.len(), 1);
            let cue = &extended_hot[0];
            assert_eq!(cue.hot_cue, 1);
            assert_eq!(cue.time, HOT_CUE_MS);
            assert_eq!(cue.cue_type, "point");
            assert_eq!(cue.comment, "Drop");
            assert_eq!(cue.color_rgb, (0xFF, 0x00, 0x00));
            // Real Rekordbox exports carry 20 unknown trailing bytes; we write
            // them zeroed so the entry size stays byte-compatible.
            assert_eq!(cue.trailing, 20);

            assert_eq!(extended_memory.len(), 1);
            let loop_cue = &extended_memory[0];
            assert_eq!(loop_cue.hot_cue, 0);
            assert_eq!(loop_cue.time, LOOP_IN_MS);
            assert_eq!(loop_cue.loop_time, LOOP_OUT_MS);
            assert_eq!(loop_cue.cue_type, "loop");
            assert_eq!(loop_cue.loop_numerator, 0);
            assert_eq!(loop_cue.loop_denominator, 0);
        } else {
            assert!(extended_hot.is_empty());
            assert!(extended_memory.is_empty());
        }
    }
}

#[test]
fn reuses_files_already_on_the_drive() {
    let fixture = Fixture::new();
    let destination = fixture.dir.path().join("drive");
    let options = fixture.options();

    assert_eq!(fixture.export(&destination, &options).tracks_reused, 0);

    let second = fixture.export(&destination, &options);
    assert_eq!(second.tracks, 2);
    assert_eq!(second.tracks_reused, 2);
    assert_eq!(second.bytes_copied, 0);

    let refreshed = fixture.export(
        &destination,
        &ExportOptions {
            refresh_files: true,
            ..options
        },
    );
    assert_eq!(refreshed.tracks_reused, 0);
    assert!(refreshed.bytes_copied > 0);
}

#[test]
fn empty_selection_still_writes_a_valid_database() {
    let fixture = Fixture::new();
    let destination = fixture.dir.path().join("empty-drive");
    let report = fixture.export(&destination, &ExportOptions::default());

    assert_eq!(report.tracks, 0);
    assert_eq!(report.playlists, 0);

    let mut db = open_pdb(&destination);
    assert_eq!(db.iter_rows::<Track>().expect("tracks").count().unwrap(), 0);
    assert_eq!(
        db.iter_rows::<PlaylistTreeNode>()
            .expect("playlist tree")
            .count()
            .unwrap(),
        0
    );
}

#[test]
fn reports_tracks_whose_file_disappeared() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.dir.path().join("music/Artist/Album/Two.wav"))
        .expect("remove track file");

    let destination = fixture.dir.path().join("drive");
    let report = fixture.export(&destination, &fixture.options());

    assert_eq!(report.tracks, 1);
    assert_eq!(report.errors.len(), 1, "errors: {:?}", report.errors);
    assert!(report.errors[0].contains("Two.wav"), "{:?}", report.errors);

    let mut db = open_pdb(&destination);
    assert_eq!(db.iter_rows::<Track>().expect("tracks").count().unwrap(), 1);
}

/// A library large enough that the track and playlist-entry tables span
/// several pages, plus a playlist listing every track.
#[test]
fn exports_a_library_spanning_several_pages() {
    let dir = tempfile::tempdir().expect("temp dir");
    let music = dir.path().join("music");
    fs::create_dir_all(&music).expect("create music dir");
    for index in 0..40 {
        write_wav(&music.join(format!("Track {index:02}.wav")), 1.0);
    }

    let mut library = LibraryManager::open_in_memory(LibraryConfig::default()).expect("library");
    let folder = library
        .add_collection(&NewCollection::folder(&music))
        .expect("folder collection");
    library.sync_collection(None).expect("scan folder");
    let playlist = library
        .add_collection(&NewCollection::playlist("All", true))
        .expect("playlist");
    let sources = library
        .list_collection_tracks(&folder.id)
        .expect("folder tracks");
    assert_eq!(sources.len(), 40);
    for source in &sources {
        library
            .add_collection_entry(&playlist.id, source.id(), None)
            .expect("add playlist entry");
    }

    let destination = dir.path().join("drive");
    let report = CdjExporter
        .export(
            &library,
            &destination,
            &ExportOptions {
                playlists: vec![playlist.id.clone()],
                folders: vec![folder.id.clone()],
                refresh_files: false,
            },
        )
        .expect("export succeeds");

    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert_eq!(report.tracks, 40);
    // The selected playlist plus the folder collection's own playlist.
    assert_eq!(report.playlists, 2);

    let mut db = open_pdb(&destination);
    let mut tracks = db.iter_rows::<Track>().expect("tracks");
    let mut titles = Vec::new();
    while let Some(track) = tracks.next().expect("track row") {
        titles.push(track.offsets.title.to_string());
        assert!(
            !track.offsets.title.to_string().is_empty(),
            "every track row carries a title"
        );
    }
    drop(tracks);
    assert_eq!(titles.len(), 40);

    let mut entries = db.iter_rows::<PlaylistEntry>().expect("playlist entries");
    let mut entry_count = 0;
    while entries.next().expect("playlist entry").is_some() {
        entry_count += 1;
    }
    assert_eq!(entry_count, 80, "both playlists list every track");

    // Each track got its own analysis files at the hash-derived directory.
    for index in 0..40 {
        let device = format!("/Contents/{}/Track {index:02}.wav", folder.name);
        for extension in ["DAT", "EXT"] {
            let path = destination.join(format!(
                "{}/ANLZ0000.{extension}",
                CdjExporter::analysis_dir(&device).trim_start_matches('/')
            ));
            assert!(path.is_file(), "missing {}", path.display());
        }
    }
}

/// Playlist names, in the order the parser walks the tree table.
fn playlist_names(destination: &Path) -> Vec<String> {
    let mut db = open_pdb(destination);
    let mut rows = db.iter_rows::<PlaylistTreeNode>().expect("playlist tree");
    let mut names = Vec::new();
    while let Some(node) = rows.next().expect("playlist node") {
        names.push(node.name.to_string());
    }
    names
}

/// Number of playlist rows across every playlist on the drive.
fn entry_count(destination: &Path) -> usize {
    let mut db = open_pdb(destination);
    db.iter_rows::<PlaylistEntry>()
        .expect("playlist entries")
        .count()
        .expect("count entries")
}

#[test]
fn a_second_export_merges_with_what_the_drive_holds() {
    let fixture = Fixture::new();
    let destination = fixture.dir.path().join("drive");

    // The UI exports one collection at a time, so a run has to add to the drive
    // rather than replace what earlier runs wrote.
    fixture.export(
        &destination,
        &ExportOptions {
            playlists: Vec::new(),
            folders: vec![fixture.folder.id.clone()],
            refresh_files: false,
        },
    );
    let second = fixture.export(
        &destination,
        &ExportOptions {
            playlists: vec![fixture.playlist.id.clone()],
            folders: Vec::new(),
            refresh_files: false,
        },
    );
    assert!(second.warnings.is_empty(), "{:?}", second.warnings);
    assert_eq!(second.tracks, 2);

    let mut db = open_pdb(&destination);
    assert_eq!(
        db.iter_rows::<Track>()
            .expect("tracks")
            .count()
            .expect("count tracks"),
        2,
        "the folder's tracks survive the playlist export"
    );
    drop(db);

    assert_eq!(playlist_names(&destination), vec!["music", "Warmup"]);
    // The playlist-only run reuses the folder placement instead of duplicating
    // the files under the artist/album tree.
    assert!(destination.join(fixture.device_path("One.wav")).is_file());
    assert!(!destination.join("Contents/Artist/Album/One.wav").exists());
}

#[test]
fn re_exporting_a_collection_updates_it_without_dropping_others() {
    let mut fixture = Fixture::new();
    let destination = fixture.dir.path().join("drive");

    fixture.export(&destination, &fixture.options());
    assert_eq!(
        entry_count(&destination),
        4,
        "both playlists list both tracks"
    );

    // Dropping a playlist entry and re-exporting only that playlist rewrites its
    // rows; the folder's own playlist and the tracks stay.
    let entry = fixture
        .library
        .list_collection_entries(&fixture.playlist.id)
        .expect("playlist entries")[0]
        .id
        .clone();
    fixture
        .library
        .remove_collection_entry(&fixture.playlist.id, &entry)
        .expect("remove entry");
    let report = fixture.export(
        &destination,
        &ExportOptions {
            playlists: vec![fixture.playlist.id.clone()],
            folders: Vec::new(),
            refresh_files: false,
        },
    );

    assert_eq!(
        report.tracks, 1,
        "the run writes the one track the playlist lists"
    );
    let mut db = open_pdb(&destination);
    assert_eq!(
        db.iter_rows::<Track>()
            .expect("tracks")
            .count()
            .expect("count tracks"),
        2,
        "the track the playlist dropped is still on the drive"
    );
    drop(db);
    // The first run wrote the selected playlist and then the folder's own
    // playlist; rewriting one keeps the order the drive already had.
    assert_eq!(playlist_names(&destination), vec!["Warmup", "music"]);
    assert_eq!(
        entry_count(&destination),
        3,
        "the rewritten playlist has one row fewer"
    );
}

#[test]
fn a_drive_changed_outside_mixar_is_rebuilt() {
    let fixture = Fixture::new();
    let destination = fixture.dir.path().join("drive");

    fixture.export(&destination, &fixture.options());

    // Something else rewrote the database (Rekordbox, another tool), so the
    // state file no longer describes it and its playlists must not come back.
    fs::write(
        destination.join("PIONEER/rekordbox/export.pdb"),
        b"not ours",
    )
    .expect("clobber export.pdb");
    let report = fixture.export(
        &destination,
        &ExportOptions {
            playlists: vec![fixture.playlist.id.clone()],
            folders: Vec::new(),
            refresh_files: false,
        },
    );

    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("changed outside Mixar")),
        "warnings: {:?}",
        report.warnings
    );
    assert_eq!(
        playlist_names(&destination),
        vec!["Warmup"],
        "the folder's playlist went with the database it was in"
    );
    assert_eq!(entry_count(&destination), 2);
}
