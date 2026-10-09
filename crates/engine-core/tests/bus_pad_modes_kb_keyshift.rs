//! Integration: Rekordbox page-based Keyboard and Key Shift pad modes (#298).

mod common;

use common::{recv_evt_kind, short_tone_fixture, TestReceiver};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin, PadMode};
use engine_core::{
    key_shift_page_action, key_shift_page_next, key_shift_page_prev, keyboard_page_action,
    keyboard_page_next, keyboard_page_prev, EngineConfig, EngineSession, PitchPadAction,
    DEFAULT_PITCH_PAGE, KEYBOARD_PAGE_COUNT, KEY_SHIFT_PAGE_COUNT,
};
use library::{LibraryConfig, LibrarySession, NewCollection, WritableLibrary};
use library_core::{AudioSource, FileAudioSource, Library, TrackId, TrackMetadata};
use omnibus::Filter;
use std::io::Write;
use std::path::Path;

fn null_session_with_loaded_deck() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    TrackId::new("kbkeyshift.wav"),
                    short_tone_fixture(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )
        })
        .expect("load");
    session
}

/// A two-second silent WAV so hot cues at 1000 ms are seekable.
fn write_silence_wav(path: &Path, seconds: u32) {
    let sample_rate = 48_000u32;
    let sample_count = sample_rate * seconds;
    let pcm = vec![0u8; (sample_count * 2) as usize];
    let data_size = pcm.len() as u32;
    let file_size = 36 + data_size;
    let byte_rate = sample_rate * 2;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&file_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_size.to_le_bytes()).unwrap();
    file.write_all(&pcm).unwrap();
}

/// Library-backed session so hot cues can be saved (needs the library cmd bus).
/// Returns the session plus its owning `LibrarySession`/temp dir and a 1000 ms hot
/// cue in slot 0.
fn library_session_with_root_hot_cue() -> (EngineSession, LibrarySession, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_silence_wav(&wav, 2);

    let library_session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let track_id = {
        let library = library_session.library();
        let mut lib = library.lock().unwrap();
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        lib.list_collection_tracks(&folder.id).unwrap()[0]
            .id()
            .clone()
    };

    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new_with_library_bus(
        config,
        library_session.library(),
        library_session.cmd_bus(),
    )
    .expect("engine session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            engine.seek_deck(0, 1000)?;
            engine.save_deck_hot_cue(0, 0)?;
            Ok(())
        })
        .expect("load + hot cue");

    (session, library_session, dir)
}

fn publish(session: &EngineSession, kind: Kind, body: &CmdBody) {
    session
        .publish_cmd(Origin::Deck(0), kind, encode_cmd_body(body).unwrap())
        .unwrap();
}

fn next_deck_updated(evt: &TestReceiver) -> EvtBody {
    decode_evt_body(recv_evt_kind(evt, Kind::Updated).payload()).expect("decode DeckUpdated")
}

fn key_shift_of(body: &EvtBody) -> f32 {
    let EvtBody::DeckUpdated { key_shift, .. } = body else {
        panic!("expected DeckUpdated");
    };
    *key_shift
}

fn keyboard_page_of(body: &EvtBody) -> u8 {
    let EvtBody::DeckUpdated { keyboard_page, .. } = body else {
        panic!("expected DeckUpdated");
    };
    *keyboard_page
}

fn key_shift_page_of(body: &EvtBody) -> u8 {
    let EvtBody::DeckUpdated { key_shift_page, .. } = body else {
        panic!("expected DeckUpdated");
    };
    *key_shift_page
}

fn deck_position(session: &EngineSession) -> i32 {
    session
        .with_engine(|e| Ok(e.deck_playback_ms(0).map(|(pos, _)| pos)))
        .expect("position call")
        .expect("loaded deck")
}

