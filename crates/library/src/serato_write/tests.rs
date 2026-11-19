//! DB-backed tests for Serato write-back (#548).
//!
//! Every file is written into a temp dir; repository samples are never touched.
//! The codec tests prove the wire format with an independent decoder, so these
//! tests focus on the library rules: which adopted kinds are projected, when a
//! plan is created, and that a queued plan writes the latest committed state.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use library_core::{ColorLabel, Library, TrackId, TrackMetadata};

use crate::serato_adoption::{SERATO_KIND_CUES, SERATO_KIND_GRID, SERATO_KIND_LOOPS};
use crate::tests::{geob_frame, id3_tag, write_tagged_wav};
use crate::{LibraryConfig, LibraryManager, TagWriteBack, TrackMetadataEdit};

// ---------------------------------------------------------------------------
// Wire builders and helpers
// ---------------------------------------------------------------------------

fn markers2_payload(records: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut binary = vec![1u8, 1];
    for (name, payload) in records {
        binary.extend_from_slice(name.as_bytes());
        binary.push(0);
        binary.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        binary.extend_from_slice(payload);
    }
    binary.push(0);
    let mut out = vec![1u8, 1];
    out.extend_from_slice(
        base64::engine::general_purpose::STANDARD
            .encode(&binary)
            .as_bytes(),
    );
    out
}

fn cue_record(slot: u8, position_ms: u32, rgb: [u8; 3], name: &str) -> Vec<u8> {
    let mut out = vec![0, slot];
    out.extend_from_slice(&position_ms.to_be_bytes());
    out.push(0);
    out.extend_from_slice(&rgb);
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out
}

fn loop_record(slot: u8, in_ms: u32, out_ms: u32, rgb: [u8; 3], name: &str) -> Vec<u8> {
    let mut out = vec![0, slot];
    out.extend_from_slice(&in_ms.to_be_bytes());
    out.extend_from_slice(&out_ms.to_be_bytes());
    out.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    out.extend_from_slice(&[0, rgb[0], rgb[1], rgb[2]]);
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out
}

fn color_record(rgb: [u8; 3]) -> Vec<u8> {
    vec![0, rgb[0], rgb[1], rgb[2]]
}

fn grid_payload(markers: &[(f32, u32)]) -> Vec<u8> {
    let mut out = vec![1u8, 0];
    out.extend_from_slice(&(markers.len() as u32).to_be_bytes());
    for (position, value) in markers {
        out.extend_from_slice(&position.to_be_bytes());
        out.extend_from_slice(&value.to_be_bytes());
    }
    out.push(0x37);
    out
}

fn tagged_wav(dir: &Path, frames: &[Vec<u8>]) -> PathBuf {
    let path = dir.join("track.wav");
    write_tagged_wav(&path, &id3_tag(frames));
    path
}

fn manager(write_back: bool) -> LibraryManager {
    LibraryManager::open_in_memory(LibraryConfig {
        tag_write_back: if write_back {
            TagWriteBack::On
        } else {
            TagWriteBack::Off
        },
    })
    .unwrap()
}

fn payload(path: &Path) -> codec::SeratoPayload {
    codec::read_file_serato(path).unwrap()
}

fn frames_of(path: &Path, kind: codec::serato::SeratoFrameKind) -> Vec<Vec<u8>> {
    payload(path)
        .frames
        .into_iter()
        .filter(|frame| frame.kind == kind)
        .map(|frame| frame.payload)
        .collect()
}

/// Run the file mirror a worker command would run.
fn mirror(lib: &LibraryManager, id: &TrackId) -> bool {
    mirror_with(lib, id, false)
}

fn mirror_with(lib: &LibraryManager, id: &TrackId, include_track_tags: bool) -> bool {
    match crate::tags::plan_for_track(lib, id, include_track_tags) {
        Some(plan) => plan.execute_with_library(lib).is_some(),
        None => false,
    }
}

fn edit_from(metadata: &TrackMetadata) -> TrackMetadataEdit {
    TrackMetadataEdit {
        title: metadata.title.clone(),
        artist: metadata.artist.clone(),
        album: metadata.album.clone(),
        genre: metadata.genre.clone(),
        bpm: metadata.bpm,
        key: metadata.key.clone(),
        isrc: metadata.isrc.clone(),
        rating: metadata.rating,
        comment: metadata.comment.clone(),
        year: metadata.year,
        track_number: metadata.track_number,
        disc_number: metadata.disc_number,
        color_label: metadata.color_label,
    }
}

// ---------------------------------------------------------------------------
// Adopted kinds
// ---------------------------------------------------------------------------

