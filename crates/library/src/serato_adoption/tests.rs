//! DB-backed tests for per-file Serato adoption (#743).
//!
//! Fixtures and builders come from the reader work in #742 (`crate::tests`), so
//! this module never re-implements a parser. Every file is copied or written
//! into a temp dir; repository samples are never modified.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseBackend, EntityTrait, Statement};

use crate::entity::{tracks, TrackEntity};
use crate::tests::{
    geob_frame, id3_tag, serato_fixture, stored_grid, tea_leaves_tag, write_minimal_wav,
    write_tagged_wav,
};
use crate::{
    beat_grid, deck_data, list_hot_cues, list_loops, Library, LibraryConfig, LibraryError,
    LibraryManager, TagWriteBack, TrackId, TrackMetadata, TrackMetadataEdit, WritableLibrary,
    SERATO_KIND_COLOUR, SERATO_KIND_CUES, SERATO_KIND_GRID, SERATO_KIND_LOOPS,
};
use library_core::ColorLabel;

use super::{start_counting, stop_counting, ApplyOutcome, SeratoAdoptionState};

fn column_name(column: impl sea_orm::ColumnTrait) -> String {
    column.as_column_ref().1.to_string()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn library() -> LibraryManager {
    LibraryManager::open_in_memory(LibraryConfig::default()).unwrap()
}

fn normalized(path: &Path) -> PathBuf {
    crate::normalize_path(path).unwrap()
}

/// Insert a row exactly as a pre-#743 library would have: no adoption marker.
fn seed_with(lib: &LibraryManager, path: &Path, metadata: TrackMetadata) -> TrackId {
    let path = normalized(path);
    let id = LibraryManager::track_id_for(&path);
    lib.store()
        .upsert_file_track(
            &id,
            &path,
            &metadata,
            crate::store::TrackMetadataOrigin::Tag,
            "1",
        )
        .unwrap();
    id
}

fn seed_unmarked(lib: &LibraryManager, path: &Path) -> TrackId {
    let metadata = crate::tags::read_tags(path).unwrap();
    seed_with(lib, path, metadata)
}

fn set_marker(lib: &LibraryManager, id: &TrackId, at: Option<&str>, kinds: Option<i32>) {
    let guard = lib.db.conn().unwrap();
    tracks::ActiveModel {
        id: sea_orm::Set(id.as_str().to_string()),
        serato_imported_at: sea_orm::Set(at.map(str::to_string)),
        serato_adopted_kinds: sea_orm::Set(kinds),
        ..Default::default()
    }
    .update(guard.as_connection())
    .unwrap();
}

fn set_colour(lib: &LibraryManager, id: &TrackId, label: Option<&str>, rgb: Option<i32>) {
    let guard = lib.db.conn().unwrap();
    tracks::ActiveModel {
        id: sea_orm::Set(id.as_str().to_string()),
        color_label: sea_orm::Set(label.map(str::to_string)),
        color_rgb: sea_orm::Set(rgb),
        ..Default::default()
    }
    .update(guard.as_connection())
    .unwrap();
}

fn set_source_ref(lib: &LibraryManager, id: &TrackId, source_ref: &str) {
    let guard = lib.db.conn().unwrap();
    tracks::ActiveModel {
        id: sea_orm::Set(id.as_str().to_string()),
        source_ref: sea_orm::Set(source_ref.to_string()),
        ..Default::default()
    }
    .update(guard.as_connection())
    .unwrap();
}

fn cue_map(lib: &LibraryManager, id: &TrackId) -> BTreeMap<u8, deck_data::HotCueRecord> {
    list_hot_cues(&lib.db, id)
        .unwrap()
        .into_iter()
        .map(|cue| (cue.slot_index, cue))
        .collect()
}

fn state(lib: &LibraryManager, id: &TrackId) -> SeratoAdoptionState {
    lib.serato_adoption_state(id).unwrap()
}

fn assert_marked(lib: &LibraryManager, id: &TrackId) -> SeratoAdoptionState {
    let state = state(lib, id);
    assert!(state.imported_at.is_some(), "adoption timestamp is set");
    state
}

/// Run one off-mutex entry point and hand the manager back.
///
/// Off-mutex APIs take `&Mutex<LibraryManager>`; the wrap/call/`into_inner`
/// triple belongs here so a missed `into_inner` cannot poison later assertions.
fn with_off_mutex<T>(
    lib: LibraryManager,
    f: impl FnOnce(&Mutex<LibraryManager>) -> T,
) -> (LibraryManager, T) {
    let library = Mutex::new(lib);
    let value = f(&library);
    (library.into_inner().unwrap(), value)
}

/// `Serato Markers2` logical payload, base64-wrapped exactly as Serato writes it.
fn markers2_payload(records: &[(&[u8], &[u8])]) -> Vec<u8> {
    use base64::Engine as _;

    let mut decoded = vec![1u8, 1];
    for (name, payload) in records {
        decoded.extend_from_slice(name);
        decoded.push(0);
        decoded.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        decoded.extend_from_slice(payload);
    }
    decoded.push(0);
    let mut out = vec![1u8, 1];
    out.extend_from_slice(
        base64::engine::general_purpose::STANDARD
            .encode(decoded)
            .as_bytes(),
    );
    out
}

fn colour_record(rgb: [u8; 3]) -> Vec<u8> {
    vec![0, rgb[0], rgb[1], rgb[2]]
}

fn mik_cue_points_body() -> Vec<u8> {
    use base64::Engine as _;
    let json = serde_json::json!({
        "source": "mixedinkey",
        "cues": [{"time": 1000.0, "name": "Intro"}, {"time": 2000.0, "name": "Drop"}],
    });
    base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_vec(&json).unwrap())
        .into_bytes()
}