#[test]
fn keyboard_and_key_shift_page_tables_match_rekordbox() {
    use PitchPadAction::{KeyReset, KeySync, None as NoOp, Semitone, SemitoneDown, SemitoneUp};

    // Keyboard is pitch-only: 4 pages, no utility actions.
    assert_eq!(KEYBOARD_PAGE_COUNT, 4);
    assert_eq!(
        (0..8)
            .map(|s| keyboard_page_action(1, s))
            .collect::<Vec<_>>(),
        vec![
            Semitone(8),
            Semitone(9),
            Semitone(10),
            Semitone(11),
            Semitone(12),
            NoOp,
            NoOp,
            NoOp
        ]
    );
    assert_eq!(
        (0..8)
            .map(|s| keyboard_page_action(2, s))
            .collect::<Vec<_>>(),
        vec![
            Semitone(0),
            Semitone(1),
            Semitone(2),
            Semitone(3),
            Semitone(4),
            Semitone(5),
            Semitone(6),
            Semitone(7)
        ]
    );
    assert_eq!(
        (0..8)
            .map(|s| keyboard_page_action(3, s))
            .collect::<Vec<_>>(),
        vec![
            Semitone(-8),
            Semitone(-7),
            Semitone(-6),
            Semitone(-5),
            Semitone(-4),
            Semitone(-3),
            Semitone(-2),
            Semitone(-1)
        ]
    );
    assert_eq!(
        (0..8)
            .map(|s| keyboard_page_action(4, s))
            .collect::<Vec<_>>(),
        vec![
            NoOp,
            NoOp,
            NoOp,
            NoOp,
            Semitone(-12),
            Semitone(-11),
            Semitone(-10),
            Semitone(-9)
        ]
    );
    for page in 1..=KEYBOARD_PAGE_COUNT {
        for slot in 0..8 {
            assert!(
                !matches!(
                    keyboard_page_action(page, slot),
                    KeyReset | SemitoneUp | SemitoneDown | KeySync
                ),
                "Keyboard page {page} slot {slot} must be pitch-only"
            );
        }
    }

    // Key Shift shares pages 1–4 and adds the utility page as page 5.
    assert_eq!(KEY_SHIFT_PAGE_COUNT, 5);
    for page in 1..=4 {
        for slot in 0..8 {
            assert_eq!(
                key_shift_page_action(page, slot),
                keyboard_page_action(page, slot),
                "Key Shift page {page} slot {slot} must match Keyboard"
            );
        }
    }
    assert_eq!(
        (0..8)
            .map(|s| key_shift_page_action(5, s))
            .collect::<Vec<_>>(),
        vec![
            KeyReset,
            SemitoneDown,
            Semitone(-5),
            Semitone(-12),
            KeySync,
            SemitoneUp,
            Semitone(7),
            Semitone(12)
        ]
    );

    // Per-mode page wrapping.
    assert_eq!(keyboard_page_next(4), 1);
    assert_eq!(keyboard_page_next(2), 3);
    assert_eq!(keyboard_page_prev(1), KEYBOARD_PAGE_COUNT);
    assert_eq!(keyboard_page_prev(3), 2);
    assert_eq!(key_shift_page_next(5), 1);
    assert_eq!(key_shift_page_next(2), 3);
    assert_eq!(key_shift_page_prev(1), KEY_SHIFT_PAGE_COUNT);
    assert_eq!(key_shift_page_prev(3), 2);

    // Out-of-range pages clamp into each mode's own range.
    assert_eq!(keyboard_page_action(0, 0), Semitone(8));
    assert_eq!(keyboard_page_action(9, 7), Semitone(-9));
    assert_eq!(key_shift_page_action(0, 0), Semitone(8));
    assert_eq!(key_shift_page_action(9, 7), Semitone(12));
    assert_eq!(DEFAULT_PITCH_PAGE, 2);
}

