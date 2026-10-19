use super::*;
#[cfg(feature = "analysis")]
use crate::analysis::{compute_file_analysis, preferred_loudness_lufs};
use std::io::Write;

fn write_minimal_wav(path: &Path) {
    let mut file = std::fs::File::create(path).unwrap();
    let data_size: u32 = 16;
    let file_size: u32 = 36 + data_size;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&8000u32.to_le_bytes()).unwrap();
    file.write_all(&8000u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&8u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&[0u8; 16]).unwrap();
}

/// Mono 16-bit PCM sine (~3s @ 48 kHz) so EBU R128 loudness stays finite.
fn write_analysis_wav(path: &Path) {
    let sample_rate = 48_000u32;
    let duration_secs = 3u32;
    let sample_count = (sample_rate * duration_secs) as usize;
    let mut pcm = Vec::with_capacity(sample_count * 2);
    for index in 0..sample_count {
        let time = index as f32 / sample_rate as f32;
        let sample =
            (0.25 * (2.0 * std::f32::consts::PI * 440.0 * time).sin() * i16::MAX as f32) as i16;
        pcm.extend_from_slice(&sample.to_le_bytes());
    }
    let data_size = pcm.len() as u32;
    let file_size = 36 + data_size;
    let byte_rate = sample_rate * 2;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    file.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap(); // block align
    file.write_all(&16u16.to_le_bytes()).unwrap(); // bits
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&pcm).unwrap();
}

#[test]
fn import_and_get_track_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_minimal_wav(&wav);

    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();

    assert_eq!(track.metadata().title.as_deref(), Some("track"));
    let fetched = lib.get_track(track.id()).unwrap().unwrap();
    assert_eq!(fetched.file().unwrap().path(), track.file().unwrap().path());
}

#[test]
fn import_track_inserts_missing_file_track() {
    let dir = tempfile::tempdir().unwrap();
    // Deliberately absent: import_track must not require the file on disk.
    let missing = dir.path().join("gone.flac");
    let metadata = TrackMetadata {
        title: Some("Gone".into()),
        bpm: Some(128.0),
        ..TrackMetadata::default()
    };

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let imported = lib.import_track(&missing, &metadata).unwrap();
    assert!(imported.created, "first import inserts the track");
    assert_eq!(imported.source.metadata().title.as_deref(), Some("Gone"));

    let fetched = lib.get_track(imported.source.id()).unwrap().unwrap();
    assert_eq!(fetched.file().unwrap().path(), missing.as_path());

    // Second import of the same path is an update, not a new row.
    let again = lib.import_track(&missing, &metadata).unwrap();
    assert!(!again.created, "re-import updates the existing row");
}

#[test]
fn update_track_metadata_normalizes_key_and_blank_fields() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let imported = lib
        .import_track(Path::new("/music/gone.flac"), &TrackMetadata::default())
        .unwrap();
    let id = imported.source.id().clone();

    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            title: Some("  ".into()),
            artist: None,
            album: None,
            genre: None,
            bpm: None,
            key: Some("8A".into()),
            isrc: Some("  USRC1  ".into()),
        },
    )
    .unwrap();

    let fetched = lib.get_track(&id).unwrap().unwrap();
    let metadata = fetched.metadata();
    assert_eq!(metadata.key.as_deref(), Some("Am"), "Camelot normalized");
    assert_eq!(metadata.title, None, "whitespace-only clears the field");
    assert_eq!(metadata.isrc.as_deref(), Some("USRC1"), "trimmed");
}

#[test]
fn update_track_metadata_rejects_invalid_bpm() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let imported = lib
        .import_track(Path::new("/music/gone.flac"), &TrackMetadata::default())
        .unwrap();
    let id = imported.source.id().clone();

    for invalid in [0.0, -5.0, f64::NAN, f64::INFINITY] {
        let err = lib
            .update_track_metadata(
                &id,
                TrackMetadataEdit {
                    title: None,
                    artist: None,
                    album: None,
                    genre: None,
                    bpm: Some(invalid),
                    key: None,
                    isrc: None,
                },
            )
            .expect_err("invalid BPM must be an error, not a silent clear");
        assert!(
            format!("{err}").contains("invalid BPM"),
            "unexpected error: {err}"
        );
    }

    // None still clears the column — only a non-None invalid value errors.
    lib.update_track_metadata(&id, TrackMetadataEdit::default())
        .unwrap();
    let fetched = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(fetched.metadata().bpm, None);
}

#[test]
fn prepare_track_for_playback_reuses_cached_decode() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_analysis_wav(&wav);

    let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
    let track_id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };

    let first = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    let second = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();

    assert_eq!(first.track_id, track_id);
    assert_eq!(second.track_id, track_id);
    assert!(std::sync::Arc::ptr_eq(&first.audio, &second.audio));
}