/// A tagged WAV carrying one Markers2 colour, no cues: label + exact RGB.
fn write_colour_wav(path: &Path) {
    write_tagged_wav(
        path,
        &id3_tag(&[geob_frame(
            "Serato Markers2",
            &markers2_payload(&[(b"COLOR", &colour_record([0xFF, 0x00, 0x00]))]),
        )]),
    );
}

/// A tagged WAV with the imported grid and a red track colour.
fn write_grid_and_colour_wav(path: &Path) {
    write_tagged_wav(
        path,
        &id3_tag(&[
            geob_frame(
                "Serato Markers2",
                &markers2_payload(&[(b"COLOR", &colour_record([0xFF, 0x00, 0x00]))]),
            ),
            geob_frame(
                "Serato BeatGrid",
                &serato_fixture("beatgrid/manual-beatgrid-changing-tempo.octet-stream"),
            ),
        ]),
    );
}

fn sha(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    // A plain digest is enough to prove the bytes did not move; avoid pulling a
    // crypto dependency into the library's dev-deps for one assertion.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

// ---------------------------------------------------------------------------
// 1. Schema upgrade, no eager work
// ---------------------------------------------------------------------------

#[test]
fn legacy_database_gains_nullable_adoption_columns_without_eager_work() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("legacy.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());
    let db_path = dir.path().join("library.db");

    let id = {
        let lib = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();
        let id = seed_unmarked(&lib, &wav);
        // A library-only slot outside the Serato range, a loop, a grid and a
        // colour: everything a populated pre-feature library can hold.
        deck_data::save_hot_cue(&lib.db, &id, 12, 3_210, None, Some("#123456".into()), None)
            .unwrap();
        deck_data::save_loop(&lib.db, &id, 3, 100, 900, Some("loop".into()), None).unwrap();
        let grid = beat_grid::from_constant(128.0, 500, 4, 4, 500, None, crate::SOURCE_MANUAL);
        beat_grid::write_track_beat_grid(&lib.db, &id, &grid, crate::SOURCE_MANUAL).unwrap();
        set_colour(&lib, &id, Some("aqua"), Some(0x00_FFFF));
        id
    };

    // Emulate the database written before this feature existed: raw SQLite drops
    // the columns and forgets only the baseline, so reopening runs it again.
    {
        use sea_orm::EntityName;

        let table = tracks::Entity.table_name();
        let imported_at = column_name(tracks::Column::SeratoImportedAt);
        let adopted_kinds = column_name(tracks::Column::SeratoAdoptedKinds);
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let mut sql = String::new();
        for column in [imported_at, adopted_kinds] {
            sql.push_str(&format!("ALTER TABLE {table} DROP COLUMN {column};\n"));
        }
        sql.push_str(&format!(
            "DELETE FROM {ledger} WHERE {version} = '{}';",
            crate::migrations::BASELINE_VERSION,
            ledger = crate::migrations::LEDGER_TABLE,
            version = crate::migrations::VERSION_COLUMN
        ));
        conn.execute_batch(&sql).unwrap();
    }

    let lib = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();
    // Columns exist again, both nullable.
    let columns: Vec<String> = {
        let guard = lib.db.conn().unwrap();
        {
            guard.as_connection().query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA table_info(tracks)".to_string(),
            ))
        }
        .unwrap()
        .iter()
        .map(|row| row.try_get::<String>("", "name").unwrap())
        .collect()
    };
    assert!(columns.iter().any(|c| c == "serato_imported_at"));
    assert!(columns.iter().any(|c| c == "serato_adopted_kinds"));
    assert_eq!(
        state(&lib, &id),
        SeratoAdoptionState {
            imported_at: None,
            adopted_kinds: 0
        }
    );

    // Every existing row survived untouched, and opening did not read the file.
    assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 1);
    assert_eq!(list_loops(&lib.db, &id).unwrap().len(), 1);
    assert_eq!(stored_grid(&lib, &id).source, crate::SOURCE_MANUAL);
    let metadata = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(metadata.metadata().color_label, Some(ColorLabel::Aqua));
    assert_eq!(metadata.metadata().color_rgb, Some(0x00_FFFF));

    // A close/reopen keeps it unmarked and still does not import file cues.
    drop(lib);
    let lib = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();
    assert_eq!(state(&lib, &id).imported_at, None);
    assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// 2. Every lazy path adopts an unmarked legacy row
// ---------------------------------------------------------------------------

fn assert_file_cues_imported(lib: &LibraryManager, id: &TrackId) {
    let state = assert_marked(lib, id);
    assert_eq!(state.adopted_kinds & SERATO_KIND_CUES, SERATO_KIND_CUES);
    let cues = cue_map(lib, id);
    assert_eq!(cues.len(), 8);
    assert_eq!(cues[&0].position_ms, 510);
    assert_eq!(cues[&7].position_ms, 182_777);
}

#[test]
fn direct_import_and_refresh_adopt() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("refresh.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    lib.refresh_file_source(&wav).unwrap();
    assert_file_cues_imported(&lib, &id);

    // A second import keeps the same state and does not duplicate rows.
    let marker = state(&lib, &id);
    lib.import_file_path(&wav).unwrap();
    assert_eq!(state(&lib, &id), marker);
    assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 8);
}

