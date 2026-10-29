//! Integration: SetKeyShift / SetKeyboardPage / SetKeyShiftPage / SetKeyboardRoot publish on DeckUpdated.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::SetKeyShift;
use bus_api::SetKeyShiftPage;
use bus_api::SetKeyboardPage;
use bus_api::SetKeyboardRoot;
use common::recv_updated;
use common::short_tone_fixture;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use engine_core::DEFAULT_PITCH_PAGE;
use engine_core::KEYBOARD_PAGE_COUNT;
use library_core::AudioSource;
use library_core::FileAudioSource;
use library_core::TrackId;
use library_core::TrackMetadata;

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
                    TrackId::new("keyshift.wav"),
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

#[test]
fn set_key_shift_pages_and_root_publish_fields() {
    let session = null_session_with_loaded_deck();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetKeyShift(SetKeyShift {
            deck: 0,
            semitones: 2.0,
        }))
        .unwrap();
    let update = recv_updated(&mut evt);
    assert!((update.key_shift - 2.0).abs() < 1e-6);

    session
        .publish_cmd(Event::SetKeyboardPage(SetKeyboardPage { deck: 0, page: 4 }))
        .unwrap();
    let update = recv_updated(&mut evt);
    assert_eq!(update.keyboard_page, 4);
    // The Key Shift page is untouched by a Keyboard page command.
    assert_eq!(update.key_shift_page, DEFAULT_PITCH_PAGE);

    session
        .publish_cmd(Event::SetKeyShiftPage(SetKeyShiftPage { deck: 0, page: 1 }))
        .unwrap();
    let update = recv_updated(&mut evt);
    assert_eq!(update.keyboard_page, 4);
    assert_eq!(update.key_shift_page, 1);

    session
        .publish_cmd(Event::SetKeyboardRoot(SetKeyboardRoot { deck: 0, slot: 3 }))
        .unwrap();
    let update = recv_updated(&mut evt);
    assert_eq!(update.keyboard_root_hot_cue, 3);
}

#[test]
fn pitch_pad_setters_clamp_out_of_range_values() {
    let session = null_session_with_loaded_deck();
    session
        .with_engine(|engine| {
            engine.set_deck_keyboard_page(0, 99)?;
            engine.set_deck_key_shift_page(0, 0)?;
            engine.set_deck_keyboard_root(0, 99)?;
            engine.set_deck_key_shift(0, 999.0)?;
            Ok(())
        })
        .expect("set out-of-range");
    let snap = session
        .with_engine(|e| Ok(e.deck_snapshot(0).expect("snapshot")))
        .expect("snapshot call");
    assert_eq!(snap.keyboard_page, KEYBOARD_PAGE_COUNT);
    assert_eq!(snap.key_shift_page, 1);
    assert_eq!(snap.keyboard_root_hot_cue, 15);
    assert_eq!(snap.key_shift, 16.0);
}

#[test]
fn unload_resets_key_shift_pages_and_root() {
    let session = null_session_with_loaded_deck();
    // Set the pitch-pad state synchronously: `publish_cmd` is fire-and-forget, so
    // bus cmds could otherwise be processed after `unload_deck`.
    session
        .with_engine(|engine| {
            engine.set_deck_key_shift(0, 12.0)?;
            engine.set_deck_keyboard_page(0, 4)?;
            engine.set_deck_key_shift_page(0, 1)?;
            engine.set_deck_keyboard_root(0, 3)?;
            Ok(())
        })
        .expect("set pitch pad state");

    session.with_engine(|e| e.unload_deck(0)).expect("unload");
    let snap = session
        .with_engine(|e| Ok(e.deck_snapshot(0).expect("snapshot")))
        .expect("snapshot call");
    assert_eq!(snap.key_shift, 0.0);
    assert_eq!(snap.keyboard_page, DEFAULT_PITCH_PAGE);
    assert_eq!(snap.key_shift_page, DEFAULT_PITCH_PAGE);
    assert_eq!(snap.keyboard_root_hot_cue, 0);
}

#[test]
fn load_resets_key_shift_pages_and_root() {
    let session = null_session_with_loaded_deck();
    // Set the pitch-pad state synchronously: `publish_cmd` is fire-and-forget, so
    // bus cmds could otherwise be processed after `load_track` and the assertion
    // below would see the stale page.
    session
        .with_engine(|engine| {
            engine.set_deck_key_shift(0, 12.0)?;
            engine.set_deck_keyboard_page(0, 4)?;
            engine.set_deck_key_shift_page(0, 1)?;
            engine.set_deck_keyboard_root(0, 2)?;
            Ok(())
        })
        .expect("set pitch pad state");

    // Loading a second track must not inherit the session shift/page.
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    TrackId::new("keyshift2.wav"),
                    short_tone_fixture(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )
        })
        .expect("load second");
    let snap = session
        .with_engine(|e| Ok(e.deck_snapshot(0).expect("snapshot")))
        .expect("snapshot call");
    assert_eq!(snap.key_shift, 0.0);
    assert_eq!(snap.keyboard_page, DEFAULT_PITCH_PAGE);
    assert_eq!(snap.key_shift_page, DEFAULT_PITCH_PAGE);
    assert_eq!(snap.keyboard_root_hot_cue, 0);
}