#[test]
fn cue_and_loop_edits_reach_an_adopted_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[
                ("CUE", cue_record(0, 1000, [0xAA, 0xBB, 0xCC], "Intro")),
                ("LOOP", loop_record(0, 1000, 2000, [1, 2, 3], "Roll")),
            ]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    let kinds = lib.serato_adoption_state(&id).unwrap().adopted_kinds;
    assert_eq!(
        kinds & (SERATO_KIND_CUES | SERATO_KIND_LOOPS),
        SERATO_KIND_CUES | SERATO_KIND_LOOPS
    );
    assert_eq!(payload(&path).cues.len(), 1, "the file carried one cue");

    lib.save_track_hot_cue(
        &id,
        0,
        5000,
        None,
        Some("#AABBCC".into()),
        Some("Drop".into()),
    )
    .unwrap();
    assert!(mirror(&lib, &id));
    let after = payload(&path);
    assert_eq!(after.cues.len(), 1);
    assert_eq!(after.cues[0].position_ms, 5000);
    assert_eq!(after.cues[0].label.as_deref(), Some("Drop"));

    // Deleting the imported cue reaches the file, so a later read cannot
    // resurrect it.
    lib.delete_track_hot_cue(&id, 0).unwrap();
    assert!(mirror(&lib, &id));
    assert!(payload(&path).cues.is_empty());
    assert!(lib.list_track_hot_cues(&id).unwrap().is_empty());

    // A saved loop lands in the next free slot and leaves the imported one alone.
    lib.save_track_loop(&id, 2, 1000, 3000, Some("Out".into()), None)
        .unwrap();
    assert!(mirror(&lib, &id));
    let after = payload(&path);
    let slots: Vec<(u8, u32, u32)> = after
        .loops
        .iter()
        .map(|saved| (saved.slot, saved.in_ms, saved.out_ms))
        .collect();
    assert_eq!(slots, vec![(0, 1000, 2000), (2, 1000, 3000)]);
}

#[test]
fn db_only_cue_slots_never_reach_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();

    lib.save_track_hot_cue(&id, 9, 4000, None, None, Some("DB only".into()))
        .unwrap();
    assert!(mirror(&lib, &id));

    let db_slots: Vec<u8> = lib
        .list_track_hot_cues(&id)
        .unwrap()
        .iter()
        .map(|cue| cue.slot_index)
        .collect();
    assert!(db_slots.contains(&9), "the DB keeps slots 8–15");
    assert!(
        payload(&path).cues.iter().all(|cue| cue.slot < 8),
        "Markers2 only carries slots 0–7"
    );
    assert_eq!(payload(&path).cues[0].position_ms, 1000);
}

#[test]
fn a_file_without_marker_frames_never_gains_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("frames-none.wav");
    // A valid ID3v2 tag with no vendor frames at all.
    write_tagged_wav(&path, &id3_tag(&[]));
    let before = std::fs::read(&path).unwrap();

    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    assert_eq!(lib.serato_adoption_state(&id).unwrap().adopted_kinds, 0);

    lib.save_track_hot_cue(&id, 0, 1234, None, None, Some("Intro".into()))
        .unwrap();
    assert!(mirror(&lib, &id));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "no frame is authored"
    );
    assert!(frames_of(&path, codec::serato::SeratoFrameKind::Markers2).is_empty());
    assert_eq!(lib.list_track_hot_cues(&id).unwrap().len(), 1);
}

#[test]
fn automatic_loop_memory_is_never_exported() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("LOOP", loop_record(0, 1000, 2000, [1, 2, 3], "Roll"))]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    let before = std::fs::read(&path).unwrap();

    lib.auto_save_track_loop(&id, 4000, 6000).unwrap();
    assert!(mirror(&lib, &id));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "automatic recall is not an explicit saved loop"
    );

    // Deleting the automatic row is DB-only too.
    let auto_slot = lib
        .list_track_loops(&id)
        .unwrap()
        .into_iter()
        .find(|row| row.auto_saved)
        .expect("automatic row")
        .slot_index;
    lib.delete_track_loop(&id, auto_slot).unwrap();
    assert!(mirror(&lib, &id));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        payload(&path).loops.len(),
        1,
        "the imported saved loop survives"
    );
}

#[test]
fn write_back_off_creates_no_plan() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let lib = manager(false);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    let before = std::fs::read(&path).unwrap();

    lib.save_track_hot_cue(&id, 0, 9000, None, None, None)
        .unwrap();
    assert!(crate::tags::plan_for_track(&lib, &id, false).is_none());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

// ---------------------------------------------------------------------------
// Metadata (colour) mirroring
// ---------------------------------------------------------------------------