#[test]
fn off_mutex_import_and_refresh_adopt() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("off-mutex.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::refresh_file_off_mutex(library, &wav).unwrap();
    });
    assert_file_cues_imported(&lib, &id);
}

#[test]
fn folder_and_playlist_sync_adopt() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("sync.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let mut lib = library();
    let id = seed_unmarked(&lib, &wav);
    let folder = lib
        .add_collection(&crate::NewCollection::folder_named(dir.path(), "Music"))
        .unwrap();
    lib.sync_collection(Some(&folder.id)).unwrap();
    assert_file_cues_imported(&lib, &id);

    // A playlist sync takes the off-mutex path.
    set_marker(&lib, &id, None, None);
    for slot in 0..8u8 {
        deck_data::delete_hot_cue(&lib.db, &id, slot).unwrap();
    }
    let playlist = lib
        .add_collection(&crate::NewCollection::playlist("Set", false))
        .unwrap();
    lib.add_collection_entry(&playlist.id, &id, None).unwrap();
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::sync_collection_off_mutex(library, Some(&playlist.id)).unwrap();
    });
    assert_file_cues_imported(&lib, &id);
}

#[test]
fn deck_load_adopts_even_when_analysis_is_not_needed() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("load.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    #[cfg(feature = "analysis")]
    seed_completed_analysis(&lib, &id, &wav);
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::prepare_track_for_playback(library, &id).unwrap();
    });
    assert_file_cues_imported(&lib, &id);
}

#[test]
fn file_path_deck_load_adopts() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("path-load.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    let (lib, prepared) = with_off_mutex(lib, |library| {
        LibraryManager::prepare_file_path_for_playback(library, &wav).unwrap()
    });
    assert_eq!(prepared.track_id, id);
    assert_file_cues_imported(&lib, &id);
}

#[cfg(feature = "analysis")]
#[test]
fn direct_and_off_mutex_analysis_adopt() {
    let dir = tempfile::tempdir().unwrap();
    let direct = dir.path().join("direct.wav");
    write_tagged_wav(&direct, &tea_leaves_tag());
    let off_mutex = dir.path().join("off-mutex.wav");
    std::fs::copy(&direct, &off_mutex).unwrap();

    let lib = library();
    let direct_id = seed_unmarked(&lib, &direct);
    lib.analyze_file_source(&direct, crate::AnalyzeTrackOptions::default())
        .unwrap();
    assert_file_cues_imported(&lib, &direct_id);

    let off_id = seed_unmarked(&lib, &off_mutex);
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::analyze_track_off_mutex(
            library,
            &off_id,
            crate::AnalyzeTrackOptions::default(),
        )
        .unwrap();
    });
    assert_file_cues_imported(&lib, &off_id);
}

#[cfg(feature = "analysis")]
#[test]
fn worker_analyze_adopts() {
    use bus_api::{AnalyzeTrack, Command};
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("worker.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let session = crate::LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let id = {
        let library = session.library();
        let lib = library.lock().unwrap();
        seed_unmarked(&lib, &wav)
    };
    session
        .publish_cmd(Command::AnalyzeTrack(AnalyzeTrack {
            track_id: id.as_str().to_string(),
            force: false,
        }))
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        {
            let library = session.library();
            let lib = library.lock().unwrap();
            if type_has_marker(&lib, &id) {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "the worker's AnalyzeTrack entry must adopt"
        );
        std::thread::sleep(Duration::from_millis(25));
    }

    let library = session.library();
    let lib = library.lock().unwrap();
    assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 8);
}

fn type_has_marker(lib: &LibraryManager, id: &TrackId) -> bool {
    state(lib, id).imported_at.is_some()
}

#[cfg(feature = "analysis")]
fn seed_completed_analysis(lib: &LibraryManager, id: &TrackId, path: &Path) {
    // Completed analysis with a cue and loudness is enough even if detection
    // found no BPM or key. Adoption must then come from deck load itself.
    let mut computed = crate::tests::detector_fixture(path, 117.0, "F#m", false);
    computed.analysis.bpm = None;
    computed.analysis.key = None;
    computed.analysis.beat_grid = None;
    crate::analysis::persist_analysis_result(&lib.db, id, &computed).unwrap();
    assert!(lib.store().has_completed_analysis(id).unwrap());
    assert!(!lib.needs_playback_analysis(id).unwrap());
}

// ---------------------------------------------------------------------------
// 3. Merge and writer handoff
// ---------------------------------------------------------------------------