#[test]
fn clear_waveform_cache_drops_decode_cache() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_analysis_wav(&wav);

    let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
    let track_id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };

    let first = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    {
        let lib = library.lock().unwrap();
        assert!(lib.decode_cache.lock().unwrap().contains(&track_id));
        lib.clear_waveform_cache().unwrap();
        assert_eq!(lib.decode_cache.lock().unwrap().len(), 0);
    }
    let second = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    assert!(!std::sync::Arc::ptr_eq(&first.audio, &second.audio));
}

#[test]
fn compute_waveform_window_from_decode_cache() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_analysis_wav(&wav);

    let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
    let track_id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };

    LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    let (peaks, start_ms, end_ms) =
        LibraryManager::compute_waveform_window(&library, &track_id, -1_000, 500, 64).unwrap();
    assert_eq!(start_ms, 0);
    assert_eq!(end_ms, 500);
    assert_eq!(peaks.len(), 64);
    assert!(peaks.iter().any(|p| p.low + p.mid + p.high > 0.0));
}

#[test]
fn compute_waveform_window_rejects_out_of_range_buckets() {
    let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
    let id = TrackId::new("missing");
    let zero = LibraryManager::compute_waveform_window(&library, &id, 0, 100, 0).unwrap_err();
    assert!(
        zero.to_string().contains("bucket"),
        "zero buckets should fail validation, got {zero}"
    );
    let huge = LibraryManager::compute_waveform_window(
        &library,
        &id,
        0,
        100,
        audio_core::MAX_WAVEFORM_BUCKETS + 1,
    )
    .unwrap_err();
    assert!(
        huge.to_string().contains("bucket"),
        "oversized buckets should fail validation, got {huge}"
    );
}

#[test]
fn prepare_track_for_playback_does_not_require_waveform() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_analysis_wav(&wav);

    let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
    let track_id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };

    assert!(library
        .lock()
        .unwrap()
        .get_track_waveform_overview(&track_id)
        .unwrap()
        .is_none());

    let prepared = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    assert_eq!(prepared.track_id, track_id);
    assert!(prepared.loudness_lufs.is_some());
    assert!(library
        .lock()
        .unwrap()
        .get_track_waveform_overview(&track_id)
        .unwrap()
        .is_none());

    LibraryManager::ensure_track_waveform(&library, &track_id).unwrap();
    assert!(library
        .lock()
        .unwrap()
        .get_track_waveform_overview(&track_id)
        .unwrap()
        .is_some());
}

#[cfg(feature = "analysis")]
fn write_tiny_stem_mp4(dir: &Path) -> PathBuf {
    use codec::{encode_stem_mp4, StemAtom, StemMuxFormat};

    let rate: u32 = 48_000;
    let frames = (rate / 10) as usize;
    let tone = |amp: f32| -> Vec<f32> {
        (0..frames * 2)
            .map(|i| amp * ((i / 2) as f32 * 0.01).sin())
            .collect()
    };
    let mix = tone(0.1);
    let stems = [tone(0.2), tone(0.3), tone(0.4), tone(0.5)];
    let path = dir.join("fixture.stem.mp4");
    encode_stem_mp4(
        &path,
        StemMuxFormat::Opus,
        rate,
        &mix,
        [&stems[0], &stems[1], &stems[2], &stems[3]],
        &StemAtom::default_ni(),
    )
    .unwrap();
    path
}

#[cfg(feature = "analysis")]
#[test]
fn import_and_prepare_native_stem_file() {
    let dir = tempfile::tempdir().unwrap();
    let stem_path = write_tiny_stem_mp4(dir.path());
    let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());

    let prepared = LibraryManager::prepare_file_path_for_playback(&library, &stem_path).unwrap();
    assert!(prepared.stems.is_some());
    assert!(prepared.audio.samples.len() > 1_000);
    let stems = prepared.stems.unwrap();
    assert_eq!(stems.len(), 4);
    assert!(stems[0].source_id.ends_with("#drums"));
}

#[cfg(feature = "analysis")]
#[test]
fn import_file_path_accepts_native_stem_file() {
    let dir = tempfile::tempdir().unwrap();
    let stem_path = write_tiny_stem_mp4(dir.path());
    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&stem_path).unwrap();
    assert_eq!(track.file().unwrap().path(), stem_path.as_path());
}

#[cfg(feature = "analysis")]
#[test]
fn collect_audio_files_includes_native_stem_file() {
    let dir = tempfile::tempdir().unwrap();
    let stem_path = write_tiny_stem_mp4(dir.path());
    write_minimal_wav(&dir.path().join("plain.wav"));

    let files = collect_audio_files(dir.path(), false).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|path| path == &stem_path));
}

#[cfg(feature = "analysis")]
fn matching_stem_fingerprint(source: &Path) -> String {
    let meta = std::fs::metadata(source).unwrap();
    let mtime = meta
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("v1:{}:{}:{}:48000:2:0", source.display(), mtime, meta.len())
}

