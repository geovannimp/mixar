//! End-to-end: app setting → `LibraryTransport` → worker analysis → file tags (#451).

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bus_api::Event;
use bus_api::TrackAnalyzed;
use host_flutter::api::library::{LibraryAnalysisDurationSetting, LibraryTransport};

/// 6 s of 120 BPM clicks — the fixture engine tests use when they need a tempo.
/// Copied into a temp dir: write-back must never touch a repo file.
fn short_beats_fixture_copy(dir: &Path) -> PathBuf {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-beats.wav");
    let path = dir.join("short-beats.wav");
    std::fs::copy(source, &path).unwrap();
    path
}

fn only_track_id(transport: &LibraryTransport) -> String {
    let collection = transport.list_collections().unwrap().remove(0);
    let tracks = transport.list_collection_entries(collection.id).unwrap();
    assert_eq!(tracks.len(), 1);
    tracks.into_iter().next().unwrap().id
}

/// Queue analysis and wait for the worker's `TrackAnalyzed` event, returning the
/// library-side track summary it carries.
fn analyze_and_wait(
    transport: &LibraryTransport,
    track_id: &str,
    timeout: Duration,
) -> bus_api::TrackSummary {
    let mut rx = transport.bus().subscribe_evt().unwrap();
    transport.analyze_track(track_id.to_string(), true).unwrap();

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(100)) {
            // Analysis also emits `TrackProgress` first; only the final summary matters.
            Ok(Some(Event::TrackAnalyzed(TrackAnalyzed { track }))) => return track,
            Ok(Some(_)) | Ok(None) => continue,
            Err(err) => panic!("library evt bus closed: {err}"),
        }
    }
    panic!("no TrackAnalyzed event within {timeout:?}");
}

#[test]
fn analyze_writes_the_library_bpm_and_key_into_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let wav = short_beats_fixture_copy(dir.path());

    let transport = LibraryTransport::open_in_memory().unwrap();
    transport
        .apply_library_settings(
            LibraryAnalysisDurationSetting::Fast,
            "opus".into(),
            "htdemucs_mixxx_v1".into(),
            true,
        )
        .unwrap();
    transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned(), true, None)
        .unwrap();

    let track = analyze_and_wait(
        &transport,
        &only_track_id(&transport),
        Duration::from_secs(60),
    );

    // Read back with lofty directly: the tag must be visible to other software,
    // not just to Mixar's own reader.
    let tags = codec::read_file_tags(&wav).unwrap();
    let bpm = track.bpm.expect("analysis should produce a BPM");
    assert_eq!(tags.bpm, Some(bpm.round()));
    assert_eq!(tags.key, track.key);
}

#[test]
fn analyze_leaves_the_file_alone_without_opt_in() {
    let dir = tempfile::tempdir().unwrap();
    let wav = short_beats_fixture_copy(dir.path());
    let before = std::fs::read(&wav).unwrap();

    let transport = LibraryTransport::open_in_memory().unwrap();
    // Default: tag write-back off.
    transport
        .apply_library_settings(
            LibraryAnalysisDurationSetting::Fast,
            "opus".into(),
            "htdemucs_mixxx_v1".into(),
            false,
        )
        .unwrap();
    transport
        .add_folder_collection(dir.path().to_string_lossy().into_owned(), true, None)
        .unwrap();

    let track = analyze_and_wait(
        &transport,
        &only_track_id(&transport),
        Duration::from_secs(60),
    );

    assert!(track.bpm.is_some());
    assert_eq!(std::fs::read(&wav).unwrap(), before);
    assert_eq!(codec::read_file_tags(&wav).unwrap().bpm, None);
}