#[test]
fn adoption_fills_vacancies_and_keeps_everything_occupied() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("merge.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    // Slot 0 conflicts with the file and must keep its whole row.
    deck_data::save_hot_cue(
        &lib.db,
        &id,
        0,
        999,
        Some(2),
        Some("#ABCDEF".into()),
        Some("mine".into()),
    )
    .unwrap();
    // Library-only slots outside the Serato range.
    for slot in 8..16u8 {
        deck_data::save_hot_cue(
            &lib.db,
            &id,
            slot,
            i32::from(slot) * 1_000,
            None,
            None,
            Some("db only".into()),
        )
        .unwrap();
    }
    deck_data::save_loop(&lib.db, &id, 0, 10, 90, Some("db loop".into()), None).unwrap();
    let manual = beat_grid::from_constant(140.0, 1_000, 4, 4, 1_000, None, crate::SOURCE_MANUAL);
    beat_grid::write_track_beat_grid(&lib.db, &id, &manual, crate::SOURCE_MANUAL).unwrap();
    set_colour(&lib, &id, Some("aqua"), Some(0x00_FFFF));

    lib.ensure_serato_adopted(&id).unwrap();

    let cues = cue_map(&lib, &id);
    assert_eq!(cues.len(), 16, "all library slots survive adoption");
    let slot0 = &cues[&0];
    assert_eq!(slot0.position_ms, 999);
    assert_eq!(slot0.loop_length_beats, Some(2));
    assert_eq!(slot0.color.as_deref(), Some("#ABCDEF"));
    assert_eq!(slot0.label.as_deref(), Some("mine"));
    assert_eq!(cues[&1].position_ms, 12_661, "vacant slots are filled");
    for slot in 8..16u8 {
        assert_eq!(cues[&slot].position_ms, i32::from(slot) * 1_000);
        assert_eq!(cues[&slot].label.as_deref(), Some("db only"));
    }

    let loops = list_loops(&lib.db, &id).unwrap();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].in_ms, 10);

    assert_eq!(
        stored_grid(&lib, &id),
        manual,
        "an existing grid is verbatim"
    );
    let metadata = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(metadata.metadata().color_label, Some(ColorLabel::Aqua));
    assert_eq!(metadata.metadata().color_rgb, Some(0x00_FFFF));

    // The mask comes from the file, not from how many rows were filled.
    assert_eq!(
        state(&lib, &id).adopted_kinds,
        SERATO_KIND_CUES | SERATO_KIND_LOOPS | SERATO_KIND_GRID | SERATO_KIND_COLOUR
    );
}

#[test]
fn first_hot_cue_save_lands_after_adoption() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("first-save.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    lib.save_track_hot_cue(&id, 0, 4_242, None, None, Some("saved".into()))
        .unwrap();

    let cues = cue_map(&lib, &id);
    assert_eq!(cues.len(), 8);
    assert_eq!(cues[&0].position_ms, 4_242, "the save owns its target");
    assert_eq!(cues[&0].label.as_deref(), Some("saved"));
    assert_eq!(
        cues[&1].position_ms, 12_661,
        "other file cues were imported"
    );
    assert_marked(&lib, &id);
}

// ---------------------------------------------------------------------------
// 4. Never resurrect
// ---------------------------------------------------------------------------

fn clear_everything(lib: &LibraryManager, id: &TrackId) {
    for slot in 0..16u8 {
        lib.delete_track_hot_cue(id, slot).unwrap();
    }
    for slot in 0..16u8 {
        lib.delete_track_loop(id, slot).unwrap();
    }
}

#[test]
fn adopted_state_is_never_resurrected() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("never.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());
    let db_path = dir.path().join("library.db");

    let id;
    let marker;
    {
        let lib = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();
        id = lib.import_file_path(&wav).unwrap().id().clone();
        marker = assert_marked(&lib, &id);
        clear_everything(&lib, &id);

        // Rescan and a second import.
        lib.refresh_file_source(&wav).unwrap();
        lib.import_file_path(&wav).unwrap();
        assert!(list_hot_cues(&lib.db, &id).unwrap().is_empty());
        assert!(list_loops(&lib.db, &id).unwrap().is_empty());

        // Deck load.
        let (lib, ()) = with_off_mutex(lib, |library| {
            LibraryManager::prepare_file_path_for_playback(library, &wav).unwrap();
        });
        assert!(list_hot_cues(&lib.db, &id).unwrap().is_empty());

        // Forced analysis.
        #[cfg(feature = "analysis")]
        lib.analyze_file_source(&wav, crate::AnalyzeTrackOptions::force())
            .unwrap();
        assert!(list_hot_cues(&lib.db, &id).unwrap().is_empty());
        assert_eq!(state(&lib, &id), marker);
    }

    // Close and reopen: the marker and the cleared rows persist.
    let lib = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();
    assert_eq!(state(&lib, &id), marker);
    assert!(list_hot_cues(&lib.db, &id).unwrap().is_empty());
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::prepare_file_path_for_playback(library, &wav).unwrap();
    });
    assert!(list_hot_cues(&lib.db, &id).unwrap().is_empty());
    assert_eq!(state(&lib, &id), marker);
}

#[test]
fn mik_fallback_never_resurrects_cleared_cues() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("mik.wav");
    write_tagged_wav(
        &wav,
        &id3_tag(&[geob_frame("CuePoints", &mik_cue_points_body())]),
    );

    for write_back in [TagWriteBack::Off, TagWriteBack::On] {
        let lib = LibraryManager::open_in_memory(LibraryConfig {
            tag_write_back: write_back,
        })
        .unwrap();
        let id = lib.import_file_path(&wav).unwrap().id().clone();
        assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 2);
        // Mixed In Key is a fallback, never a Serato kind.
        assert_eq!(assert_marked(&lib, &id).adopted_kinds, 0);

        clear_everything(&lib, &id);
        lib.refresh_file_source(&wav).unwrap();
        let (lib, ()) = with_off_mutex(lib, |library| {
            LibraryManager::prepare_file_path_for_playback(library, &wav).unwrap();
        });
        assert!(
            list_hot_cues(&lib.db, &id).unwrap().is_empty(),
            "MIK cues must not come back ({write_back:?})"
        );
    }
}