#[cfg(feature = "analysis")]
#[test]
fn prepare_resolves_stem_cache_hit() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_analysis_wav(&wav);
    let cache_path = write_tiny_stem_mp4(dir.path());

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = LibraryBuses::new();
    lib.set_buses(buses);

    let library = Mutex::new(lib);
    let track_id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };
    {
        let lib = library.lock().unwrap();
        stems::upsert_track_stems(
            &lib.db,
            &track_id,
            &TrackStemsInfo {
                backend: "htdemucs_mixxx_v1".into(),
                source_fingerprint: matching_stem_fingerprint(&wav),
                format: "opus".into(),
                sample_rate: 48_000,
                path: cache_path,
                generated_at: "1".into(),
            },
        )
        .unwrap();
    }

    let prepared = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    assert!(prepared.stems.is_some());
    assert!(prepared.audio.samples.len() > 1_000);
}

#[cfg(feature = "analysis")]
#[test]
fn prepare_discards_fingerprint_mismatched_stem_cache() {
    // Deck load always consults the stem cache now, so a cache whose
    // fingerprint no longer matches the source file must be discarded and
    // playback must fall back to the original. Assert the discard itself,
    // not just the absent stems, so this cannot silently start passing for
    // a different reason.
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_analysis_wav(&wav);
    let cache_path = write_tiny_stem_mp4(dir.path());

    let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
    let track_id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };
    {
        let lib = library.lock().unwrap();
        stems::upsert_track_stems(
            &lib.db,
            &track_id,
            &TrackStemsInfo {
                backend: "htdemucs_mixxx_v1".into(),
                // Deliberately not `matching_stem_fingerprint(&wav)`.
                source_fingerprint: "fp".into(),
                format: "opus".into(),
                sample_rate: 48_000,
                path: cache_path.clone(),
                generated_at: "1".into(),
            },
        )
        .unwrap();
    }

    let prepared = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    assert!(prepared.stems.is_none());

    // The stale row and its file are dropped, so the next prepare does not
    // retry the same decode.
    {
        let lib = library.lock().unwrap();
        assert!(!stems::has_track_stems(&lib.db, &track_id).unwrap());
    }
    assert!(!cache_path.is_file(), "stale stem file left on disk");
}

#[cfg(feature = "analysis")]
#[test]
fn prepare_corrupt_stem_cache_falls_back_to_original() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_analysis_wav(&wav);
    let corrupt = dir.path().join("bad.stem.mp4");
    std::fs::write(&corrupt, b"not a stem file").unwrap();

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = LibraryBuses::new();
    lib.set_buses(buses);

    let library = Mutex::new(lib);
    let track_id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };
    {
        let lib = library.lock().unwrap();
        stems::upsert_track_stems(
            &lib.db,
            &track_id,
            &TrackStemsInfo {
                backend: "htdemucs_mixxx_v1".into(),
                source_fingerprint: matching_stem_fingerprint(&wav),
                format: "opus".into(),
                sample_rate: 48_000,
                path: corrupt,
                generated_at: "1".into(),
            },
        )
        .unwrap();
    }

    let prepared = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    assert!(prepared.stems.is_none());
    assert!(library
        .lock()
        .unwrap()
        .get_track_stems(&track_id)
        .unwrap()
        .is_none());
    assert!(prepared.audio.samples.len() > 1_000);
}

#[test]
fn lookup_file_track_at_path_finds_imported_track() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("track.wav");
    write_minimal_wav(&wav);

    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();

    let found = lib.lookup_file_track_at_path(&wav).unwrap().unwrap();
    assert_eq!(found.id(), track.id());
}

#[test]
fn add_collection_and_sync() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("house");
    std::fs::create_dir_all(&nested).unwrap();
    write_minimal_wav(&dir.path().join("a.wav"));
    write_minimal_wav(&nested.join("b.wav"));
    std::fs::write(dir.path().join("readme.txt"), b"nope").unwrap();

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let folder = lib
        .add_collection(&NewCollection::folder_named(dir.path(), "Music"))
        .unwrap();
    assert_eq!(folder.collection_type(), CollectionType::Folder);
    assert_eq!(folder.name, "Music");

    let report = lib.sync_collection(Some(&folder.id)).unwrap();
    assert_eq!(report.added, 2);

    let tracks = lib.list_collection_tracks(&folder.id).unwrap();
    assert_eq!(tracks.len(), 2);
}

#[test]
fn resync_folder_counts_updated_not_added() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("a.wav"));

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let folder = lib
        .add_collection(&NewCollection::folder_named(dir.path(), "Music"))
        .unwrap();

    let first = lib.sync_collection(Some(&folder.id)).unwrap();
    assert_eq!(first.added, 1);
    assert_eq!(first.updated, 0);

    let second = lib.sync_collection(Some(&folder.id)).unwrap();
    assert_eq!(second.added, 0);
    assert_eq!(second.updated, 1);
}