#[test]
fn key_shift_default_page_slots_and_page_actions() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    publish(
        &session,
        Kind::SetPadMode,
        &CmdBody::SetPadMode {
            mode: PadMode::KeyShift,
        },
    );
    let _ = next_deck_updated(&evt);

    // Default page 2: slot 1 → +1, slot 7 → +7.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 1,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 1.0).abs() < 1e-6);
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 7,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 7.0).abs() < 1e-6);

    // Page 3 slot 0 → -8.
    publish(
        &session,
        Kind::SetKeyShiftPage,
        &CmdBody::SetKeyShiftPage { page: 3 },
    );
    assert_eq!(key_shift_page_of(&next_deck_updated(&evt)), 3);
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 0,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) + 8.0).abs() < 1e-6);

    // Page 5: slot 0 → Reset, slot 1 → Down.
    publish(
        &session,
        Kind::SetKeyShiftPage,
        &CmdBody::SetKeyShiftPage { page: 5 },
    );
    assert_eq!(key_shift_page_of(&next_deck_updated(&evt)), 5);
    publish(
        &session,
        Kind::SetKeyShift,
        &CmdBody::SetKeyShift { semitones: 7.0 },
    );
    let _ = next_deck_updated(&evt);
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 0,
            shift: false,
        },
    );
    assert!(key_shift_of(&next_deck_updated(&evt)).abs() < 1e-6);
    publish(
        &session,
        Kind::SetKeyShift,
        &CmdBody::SetKeyShift { semitones: 7.0 },
    );
    let _ = next_deck_updated(&evt);
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 1,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 6.0).abs() < 1e-6);
}

#[test]
fn key_shift_shift_bank_switches_page() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    // Default page 2 → slot 6 = next = 3.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 6,
            shift: true,
        },
    );
    assert_eq!(key_shift_page_of(&next_deck_updated(&evt)), 3);
    // slot 7 = prev = 2.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 7,
            shift: true,
        },
    );
    assert_eq!(key_shift_page_of(&next_deck_updated(&evt)), 2);
    // Other shift-bank slots are no-ops.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 0,
            shift: true,
        },
    );
    assert_eq!(key_shift_page_of(&next_deck_updated(&evt)), 2);
    // Wrapping from page 1: prev → 5.
    publish(
        &session,
        Kind::SetKeyShiftPage,
        &CmdBody::SetKeyShiftPage { page: 1 },
    );
    let _ = next_deck_updated(&evt);
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 7,
            shift: true,
        },
    );
    assert_eq!(key_shift_page_of(&next_deck_updated(&evt)), 5);

    // The Key Shift shift-bank page switch wraps at 5 (not 4): 5 → 1.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 6,
            shift: true,
        },
    );
    assert_eq!(key_shift_page_of(&next_deck_updated(&evt)), 1);
}

#[test]
fn keyboard_and_key_shift_pages_are_independent() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Kind::SetKeyboardPage,
        &CmdBody::SetKeyboardPage { page: 3 },
    );
    let body = next_deck_updated(&evt);
    assert_eq!(keyboard_page_of(&body), 3);
    assert_eq!(key_shift_page_of(&body), DEFAULT_PITCH_PAGE);

    publish(
        &session,
        Kind::SetKeyShiftPage,
        &CmdBody::SetKeyShiftPage { page: 1 },
    );
    let body = next_deck_updated(&evt);
    assert_eq!(keyboard_page_of(&body), 3);
    assert_eq!(key_shift_page_of(&body), 1);

    // A Key Shift shift-bank page switch advances only the Key Shift page.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 6,
            shift: true,
        },
    );
    let body = next_deck_updated(&evt);
    assert_eq!(keyboard_page_of(&body), 3);
    assert_eq!(key_shift_page_of(&body), 2);

    // A Keyboard shift-bank page switch advances only the Keyboard page.
    publish(
        &session,
        Kind::SetPadMode,
        &CmdBody::SetPadMode {
            mode: PadMode::Keyboard,
        },
    );
    let _ = next_deck_updated(&evt);
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 6,
            shift: true,
        },
    );
    let body = next_deck_updated(&evt);
    assert_eq!(keyboard_page_of(&body), 4);
    assert_eq!(key_shift_page_of(&body), 2);

    // The Keyboard shift-bank page switch wraps at 4 (not 5).
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 6,
            shift: true,
        },
    );
    let body = next_deck_updated(&evt);
    assert_eq!(keyboard_page_of(&body), 1);
    assert_eq!(key_shift_page_of(&body), 2);
}