#[test]
fn first_edit_is_a_delete() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("first-delete.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    lib.delete_track_hot_cue(&id, 0).unwrap();

    let cues = cue_map(&lib, &id);
    assert_eq!(cues.len(), 7);
    assert!(!cues.contains_key(&0), "the cleared slot stays cleared");
    assert_marked(&lib, &id);
}

#[test]
fn first_colour_clear_clears_the_whole_pair() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("clear.wav");
    write_colour_wav(&wav);

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    // Load adopts the file colour, which the deck/UI then shows.
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::prepare_file_path_for_playback(library, &wav).unwrap();
    });
    let adopted = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(adopted.metadata().color_label, Some(ColorLabel::Red));
    assert_eq!(adopted.metadata().color_rgb, Some(0xFF_0000));

    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            color_label: None,
            ..TrackMetadataEdit::default()
        },
    )
    .unwrap();
    let cleared = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(cleared.metadata().color_label, None);
    assert_eq!(cleared.metadata().color_rgb, None);

    // Rescan and load cannot bring it back.
    lib.refresh_file_source(&wav).unwrap();
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::prepare_file_path_for_playback(library, &wav).unwrap();
    });
    assert_eq!(
        lib.get_track(&id).unwrap().unwrap().metadata().color_rgb,
        None
    );
}

// ---------------------------------------------------------------------------
// 5. Zero-kind adoption and provenance
// ---------------------------------------------------------------------------

#[test]
fn readable_file_without_serato_frames_is_adopted_with_zero_kinds() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("plain.wav");
    write_minimal_wav(&wav);

    let lib = library();
    let source = lib.import_file_path(&wav).unwrap();
    let state = assert_marked(&lib, &source.id().clone());
    assert_eq!(state.adopted_kinds, 0);
    assert!(list_hot_cues(&lib.db, source.id()).unwrap().is_empty());
}

#[test]
fn successfully_empty_serato_markers_count_as_adopted() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("empty-markers.wav");
    let tag = id3_tag(&[geob_frame("Serato Markers2", &markers2_payload(&[]))]);
    write_tagged_wav(&wav, &tag);
    // The fixture really is a valid, empty frame.
    let payload = codec::read_file_serato(&wav).unwrap();
    assert!(payload.scan_complete);
    assert_eq!(payload.frames[0].status, codec::serato::FrameStatus::Valid);
    assert!(payload.cues.is_empty() && payload.loops.is_empty());

    let lib = library();
    let source = lib.import_file_path(&wav).unwrap();
    let state = assert_marked(&lib, &source.id().clone());
    assert_eq!(
        state.adopted_kinds,
        SERATO_KIND_CUES | SERATO_KIND_LOOPS,
        "an understood kind stays adopted even when empty"
    );
    assert!(list_hot_cues(&lib.db, source.id()).unwrap().is_empty());
}

#[test]
fn malformed_markers_do_not_fail_an_import_and_grant_no_kind() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("malformed.wav");
    let tag = id3_tag(&[geob_frame("Serato Markers2", b"\x01\x01!!!!")]);
    write_tagged_wav(&wav, &tag);

    let lib = library();
    let source = lib.import_file_path(&wav).unwrap();
    let state = assert_marked(&lib, &source.id().clone());
    assert_eq!(state.adopted_kinds, 0);
    assert!(list_hot_cues(&lib.db, source.id()).unwrap().is_empty());
}

#[test]
fn unsupported_container_stays_unmarked() {
    let dir = tempfile::tempdir().unwrap();
    // A native stem bundle is outside the containers the reader understands.
    let bundle = dir.path().join("bundle.stem.mp4");
    std::fs::write(&bundle, b"not an ID3 container").unwrap();

    let lib = library();
    let id = seed_with(&lib, &bundle, TrackMetadata::default());
    lib.ensure_serato_adopted(&id).unwrap();
    assert_eq!(
        state(&lib, &id),
        SeratoAdoptionState {
            imported_at: None,
            adopted_kinds: 0
        },
        "no container means no evidence, so no marker and no kind"
    );

    // The same rule inside the transaction: a payload without a container is
    // not applicable and writes nothing.
    let outcome = {
        let guard = lib.db.conn().unwrap();
        super::apply_payload_in_tx(
            guard.as_connection(),
            &id,
            &normalized(&bundle),
            &codec::SeratoPayload::default(),
        )
        .unwrap()
    };
    assert_eq!(outcome, ApplyOutcome::NotApplicable);
    assert_eq!(state(&lib, &id).imported_at, None);
}

#[test]
fn stream_rows_are_not_applicable() {
    let lib = library();
    let id = TrackId::new("stream:http://radio.example/stream");
    {
        let guard = lib.db.conn().unwrap();
        {
            tracks::ActiveModel {
                id: sea_orm::Set(id.as_str().to_string()),
                source_type: sea_orm::Set("stream".into()),
                source_ref: sea_orm::Set("http://radio.example/stream".into()),
                added_at: sea_orm::Set("1".into()),
                updated_at: sea_orm::Set("1".into()),
                ..Default::default()
            }
            .insert(guard.as_connection())
            .unwrap();
        };
    }
    lib.ensure_serato_adopted(&id).unwrap();
    assert_eq!(state(&lib, &id).imported_at, None);
}