#[test]
fn sync_collection_off_mutex_matches_in_process_sync() {
    use std::sync::Mutex;

    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("a.wav"));

    let direct = {
        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder_named(dir.path(), "Music"))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap()
    };

    let off_mutex = {
        let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
        let folder_id = {
            let mut lib = library.lock().unwrap();
            lib.add_collection(&NewCollection::folder_named(dir.path(), "Music"))
                .unwrap()
                .id
        };
        LibraryManager::sync_collection_off_mutex(&library, Some(&folder_id)).unwrap()
    };

    assert_eq!(direct.added, off_mutex.added);
    assert_eq!(direct.updated, off_mutex.updated);
    assert_eq!(direct.skipped, off_mutex.skipped);
    assert_eq!(direct.failed, off_mutex.failed);
}

#[test]
#[cfg(feature = "analysis")]
fn analyze_track_off_mutex_does_not_recreate_deleted_track() {
    use std::sync::Mutex;

    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("a.wav");
    write_minimal_wav(&wav);

    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let id = track.id().clone();
    let library = Mutex::new(lib);

    let path = {
        let lib = library.lock().unwrap();
        lib.get_track(&id)
            .unwrap()
            .unwrap()
            .file()
            .unwrap()
            .path()
            .to_path_buf()
    };

    let options = AnalyzeTrackOptions {
        force: true,
        analysis_duration: AnalysisDurationMode::Fast,
    };
    let _computed = compute_file_analysis(&path, options).unwrap();

    {
        let lib = library.lock().unwrap();
        lib.store().delete_track(&id).unwrap();
        assert!(!lib.store().track_exists(&id).unwrap());
        assert!(matches!(
            lib.ensure_track_still_at_path(&id, &path),
            Err(LibraryError::NotFound(_))
        ));
    }

    assert!(!library.lock().unwrap().store().track_exists(&id).unwrap());
}

#[test]
fn folder_scan_folder_tree_survives_db_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("house");
    std::fs::create_dir_all(&nested).unwrap();
    write_minimal_wav(&dir.path().join("a.wav"));
    write_minimal_wav(&nested.join("b.wav"));

    let db_path = dir.path().join("library.db");
    let folder_id = {
        let mut lib = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();
        let folder = lib
            .add_collection(&NewCollection {
                name: Some("Flat".into()),
                config: CollectionConfig::Folder {
                    fs_path: dir.path().to_path_buf(),
                    scan_folder_tree: false,
                },
            })
            .unwrap();
        folder.id.clone()
    };

    let mut lib = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();
    let loaded = lib.get_collection(&folder_id).unwrap().unwrap();
    let CollectionConfig::Folder {
        scan_folder_tree, ..
    } = loaded.config
    else {
        panic!("expected folder collection");
    };
    assert!(!scan_folder_tree);

    let report = lib.sync_collection(Some(&folder_id)).unwrap();
    assert_eq!(report.added, 1);
}

#[test]
fn playlist_membership_and_order() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    write_minimal_wav(&a);
    write_minimal_wav(&b);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let ta = lib.import_file_path(&a).unwrap();
    let tb = lib.import_file_path(&b).unwrap();

    let pl = lib
        .add_collection(&NewCollection::playlist("Warmup", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, ta.id(), None).unwrap();
    lib.add_collection_entry(&pl.id, tb.id(), None).unwrap();

    let tracks = lib.list_collection_tracks(&pl.id).unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].id(), ta.id());
    assert_eq!(tracks[1].id(), tb.id());

    let rows = lib.list_collection_entries(&pl.id).unwrap();
    let ta_row = rows
        .iter()
        .find(|row| row.track_id == *ta.id())
        .expect("ta row");
    let tb_row = rows
        .iter()
        .find(|row| row.track_id == *tb.id())
        .expect("tb row");
    lib.update_collection_entries(&pl.id, &[tb_row.id.clone(), ta_row.id.clone()])
        .unwrap();
    let tracks = lib.list_collection_tracks(&pl.id).unwrap();
    assert_eq!(tracks[0].id(), tb.id());
    assert_eq!(tracks[1].id(), ta.id());

    lib.remove_collection_entry(&pl.id, &ta_row.id).unwrap();
    assert_eq!(lib.list_collection_tracks(&pl.id).unwrap().len(), 1);
}

#[test]
fn collection_entries_with_sources_reports_missing_track() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("a.wav");
    write_minimal_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let pl = lib
        .add_collection(&NewCollection::playlist("Set", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, track.id(), None).unwrap();
    lib.store().delete_track_row_only(track.id()).unwrap();

    let rows = lib.collection_entries_with_sources(&pl.id).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].1.is_none());
    assert_eq!(lib.list_collection_tracks(&pl.id).unwrap().len(), 0);
}