#[test]
fn colour_edits_mirror_only_an_adopted_colour() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[
                ("CUE", cue_record(0, 1000, [1, 2, 3], "Intro")),
                ("COLOR", color_record([0x11, 0x22, 0x33])),
            ]),
        )],
    );
    let library = Mutex::new(manager(true));
    let id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&path).unwrap().id().clone()
    };

    // A non-colour edit stays DB-only.
    let before = std::fs::read(&path).unwrap();
    let edit = {
        let lib = library.lock().unwrap();
        let mut edit = edit_from(lib.get_track(&id).unwrap().unwrap().metadata());
        edit.comment = Some("peak time".into());
        edit
    };
    LibraryManager::update_track_metadata_off_mutex(&library, &id, edit).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "non-colour edits stay in the DB"
    );

    // A colour change mirrors into the adopted Markers2 COLOR record.
    let edit = {
        let lib = library.lock().unwrap();
        let mut edit = edit_from(lib.get_track(&id).unwrap().unwrap().metadata());
        edit.color_label = Some(ColorLabel::Blue);
        edit
    };
    LibraryManager::update_track_metadata_off_mutex(&library, &id, edit).unwrap();
    assert_eq!(
        payload(&path).color_rgb,
        Some(ColorLabel::Blue.rgb()),
        "the adopted track colour reaches the file"
    );
}

#[test]
fn a_colour_absent_at_adoption_is_not_invented() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let library = Mutex::new(manager(true));
    let id = {
        let lib = library.lock().unwrap();
        lib.import_file_path(&path).unwrap().id().clone()
    };
    let before = std::fs::read(&path).unwrap();

    let edit = {
        let lib = library.lock().unwrap();
        let mut edit = edit_from(lib.get_track(&id).unwrap().unwrap().metadata());
        edit.color_label = Some(ColorLabel::Red);
        edit
    };
    LibraryManager::update_track_metadata_off_mutex(&library, &id, edit).unwrap();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "no COLOR record is created"
    );
}

// ---------------------------------------------------------------------------
// Grids
// ---------------------------------------------------------------------------

#[test]
fn a_manual_constant_grid_reaches_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato BeatGrid",
            &grid_payload(&[(0.0, 120.0f32.to_bits())]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    assert_eq!(
        lib.serato_adoption_state(&id).unwrap().adopted_kinds & SERATO_KIND_GRID,
        SERATO_KIND_GRID
    );

    lib.save_track_beat_grid(&id, 130.0, 0.5, true).unwrap();
    assert!(mirror_with(&lib, &id, true));

    let grid = payload(&path).beatgrid.expect("native grid");
    assert_eq!(grid.markers.len(), 1);
    match grid.markers[0] {
        codec::serato::SeratoGridMarker::Terminal {
            position_seconds,
            bpm,
        } => {
            assert_eq!(position_seconds, 0.5);
            assert_eq!(bpm, 130.0);
        }
        other => panic!("expected a terminal marker, got {other:?}"),
    }
}

#[test]
fn a_multi_segment_grid_is_left_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let grid = grid_payload(&[(0.0, 16u32), (4.0, 128.0f32.to_bits())]);
    let path = tagged_wav(dir.path(), &[geob_frame("Serato BeatGrid", &grid)]);
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    let segments = crate::beat_grid::get_track_beat_grid(&lib.db, &id)
        .unwrap()
        .expect("imported grid")
        .segments;
    assert_eq!(segments.len(), 2, "the imported grid is segmented");
    let before = frames_of(&path, codec::serato::SeratoFrameKind::BeatGrid);

    // Any mirror trigger re-plans the grid; a variable grid has no native
    // single-marker projection, so the frame must stay untouched.
    lib.save_track_hot_cue(&id, 3, 2000, None, None, None)
        .unwrap();
    assert!(mirror(&lib, &id));
    assert_eq!(
        frames_of(&path, codec::serato::SeratoFrameKind::BeatGrid),
        before
    );
}

// ---------------------------------------------------------------------------
// Planning, staleness and failures
// ---------------------------------------------------------------------------

#[test]
fn a_queued_plan_writes_the_latest_committed_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();

    // Plan while the file still has the imported position…
    let plan = crate::tags::plan_for_track(&lib, &id, false).expect("write-back on");

    // …then commit a newer value that the old plan must not overwrite.
    lib.save_track_hot_cue(
        &id,
        0,
        7000,
        None,
        Some("#00FF00".into()),
        Some("New".into()),
    )
    .unwrap();
    assert!(plan.execute_with_library(&lib).is_some());

    let after = payload(&path);
    assert_eq!(after.cues[0].position_ms, 7000);
    assert_eq!(after.cues[0].label.as_deref(), Some("New"));
}