#[test]
fn an_incomplete_scan_is_deferred_not_adopted() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("truncated.wav");
    let mut tag = id3_tag(&[geob_frame("Serato Markers2", &markers2_payload(&[]))]);
    // Claim a frame larger than the tag: the scan cannot trust its absence.
    tag[14..18].copy_from_slice(&[0x7F, 0x7F, 0x7F, 0x7F]);
    write_tagged_wav(&wav, &tag);
    let payload = codec::read_file_serato(&wav).unwrap();
    assert!(!payload.scan_complete, "fixture is a partial scan");
    assert!(payload.container.is_some());

    let lib = library();
    let id = seed_with(&lib, &wav, TrackMetadata::default());
    let error = lib.ensure_serato_adopted(&id).unwrap_err();
    assert!(
        error.to_string().contains("deferred"),
        "unexpected error: {error}"
    );
    assert_eq!(state(&lib, &id).imported_at, None);
}

// ---------------------------------------------------------------------------
// 6. Grid and colour preservation
// ---------------------------------------------------------------------------

#[test]
fn an_occupied_colour_is_never_replaced_by_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("occupied-colour.wav");
    write_colour_wav(&wav);

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    set_colour(&lib, &id, Some("aqua"), Some(0x00_FFFF));
    lib.ensure_serato_adopted(&id).unwrap();

    let metadata = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(metadata.metadata().color_label, Some(ColorLabel::Aqua));
    assert_eq!(metadata.metadata().color_rgb, Some(0x00_FFFF));
    assert_eq!(
        state(&lib, &id).adopted_kinds & SERATO_KIND_COLOUR,
        SERATO_KIND_COLOUR,
        "the file kind is still understood"
    );
}

#[test]
fn an_edited_null_colour_suppresses_adoption() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("edited-null.wav");
    write_colour_wav(&wav);

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    // A title-only edit makes the row authoritative, leaving colour NULL.
    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            title: Some("Renamed".into()),
            ..TrackMetadataEdit::default()
        },
    )
    .unwrap();
    lib.ensure_serato_adopted(&id).unwrap();

    let metadata = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(metadata.metadata().color_label, None);
    assert_eq!(metadata.metadata().color_rgb, None);
}

#[test]
fn first_colour_change_keeps_the_adopted_grid_bpm() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("rebase.wav");
    write_grid_and_colour_wav(&wav);

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    // The user saw no BPM/key (nothing adopted yet) and picks a colour: the
    // payload carries those pre-adoption values.
    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            color_label: Some(ColorLabel::Blue),
            bpm: None,
            key: None,
            ..TrackMetadataEdit::default()
        },
    )
    .unwrap();

    let adopted_grid = stored_grid(&lib, &id);
    let adopted_bpm = adopted_grid.segments[0].bpm;
    let stored = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(stored.metadata().bpm, Some(adopted_bpm));
    assert_eq!(stored.metadata().key, None, "existing rows keep their key");
    assert_eq!(stored.metadata().color_label, Some(ColorLabel::Blue));
    assert_eq!(stored.metadata().color_rgb, Some(ColorLabel::Blue.rgb()));
    assert_eq!(adopted_grid.source, crate::SOURCE_IMPORT);

    // The metadata editor cannot replace a segmented grid; the key is not
    // partially saved when the tempo is rejected.
    let error = lib
        .update_track_metadata(
            &id,
            TrackMetadataEdit {
                color_label: Some(ColorLabel::Blue),
                bpm: Some(128.0),
                key: Some("Am".into()),
                ..TrackMetadataEdit::default()
            },
        )
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("edit the segmented beat grid in the deck"));
    let stored = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(stored.metadata().bpm, Some(adopted_bpm));
    assert_eq!(stored.metadata().key, None);

    // Clearing the colour keeps the current BPM it was sent with.
    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            color_label: None,
            bpm: Some(adopted_bpm),
            key: None,
            ..TrackMetadataEdit::default()
        },
    )
    .unwrap();
    let stored = lib.get_track(&id).unwrap().unwrap();
    assert_eq!(stored.metadata().color_label, None);
    assert_eq!(stored.metadata().color_rgb, None);
    assert_eq!(stored.metadata().bpm, Some(adopted_bpm));
    assert_eq!(stored_grid(&lib, &id), adopted_grid);
}

#[cfg(feature = "analysis")]
#[test]
fn analysis_cannot_restore_a_cleared_colour() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("analyse-colour.wav");
    write_colour_wav(&wav);

    let lib = library();
    let source = lib.import_file_path(&wav).unwrap();
    let id = source.id().clone();
    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            color_label: None,
            ..TrackMetadataEdit::default()
        },
    )
    .unwrap();
    assert_eq!(
        lib.get_track(&id).unwrap().unwrap().metadata().color_rgb,
        None
    );

    lib.analyze_file_source(&wav, crate::AnalyzeTrackOptions::force())
        .unwrap();
    assert_eq!(
        lib.get_track(&id).unwrap().unwrap().metadata().color_rgb,
        None,
        "analysis must not restore a cleared colour"
    );
}

// ---------------------------------------------------------------------------
// 7. Failure, atomicity and races
// ---------------------------------------------------------------------------

#[test]
fn first_edits_fail_without_mutation_on_an_unreadable_file() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("gone.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    std::fs::remove_file(&wav).unwrap();

    let cue_error = lib
        .save_track_hot_cue(&id, 0, 1_000, None, None, None)
        .unwrap_err();
    assert!(matches!(cue_error, LibraryError::Backend { .. }));
    assert!(lib.save_track_loop(&id, 0, 10, 90, None, None).is_err());
    assert!(lib.save_track_beat_grid(&id, 128.0, 0.0, false).is_err());
    assert!(lib
        .update_track_metadata(
            &id,
            TrackMetadataEdit {
                color_label: Some(ColorLabel::Blue),
                ..TrackMetadataEdit::default()
            },
        )
        .is_err());

    assert!(list_hot_cues(&lib.db, &id).unwrap().is_empty());
    assert!(list_loops(&lib.db, &id).unwrap().is_empty());
    assert!(beat_grid::get_track_beat_grid(&lib.db, &id)
        .unwrap()
        .is_none());
    assert_eq!(
        lib.get_track(&id).unwrap().unwrap().metadata().color_label,
        None
    );
    assert_eq!(
        state(&lib, &id).imported_at,
        None,
        "no fake zero-kind adoption"
    );
}