#[test]
fn sortable_playlist_allows_duplicate_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("a.wav");
    write_minimal_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let pl = lib
        .add_collection(&NewCollection::playlist("Set", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, track.id(), None).unwrap();
    lib.add_collection_entry(&pl.id, track.id(), None).unwrap();

    let tracks = lib.list_collection_tracks(&pl.id).unwrap();
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].id(), track.id());
    assert_eq!(tracks[1].id(), track.id());

    let rows = lib.list_collection_entries(&pl.id).unwrap();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].id, rows[1].id);
    lib.remove_collection_entry(&pl.id, &rows[0].id).unwrap();
    assert_eq!(lib.list_collection_tracks(&pl.id).unwrap().len(), 1);
}

#[test]
fn unsortable_playlist_rejects_reorder() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("x.wav");
    write_minimal_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let pl = lib
        .add_collection(&NewCollection::playlist("Crate", false))
        .unwrap();
    lib.add_collection_entry(&pl.id, track.id(), None).unwrap();

    let rows = lib.list_collection_entries(&pl.id).unwrap();
    let err = lib
        .update_collection_entries(&pl.id, &[rows[0].id.clone()])
        .unwrap_err();
    assert!(matches!(err, LibraryError::Unsupported(_)));
}

#[test]
fn update_collection_sortable() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.wav");
    write_minimal_wav(&a);
    write_minimal_wav(&b);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let ta = lib.import_file_path(&a).unwrap();
    let tb = lib.import_file_path(&b).unwrap();
    let pl = lib
        .add_collection(&NewCollection::playlist("Set", false))
        .unwrap();
    lib.add_collection_entry(&pl.id, ta.id(), None).unwrap();
    lib.add_collection_entry(&pl.id, tb.id(), None).unwrap();

    lib.update_collection(&pl.id, &UpdateCollection::sortable(true))
        .unwrap();
    let pl = lib.get_collection(&pl.id).unwrap().unwrap();
    assert!(pl.sortable());
    let rows = lib.list_collection_entries(&pl.id).unwrap();
    let tb_row = rows
        .iter()
        .find(|row| row.track_id == *tb.id())
        .expect("tb row");
    let ta_row = rows
        .iter()
        .find(|row| row.track_id == *ta.id())
        .expect("ta row");
    lib.update_collection_entries(&pl.id, &[tb_row.id.clone(), ta_row.id.clone()])
        .unwrap();

    lib.update_collection(&pl.id, &UpdateCollection::sortable(false))
        .unwrap();
    let rows = lib.list_collection_entries(&pl.id).unwrap();
    let err = lib
        .update_collection_entries(
            &pl.id,
            &rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>(),
        )
        .unwrap_err();
    assert!(matches!(err, LibraryError::Unsupported(_)));
}

#[cfg(unix)]
#[test]
fn collect_audio_files_skips_symlink_outside_root() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write_minimal_wav(&outside.path().join("secret.wav"));
    write_minimal_wav(&dir.path().join("inside.wav"));
    std::os::unix::fs::symlink(
        outside.path().join("secret.wav"),
        dir.path().join("link.wav"),
    )
    .unwrap();

    let files = collect_audio_files(dir.path(), false).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("inside.wav"));
}

#[test]
fn sortable_to_unsortable_dedupes_duplicate_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("a.wav");
    write_minimal_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let pl = lib
        .add_collection(&NewCollection::playlist("Set", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, track.id(), None).unwrap();
    lib.add_collection_entry(&pl.id, track.id(), None).unwrap();
    assert_eq!(lib.list_collection_entries(&pl.id).unwrap().len(), 2);

    lib.update_collection(&pl.id, &UpdateCollection::sortable(false))
        .unwrap();
    let rows = lib.list_collection_entries(&pl.id).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].track_id, *track.id());
    assert!(rows[0].position.is_none());
}

#[test]
fn delete_playlist_keeps_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("keep.wav");
    write_minimal_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let pl = lib
        .add_collection(&NewCollection::playlist("Temp", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, track.id(), None).unwrap();
    lib.delete_collection(&pl.id).unwrap();

    assert!(lib.get_collection(&pl.id).unwrap().is_none());
    assert!(lib.get_track(track.id()).unwrap().is_some());
}

#[test]
fn delete_folder_keeps_tracks() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("t.wav"));

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let folder = lib
        .add_collection(&NewCollection::folder(dir.path()))
        .unwrap();
    lib.sync_collection(Some(&folder.id)).unwrap();
    let track_id = lib.list_collection_tracks(&folder.id).unwrap()[0]
        .id()
        .clone();

    lib.delete_collection(&folder.id).unwrap();
    assert!(lib.get_track(&track_id).unwrap().is_some());
}