#[test]
fn turning_write_back_off_cancels_a_queued_plan() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let mut lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    let plan = crate::tags::plan_for_track(&lib, &id, true).expect("plan");
    let before = std::fs::read(&path).unwrap();

    // On → Off while the plan is queued: it must do nothing at all, ordinary
    // fields included.
    lib.set_tag_write_back(TagWriteBack::Off);
    lib.save_track_hot_cue(&id, 0, 8000, None, None, None)
        .unwrap();
    assert!(plan.execute_with_library(&lib).is_none());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn a_file_failure_keeps_the_database_edit() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    let before = std::fs::read(&path).unwrap();

    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).unwrap();

    lib.save_track_hot_cue(&id, 0, 9000, None, None, None)
        .unwrap();
    assert!(!mirror(&lib, &id), "the read-only file refuses the mirror");
    assert_eq!(std::fs::read(&path).unwrap(), before);

    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).unwrap();

    let cues = lib.list_track_hot_cues(&id).unwrap();
    assert_eq!(cues[0].position_ms, 9000, "the DB edit stands");
}

#[test]
fn a_deleted_or_moved_track_skips_the_write() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    let before = std::fs::read(&path).unwrap();

    let plan = crate::tags::plan_for_track(&lib, &id, false).expect("plan");
    lib.save_track_hot_cue(&id, 0, 4000, None, None, None)
        .unwrap();

    // Move the row to another file: the queued plan must refuse to mirror into
    // the old path.
    {
        use crate::entity::{tracks, TrackEntity};
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        let guard = lib.db.conn().unwrap();
        let other = dir.path().join("other.wav");
        write_tagged_wav(&other, &id3_tag(&[]));
        tracks::ActiveModel {
            id: Set(id.as_str().to_string()),
            source_ref: Set(other.to_string_lossy().into_owned()),
            ..Default::default()
        }
        .update(guard.as_connection())
        .unwrap();
        assert!(TrackEntity::find_by_id(id.as_str())
            .one(guard.as_connection())
            .unwrap()
            .is_some());
    }
    assert!(plan.execute_with_library(&lib).is_none());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn off_mutex_execution_does_not_deadlock_with_a_gate_holder() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[("CUE", cue_record(0, 1000, [1, 2, 3], "Intro"))]),
        )],
    );
    let lib = Arc::new(Mutex::new(manager(true)));
    let id = {
        let guard = lib.lock().unwrap();
        guard.import_file_path(&path).unwrap().id().clone()
    };
    let plan = {
        let guard = lib.lock().unwrap();
        guard
            .save_track_hot_cue(&id, 0, 6000, None, None, None)
            .unwrap();
        crate::tags::plan_for_track(&guard, &id, false).expect("plan")
    };

    // The synchronous compatibility path owns the manager while it waits for
    // the gate: the off-mutex writer must release the gate instead of blocking.
    let gate = {
        let guard = lib.lock().unwrap();
        guard.tag_write_gate()
    };
    let holder = {
        let lib = Arc::clone(&lib);
        std::thread::spawn(move || {
            let guard = lib.lock().unwrap();
            let _gate = gate.lock().unwrap();
            std::thread::sleep(Duration::from_millis(50));
            drop(guard);
        })
    };
    std::thread::sleep(Duration::from_millis(20));

    let (tx, rx) = mpsc::channel();
    let writer = {
        let lib = Arc::clone(&lib);
        std::thread::spawn(move || {
            let started = Instant::now();
            let report = plan.execute_off_mutex(&lib);
            tx.send((report.is_some(), started.elapsed())).unwrap();
        })
    };
    let (wrote, elapsed) = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the off-mutex write must finish instead of deadlocking");
    holder.join().unwrap();
    writer.join().unwrap();
    assert!(wrote, "the write ran once the gate was free");
    assert!(elapsed < Duration::from_secs(5));
    assert_eq!(payload(&path).cues[0].position_ms, 6000);
}

#[test]
fn snapshot_projects_only_adopted_kinds() {
    let dir = tempfile::tempdir().unwrap();
    let path = tagged_wav(
        dir.path(),
        &[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[
                ("CUE", cue_record(0, 1000, [1, 2, 3], "Intro")),
                ("COLOR", color_record([0x11, 0x22, 0x33])),
            ]),
        )],
    );
    let lib = manager(true);
    let id = lib.import_file_path(&path).unwrap().id().clone();
    lib.save_track_loop(&id, 0, 1000, 2000, None, None).unwrap();

    let snapshot = super::snapshot(&lib, &id).unwrap().expect("snapshot");
    assert!(snapshot.cues.is_some(), "cues were adopted");
    let loops = snapshot
        .loops
        .as_ref()
        .expect("the markers kind adopts loops");
    assert_eq!(loops.len(), 1, "the saved loop is projected");
    assert_eq!(
        (loops[0].slot, loops[0].in_ms, loops[0].out_ms),
        (0, 1000, 2000)
    );
    assert!(snapshot.grid.is_none(), "no grid was adopted");
    assert_eq!(snapshot.colour, Some(Some(0x0011_2233)));
}