#[test]
fn a_marked_file_can_be_edited_offline() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("offline.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = lib.import_file_path(&wav).unwrap().id().clone();
    assert_marked(&lib, &id);
    std::fs::remove_file(&wav).unwrap();

    lib.save_track_hot_cue(&id, 0, 7_777, None, None, Some("offline".into()))
        .unwrap();
    lib.delete_track_loop(&id, 0).unwrap();
    lib.save_track_beat_grid(&id, 129.0, 0.0, true).unwrap();
    let cues = cue_map(&lib, &id);
    assert_eq!(cues[&0].position_ms, 7_777, "no reader access was needed");
    assert_eq!(stored_grid(&lib, &id).segments[0].bpm, 129.0);
}

#[test]
fn a_deferred_read_is_attempted_once_per_entry_and_retried_later() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("retry.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());
    let hidden = dir.path().join("hidden.wav");

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    std::fs::rename(&wav, &hidden).unwrap();

    let (lib, (result, attempts)) = with_off_mutex(lib, |library| {
        start_counting();
        let result = LibraryManager::prepare_track_for_playback(library, &id);
        (result, stop_counting())
    });
    assert!(
        result.is_err(),
        "a missing file cannot prepare for playback"
    );
    assert_eq!(attempts, 1, "one adoption attempt per load entry");
    assert_eq!(state(&lib, &id).imported_at, None);

    std::fs::rename(&hidden, &wav).unwrap();
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::prepare_track_for_playback(library, &id).unwrap();
    });
    assert_marked(&lib, &id);
    assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 8);
}

#[cfg(feature = "analysis")]
#[test]
fn analysis_adopts_from_its_own_read_without_a_retry() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("analysis-read.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    let (lib, attempts) = with_off_mutex(lib, |library| {
        start_counting();
        LibraryManager::analyze_track_off_mutex(
            library,
            &id,
            crate::AnalyzeTrackOptions::default(),
        )
        .unwrap();
        stop_counting()
    });
    assert_eq!(
        attempts, 0,
        "discovery's payload is reused; persistence must not re-read under the lock"
    );
    assert_marked(&lib, &id);
    assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 8);
}

#[test]
fn a_database_failure_rolls_back_every_mapped_row() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("atomic.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    install_marker_failure(&lib);

    let error = lib.ensure_serato_adopted(&id).unwrap_err();
    assert!(matches!(error, LibraryError::Backend { .. }));
    assert!(
        list_hot_cues(&lib.db, &id).unwrap().is_empty(),
        "inserted cues rolled back with the marker"
    );
    assert!(beat_grid::get_track_beat_grid(&lib.db, &id)
        .unwrap()
        .is_none());
    assert_eq!(state(&lib, &id).imported_at, None);

    drop_marker_failure(&lib);
    lib.ensure_serato_adopted(&id).unwrap();
    assert_eq!(list_hot_cues(&lib.db, &id).unwrap().len(), 8);
    assert_marked(&lib, &id);
}

fn install_marker_failure(lib: &LibraryManager) {
    use sea_orm::EntityName;

    // The table and column names come from the entity, so a rename surfaces here
    // instead of leaving this trigger silently unable to fire (a false pass).
    let table = tracks::Entity.table_name();
    let imported_at = column_name(tracks::Column::SeratoImportedAt);
    let adopted_kinds = column_name(tracks::Column::SeratoAdoptedKinds);
    let guard = lib.db.conn().unwrap();
    guard
        .as_connection()
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!(
                "CREATE TRIGGER fail_adoption BEFORE UPDATE OF {imported_at} ON {table} \
                 WHEN NEW.{adopted_kinds} IS NOT NULL \
                 BEGIN SELECT RAISE(ABORT, 'injected adoption failure'); END;"
            ),
        ))
        .unwrap();
}

fn drop_marker_failure(lib: &LibraryManager) {
    let guard = lib.db.conn().unwrap();
    guard
        .as_connection()
        .execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "DROP TRIGGER fail_adoption".to_string(),
        ))
        .unwrap();
}

#[test]
fn a_competing_adoption_after_deletion_does_not_reinsert() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("competing.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());
    let payload = codec::read_file_serato(&wav).unwrap();
    let path = normalized(&wav);

    let lib = library();
    let id = lib.import_file_path(&wav).unwrap().id().clone();
    lib.delete_track_hot_cue(&id, 0).unwrap();
    let marker = state(&lib, &id);

    let outcome = {
        let guard = lib.db.conn().unwrap();
        super::apply_payload_in_tx(guard.as_connection(), &id, &path, &payload).unwrap()
    };
    assert_eq!(outcome, ApplyOutcome::AlreadyAdopted);
    assert!(!cue_map(&lib, &id).contains_key(&0), "no resurrection");
    assert_eq!(state(&lib, &id), marker);
}