#[test]
fn collection_analyze_targets_skips_tagged_and_analyzed() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("plain.wav"));
    write_minimal_wav(&dir.path().join("tagged.wav"));
    write_minimal_wav(&dir.path().join("done.wav"));

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let folder = lib
        .add_collection(&NewCollection::folder(dir.path()))
        .unwrap();
    lib.sync_collection(Some(&folder.id)).unwrap();

    // Stamp BPM on one track (as file tags would) and an analysis row on another.
    for track in lib.list_collection_tracks(&folder.id).unwrap() {
        let path = track.file().unwrap().path().to_path_buf();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "tagged.wav" {
            let mut metadata = track.metadata().clone();
            metadata.bpm = Some(128.0);
            lib.store()
                .upsert_file_track(track.id(), &path, &metadata, "now")
                .unwrap();
        }
    }
    let done_id = lib
        .list_collection_tracks(&folder.id)
        .unwrap()
        .into_iter()
        .find(|t| {
            t.file()
                .is_some_and(|f| f.path().file_name().and_then(|n| n.to_str()) == Some("done.wav"))
        })
        .unwrap()
        .id()
        .clone();
    #[cfg(feature = "analysis")]
    crate::analysis::upsert_track_analysis(
        &lib.db,
        &done_id,
        &analyzer::TrackAnalysis {
            bpm: None,
            key: None,
            beat_grid: None,
            loudness_lufs: Some(-14.0),
            metadata: analyzer::AnalysisRunMetadata {
                backend: "test".into(),
                backend_version: "1".into(),
                analyzed_at: "now".into(),
                sample_rate: 8000,
                duration_analyzed_ms: 2,
            },
        },
    )
    .unwrap();

    let mut targets = lib
        .collection_analyze_targets(&folder.id, false)
        .unwrap()
        .into_iter()
        .map(|id| id.as_str().to_string())
        .collect::<Vec<_>>();
    targets.sort();
    assert_eq!(targets.len(), 1);
    assert!(targets[0].ends_with("plain.wav"), "{targets:?}");

    // Force re-includes every file track.
    let forced = lib.collection_analyze_targets(&folder.id, true).unwrap();
    assert_eq!(forced.len(), 3);
}

#[test]
#[cfg(feature = "analysis")]
fn replaygain_loudness_replaces_measured_loudness() {
    let loudness = preferred_loudness_lufs(Some(-12.0), Some(3.2));
    assert_eq!(loudness, Some(-21.2));
    assert_eq!(preferred_loudness_lufs(Some(-12.0), None), Some(-12.0));
}

#[test]
#[cfg(feature = "analysis")]
fn analyze_track_persists_track_analysis() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_analysis_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    lib.analyze_track(track.id(), AnalyzeTrackOptions::default())
        .unwrap();

    let count = lib.store().count_track_analysis(track.id()).unwrap();
    assert_eq!(count, 1);
    assert!(lib.track_loudness_lufs(track.id()).unwrap().is_some());
}

#[test]
#[cfg(feature = "analysis")]
fn prepare_ensures_playback_analysis_without_waveform() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_analysis_wav(&wav);

    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let track_id = track.id().clone();
    assert!(lib.needs_playback_analysis(&track_id).unwrap());

    let library = Mutex::new(lib);
    let prepared = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    assert!(prepared.loudness_lufs.is_some());
    {
        let lib = library.lock().unwrap();
        assert!(!lib.needs_playback_analysis(&track_id).unwrap());
        assert!(lib
            .get_track_waveform_overview(&track_id)
            .unwrap()
            .is_none());
    }
    let again = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
    assert_eq!(again.loudness_lufs, prepared.loudness_lufs);
}

#[test]
#[cfg(feature = "analysis")]
fn ensure_track_analysis_fills_missing_loudness() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_analysis_wav(&wav);

    let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let track_id = track.id().clone();
    // Simulate tags with BPM/key but no analysis loudness yet.
    lib.upsert_file_source(
        &wav,
        &TrackMetadata {
            title: Some("song".into()),
            bpm: Some(128.0),
            key: Some("Am".into()),
            duration_ms: Some(3_000),
            sample_rate: Some(48_000),
            channels: Some(1),
            ..TrackMetadata::default()
        },
    )
    .unwrap();
    assert!(lib.needs_playback_analysis(&track_id).unwrap());

    let library = Mutex::new(lib);
    LibraryManager::ensure_track_analysis(&library, &track_id).unwrap();
    let lib = library.lock().unwrap();
    assert!(!lib.needs_playback_analysis(&track_id).unwrap());
    assert!(lib.track_loudness_lufs(&track_id).unwrap().is_some());
    assert!(lib
        .get_track_waveform_overview(&track_id)
        .unwrap()
        .is_none());
}

#[test]
#[cfg(feature = "analysis")]
fn analyze_track_persists_waveform_overview() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_analysis_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    assert!(lib
        .get_track_waveform_overview(track.id())
        .unwrap()
        .is_none());

    lib.analyze_track(track.id(), AnalyzeTrackOptions::default())
        .unwrap();

    let overview = lib
        .get_track_waveform_overview(track.id())
        .unwrap()
        .expect("waveform overview should exist after analyze");
    assert_eq!(overview.overview_count, audio_core::OVERVIEW_SAMPLE_COUNT);
    assert_eq!(overview.peaks.len(), audio_core::OVERVIEW_SAMPLE_COUNT);

    // Idempotent: ensure after analyze must not fail or clear the overview.
    let track_id = track.id().clone();
    let library = Mutex::new(lib);
    LibraryManager::ensure_track_waveform(&library, &track_id).unwrap();
    assert!(library
        .lock()
        .unwrap()
        .get_track_waveform_overview(&track_id)
        .unwrap()
        .is_some());
}