#[test]
fn keyboard_pad_seeks_to_root_and_restores() {
    let (session, _library, _dir) = library_session_with_root_hot_cue();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    // Latch Key Shift +2 (page 2, slot 2).
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 2,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 2.0).abs() < 1e-6);
    publish(
        &session,
        Kind::SetPadMode,
        &CmdBody::SetPadMode {
            mode: PadMode::Keyboard,
        },
    );
    let _ = next_deck_updated(&evt);

    // Default page 2, slot 4 → +4 semitones; seeks to the root hot cue (1000 ms).
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 4,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 4.0).abs() < 1e-6);
    assert!(
        deck_position(&session) >= 900,
        "expected seek to root hot cue, got {}",
        deck_position(&session)
    );

    // Move away, then release: gate returns to the root and restores the latch.
    session
        .with_engine(|e| e.seek_deck(0, 0))
        .expect("seek away");
    publish(
        &session,
        Kind::KeyboardPadRelease,
        &CmdBody::KeyboardPadRelease { slot: 4 },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 2.0).abs() < 1e-6);
    assert!(
        deck_position(&session) >= 900,
        "expected gate return to root, got {}",
        deck_position(&session)
    );
}

#[test]
fn keyboard_shift_bank_deletes_root_hot_cue() {
    let (session, _library, _dir) = library_session_with_root_hot_cue();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 0,
            shift: true,
        },
    );
    let _ = next_deck_updated(&evt);

    let snap = session
        .with_engine(|e| Ok(e.deck_snapshot(0).expect("snapshot")))
        .expect("snapshot call");
    assert!(
        snap.hot_cues.is_empty(),
        "shift+pad 1 must delete hot cue 0, got {:?}",
        snap.hot_cues
    );
}

#[test]
fn mode_switch_clears_held_keyboard_state() {
    let (session, _library, _dir) = library_session_with_root_hot_cue();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Kind::SetPadMode,
        &CmdBody::SetPadMode {
            mode: PadMode::Keyboard,
        },
    );
    let _ = next_deck_updated(&evt);
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 4,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 4.0).abs() < 1e-6);

    // Switch mode mid-hold; the held state must be discarded and the pre-press
    // shift (0, nothing latched) restored rather than left at the pad's +4.
    publish(
        &session,
        Kind::SetPadMode,
        &CmdBody::SetPadMode {
            mode: PadMode::HotCue,
        },
    );
    let _ = next_deck_updated(&evt);
    publish(
        &session,
        Kind::KeyboardPadRelease,
        &CmdBody::KeyboardPadRelease { slot: 4 },
    );
    let snap = session
        .with_engine(|e| Ok(e.deck_snapshot(0).expect("snapshot")))
        .expect("snapshot call");
    assert!(
        snap.key_shift.abs() < 1e-6,
        "mode switch must restore the pre-press shift, got {}",
        snap.key_shift
    );
}

#[test]
fn mode_switch_restores_latched_key_shift() {
    let (session, _library, _dir) = library_session_with_root_hot_cue();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    // Latch Key Shift +2 (default page 2, slot 2).
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 2,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 2.0).abs() < 1e-6);
    publish(
        &session,
        Kind::SetPadMode,
        &CmdBody::SetPadMode {
            mode: PadMode::Keyboard,
        },
    );
    let _ = next_deck_updated(&evt);

    // Hold a Keyboard pad (+4) on top of the latch.
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 4,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 4.0).abs() < 1e-6);

    // Switching mode mid-hold must restore the latched +2, not 0 and not +4.
    publish(
        &session,
        Kind::SetPadMode,
        &CmdBody::SetPadMode {
            mode: PadMode::HotCue,
        },
    );
    let _ = next_deck_updated(&evt);
    let snap = session
        .with_engine(|e| Ok(e.deck_snapshot(0).expect("snapshot")))
        .expect("snapshot call");
    assert!(
        (snap.key_shift - 2.0).abs() < 1e-6,
        "mode switch must restore the latched shift, got {}",
        snap.key_shift
    );
}