#[test]
fn a_row_deleted_or_moved_while_reading_gets_no_stale_apply() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("stale.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());
    let payload = codec::read_file_serato(&wav).unwrap();
    let path = normalized(&wav);

    let lib = library();
    let id = seed_unmarked(&lib, &wav);

    // Path changed after the payload was read.
    set_source_ref(&lib, &id, "/music/somewhere-else.wav");
    {
        let guard = lib.db.conn().unwrap();
        let outcome = super::apply_payload_in_tx(guard.as_connection(), &id, &path, &payload);
        assert!(matches!(outcome, Err(LibraryError::NotFound(_))));
    }
    assert!(list_hot_cues(&lib.db, &id).unwrap().is_empty());
    assert_eq!(state(&lib, &id).imported_at, None);

    // Row deleted after the payload was read.
    {
        let guard = lib.db.conn().unwrap();
        TrackEntity::delete_by_id(id.as_str())
            .exec(guard.as_connection())
            .unwrap();
        let outcome = super::apply_payload_in_tx(guard.as_connection(), &id, &path, &payload);
        assert!(matches!(outcome, Err(LibraryError::NotFound(_))));
    }
    assert!(lib.get_track(&id).unwrap().is_none());
}

// ---------------------------------------------------------------------------
// 8. No writer activation
// ---------------------------------------------------------------------------

#[test]
fn adoption_and_deck_edits_never_touch_audio_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("bytes.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());
    let before = sha(&std::fs::read(&wav).unwrap());

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    lib.ensure_serato_adopted(&id).unwrap();
    assert_eq!(sha(&std::fs::read(&wav).unwrap()), before, "adoption");

    lib.refresh_file_source(&wav).unwrap();
    let (lib, ()) = with_off_mutex(lib, |library| {
        LibraryManager::prepare_file_path_for_playback(library, &wav).unwrap();
    });
    assert_eq!(sha(&std::fs::read(&wav).unwrap()), before, "rescan + load");

    lib.save_track_hot_cue(&id, 0, 1_234, None, None, Some("cue".into()))
        .unwrap();
    lib.save_track_loop(&id, 0, 10, 90, None, None).unwrap();
    lib.delete_track_loop(&id, 0).unwrap();
    lib.save_track_beat_grid(&id, 130.0, 0.0, true).unwrap();
    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            color_label: Some(ColorLabel::Pink),
            ..TrackMetadataEdit::default()
        },
    )
    .unwrap();
    assert_eq!(
        sha(&std::fs::read(&wav).unwrap()),
        before,
        "deck and editor writes stay in the database"
    );
}

#[test]
fn adoption_never_writes_even_with_write_back_on() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("write-back-on.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());
    let before = sha(&std::fs::read(&wav).unwrap());

    let lib = LibraryManager::open_in_memory(LibraryConfig {
        tag_write_back: TagWriteBack::On,
    })
    .unwrap();
    let id = seed_unmarked(&lib, &wav);
    lib.ensure_serato_adopted(&id).unwrap();
    lib.delete_track_hot_cue(&id, 0).unwrap();
    lib.update_track_metadata(
        &id,
        TrackMetadataEdit {
            color_label: Some(ColorLabel::Pink),
            ..TrackMetadataEdit::default()
        },
    )
    .unwrap();

    assert_eq!(sha(&std::fs::read(&wav).unwrap()), before);
}

// ---------------------------------------------------------------------------
// State contract
// ---------------------------------------------------------------------------

#[test]
fn the_state_getter_reports_corrupt_masks_and_unknown_ids() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("mask.wav");
    write_minimal_wav(&wav);

    let lib = library();
    let id = seed_unmarked(&lib, &wav);
    // NULL mask means "never adopted", exposed as zero.
    assert_eq!(
        state(&lib, &id),
        SeratoAdoptionState {
            imported_at: None,
            adopted_kinds: 0
        }
    );

    for raw in [-1i32, 0x10, 0x1FF] {
        set_marker(&lib, &id, Some("1"), Some(raw));
        match lib.serato_adoption_state(&id).unwrap_err() {
            LibraryError::Backend { backend, message } => {
                assert_eq!(backend, "library");
                assert_eq!(message, "invalid Serato adoption mask");
            }
            other => panic!("unexpected error for mask {raw:#x}: {other}"),
        }
    }

    let missing = TrackId::new("/music/unknown.wav");
    assert!(matches!(
        lib.serato_adoption_state(&missing).unwrap_err(),
        LibraryError::NotFound(_)
    ));
}

#[test]
fn recreating_a_deleted_track_opens_a_new_adoption() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("recreated.wav");
    write_tagged_wav(&wav, &tea_leaves_tag());

    let lib = library();
    let id = lib.import_file_path(&wav).unwrap().id().clone();
    assert_marked(&lib, &id);
    clear_everything(&lib, &id);
    beat_grid::delete_track_beat_grid(&lib.db, &id).unwrap();

    // Delete the row itself: a recreated track is a new adoption opportunity.
    {
        let guard = lib.db.conn().unwrap();
        TrackEntity::delete_by_id(id.as_str())
            .exec(guard.as_connection())
            .unwrap();
    }
    assert!(lib.get_track(&id).unwrap().is_none());

    let recreated = lib.import_file_path(&wav).unwrap().id().clone();
    assert_eq!(recreated, id, "path identity is unchanged");
    assert_marked(&lib, &recreated);
    assert_eq!(list_hot_cues(&lib.db, &recreated).unwrap().len(), 8);
}