#[test]
fn analyze_track_refreshes_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_analysis_wav(&wav);

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let track = lib.import_file_path(&wav).unwrap();
    let analyzed = lib
        .analyze_track(track.id(), AnalyzeTrackOptions::default())
        .unwrap();

    assert_eq!(analyzed.metadata().title.as_deref(), Some("song"));
    assert_eq!(
        analyzed.metadata().sample_rate,
        track.metadata().sample_rate
    );
}

#[test]
fn analyze_track_rejects_stream() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let source = lib
        .import_stream(
            "https://example.com/track.mp3",
            &TrackMetadata::default(),
            Some(StreamProvider::Http),
        )
        .unwrap();
    let err = lib
        .analyze_track(source.id(), AnalyzeTrackOptions::default())
        .unwrap_err();
    assert!(matches!(err, LibraryError::Unsupported(_)));
}

#[test]
fn remove_collection_entry_drops_orphaned_stream() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let meta = TrackMetadata {
        title: Some("Remote Track".into()),
        ..Default::default()
    };
    let source = lib
        .import_stream(
            "https://example.com/track.mp3",
            &meta,
            Some(StreamProvider::Http),
        )
        .unwrap();
    let pl = lib
        .add_collection(&NewCollection::playlist("Streaming", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, source.id(), None).unwrap();

    let entry_id = lib.list_collection_entries(&pl.id).unwrap()[0].id.clone();
    lib.remove_collection_entry(&pl.id, &entry_id).unwrap();
    assert!(lib.get_track(source.id()).unwrap().is_none());
}

#[test]
fn remove_collection_entry_keeps_folder_linked_file() {
    let dir = tempfile::tempdir().unwrap();
    write_minimal_wav(&dir.path().join("keep.wav"));

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let folder = lib
        .add_collection(&NewCollection::folder(dir.path()))
        .unwrap();
    lib.sync_collection(Some(&folder.id)).unwrap();
    let track_id = lib.list_collection_tracks(&folder.id).unwrap()[0]
        .id()
        .clone();

    let pl = lib
        .add_collection(&NewCollection::playlist("Also", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, &track_id, None).unwrap();
    let entry_id = lib.list_collection_entries(&pl.id).unwrap()[0].id.clone();
    lib.remove_collection_entry(&pl.id, &entry_id).unwrap();

    assert!(lib.get_track(&track_id).unwrap().is_some());
}

#[test]
fn import_stream_round_trip() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let meta = TrackMetadata {
        title: Some("Remote Track".into()),
        ..Default::default()
    };
    let source = lib
        .import_stream(
            "https://example.com/track.mp3",
            &meta,
            Some(StreamProvider::Http),
        )
        .unwrap();

    let AudioSource::Stream(stream) = &source else {
        panic!("expected stream source");
    };
    assert_eq!(stream.uri(), "https://example.com/track.mp3");
    assert_eq!(source.metadata().title.as_deref(), Some("Remote Track"));

    let pl = lib
        .add_collection(&NewCollection::playlist("Streaming", true))
        .unwrap();
    lib.add_collection_entry(&pl.id, source.id(), None).unwrap();
    assert_eq!(lib.list_collection_tracks(&pl.id).unwrap().len(), 1);
}

#[test]
fn open_persists_to_disk() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("lib.db");
    let music = dir.path().join("music");
    std::fs::create_dir_all(&music).unwrap();
    write_minimal_wav(&music.join("p.wav"));

    {
        let mut lib = LibraryManager::open(&db, LibraryConfig::default()).unwrap();
        let folder = lib.add_collection(&NewCollection::folder(&music)).unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let pl = lib
            .add_collection(&NewCollection::playlist("All", true))
            .unwrap();
        let tracks = lib.list_collection_tracks(&folder.id).unwrap();
        lib.add_collection_entry(&pl.id, tracks[0].id(), None)
            .unwrap();
    }

    let lib = LibraryManager::open(&db, LibraryConfig::default()).unwrap();
    assert_eq!(lib.list_collections().unwrap().len(), 2);
    let folder = lib
        .list_collections()
        .unwrap()
        .into_iter()
        .find(|c| c.collection_type() == CollectionType::Folder)
        .unwrap();
    assert_eq!(lib.list_collection_tracks(&folder.id).unwrap().len(), 1);
}

// --- Optional DJ tag write-back (#451) -------------------------------------

/// 6 s of 120 BPM clicks — the fixture the engine tests use when they need a
/// detectable tempo. Copied into a temp dir: write-back must never touch a repo file.
#[cfg(feature = "analysis")]
fn short_beats_fixture_copy(dir: &Path) -> PathBuf {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-beats.wav");
    let path = dir.join("short-beats.wav");
    std::fs::copy(source, &path).unwrap();
    path
}

#[test]
#[cfg(feature = "analysis")]
fn analyze_track_writes_dj_tags_back_when_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let wav = short_beats_fixture_copy(dir.path());

    let mut lib = LibraryManager::open_in_memory(LibraryConfig {
        tag_write_back: TagWriteBack::On,
    })
    .unwrap();
    let id = lib.import_file_path(&wav).unwrap().id().clone();
    lib.analyze_track(&id, AnalyzeTrackOptions::force())
        .unwrap();

    let metadata = lib.get_track(&id).unwrap().unwrap().metadata().clone();
    let bpm = metadata.bpm.expect("analysis should produce a BPM");
    let tags = crate::tags::read_tags(&wav).unwrap();
    // The file tag mirrors the library value; ID3v2 TBPM is integer-only.
    assert_eq!(tags.bpm, Some(bpm.round()));
    assert_eq!(tags.key, metadata.key);

    // Re-running analysis with the same result must not rewrite the file.
    let before = std::fs::read(&wav).unwrap();
    lib.analyze_track(&id, AnalyzeTrackOptions::force())
        .unwrap();
    assert_eq!(std::fs::read(&wav).unwrap(), before);
}

#[test]
#[cfg(feature = "analysis")]
fn analyze_track_leaves_files_untouched_when_write_back_is_off() {
    let dir = tempfile::tempdir().unwrap();
    let wav = short_beats_fixture_copy(dir.path());
    let before = std::fs::read(&wav).unwrap();

    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    assert!(!lib.config().tag_write_back.is_on());
    let id = lib.import_file_path(&wav).unwrap().id().clone();
    lib.analyze_track(&id, AnalyzeTrackOptions::force())
        .unwrap();

    assert!(lib
        .get_track(&id)
        .unwrap()
        .unwrap()
        .metadata()
        .bpm
        .is_some());
    assert_eq!(std::fs::read(&wav).unwrap(), before);
    assert!(crate::tags::read_tags(&wav).unwrap().bpm.is_none());
}

#[test]
fn save_beat_grid_cmd_writes_bpm_back_when_enabled() {
    use library_api::{encode_cmd_body, CmdBody};
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("grid.wav");
    write_analysis_wav(&wav);

    let session = LibrarySession::open_in_memory(LibraryConfig {
        tag_write_back: TagWriteBack::On,
    })
    .unwrap();
    let track_id = {
        let library = session.library();
        let lib = library.lock().unwrap();
        lib.import_file_path(&wav).unwrap().id().clone()
    };

    let rx = session.subscribe_evt_track(track_id.as_str()).unwrap();
    session
        .publish_cmd(
            Origin::Library,
            Kind::SaveBeatGrid,
            encode_cmd_body(&CmdBody::SaveBeatGrid {
                track_id: track_id.as_str().to_string(),
                bpm: 124.0,
                first_beat_secs: 0.25,
            })
            .unwrap(),
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = false;
    while Instant::now() < deadline && !seen {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(Some(event)) => {
                if let Ok(EvtBody::BeatGridChanged { .. }) =
                    library_api::decode_evt_body(event.payload())
                {
                    seen = true;
                }
            }
            Ok(None) => continue,
            Err(_) => break,
        }
    }
    assert!(seen, "beat grid change should be published");

    let tags = crate::tags::read_tags(&wav).unwrap();
    assert_eq!(tags.bpm, Some(124.0));
}

#[test]
#[cfg(feature = "analysis")]
fn persist_file_analysis_defers_tag_write_back_to_its_caller() {
    use crate::analysis::compute_file_analysis;

    let dir = tempfile::tempdir().unwrap();
    let wav = short_beats_fixture_copy(dir.path());

    let lib = LibraryManager::open_in_memory(LibraryConfig {
        tag_write_back: TagWriteBack::On,
    })
    .unwrap();
    lib.import_file_path(&wav).unwrap();
    let computed = compute_file_analysis(&wav, AnalyzeTrackOptions::force()).unwrap();
    let bpm = computed
        .metadata
        .bpm
        .expect("analysis should produce a BPM");

    // Planning happens under the lock; the file must not be touched yet, because
    // callers hold the manager mutex while they call this.
    let (_, plan) = lib.persist_file_analysis(&wav, &computed, false).unwrap();
    assert!(crate::tags::read_tags(&wav).unwrap().bpm.is_none());

    // Executing the plan (callers do this after dropping the lock) writes it.
    let plan = plan.expect("write-back on should produce a plan");
    assert!(plan.execute().is_some());
    assert_eq!(
        crate::tags::read_tags(&wav).unwrap().bpm,
        Some(bpm.round()),
        "planned value should reach the file"
    );

    // With the setting off there is nothing to run later.
    let off = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    off.import_file_path(&wav).unwrap();
    let (_, plan) = off.persist_file_analysis(&wav, &computed, false).unwrap();
    assert!(plan.is_none());
}
