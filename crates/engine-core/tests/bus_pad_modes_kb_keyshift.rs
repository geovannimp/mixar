//! Integration: Keyboard and Key Shift pad modes publish on DeckUpdated.

mod common;

use common::{recv_evt_kind, short_tone_fixture, TestReceiver};
use engine_api::{
    decode_evt_body, encode_cmd_body, CmdBody, EvtBody, KeyboardScale, Kind, Origin, PadMode,
};
use engine_core::{keyboard_scale_degrees, EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
use omnibus::Filter;

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

#[test]
fn key_shift_pad_latches_and_clears() {
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
    let EvtBody::DeckUpdated { pad_mode, .. } = next_deck_updated(&evt) else {
        panic!("DeckUpdated")
    };
    assert_eq!(pad_mode, PadMode::KeyShift);

    // Press latches the pad's semitone offset.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 2,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 2.0).abs() < 1e-6);

    // Re-press of the active pad clears to 0.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 2,
            shift: false,
        },
    );
    assert!(key_shift_of(&next_deck_updated(&evt)).abs() < 1e-6);

    // Negative offset pad latches -3.0 (slot 5 per KEY_SHIFT_PAD_SEMITONES).
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 5,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) + 3.0).abs() < 1e-6);

    // Shift bank resets to 0.
    publish(
        &session,
        Kind::KeyShiftPadPress,
        &CmdBody::KeyShiftPadPress {
            slot: 5,
            shift: true,
        },
    );
    assert!(key_shift_of(&next_deck_updated(&evt)).abs() < 1e-6);
}

#[test]
fn keyboard_pad_sets_scale_degree_then_release_clears() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Kind::SetKeyboardScale,
        &CmdBody::SetKeyboardScale {
            scale: KeyboardScale::Major,
        },
    );
    let _ = next_deck_updated(&evt);

    // Major degree 4 = 7 semitones.
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 4,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 7.0).abs() < 1e-6);

    // Release clears the momentary pitch offset.
    publish(
        &session,
        Kind::KeyboardPadRelease,
        &CmdBody::KeyboardPadRelease { slot: 4 },
    );
    assert!(key_shift_of(&next_deck_updated(&evt)).abs() < 1e-6);

    // Shift bank slot 0 selects the major scale (switch away first to observe it).
    publish(
        &session,
        Kind::SetKeyboardScale,
        &CmdBody::SetKeyboardScale {
            scale: KeyboardScale::Minor,
        },
    );
    let _ = next_deck_updated(&evt);
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 0,
            shift: true,
        },
    );
    let EvtBody::DeckUpdated { keyboard_scale, .. } = next_deck_updated(&evt) else {
        panic!("DeckUpdated")
    };
    assert_eq!(keyboard_scale, KeyboardScale::Major);
}

#[test]
fn keyboard_pentatonic_top_slot_is_not_clamped() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Kind::SetKeyboardScale,
        &CmdBody::SetKeyboardScale {
            scale: KeyboardScale::Pentatonic,
        },
    );
    let _ = next_deck_updated(&evt);

    // Pentatonic slot 7 = +16 semitones; the clamp must reach it.
    publish(
        &session,
        Kind::KeyboardPadPress,
        &CmdBody::KeyboardPadPress {
            slot: 7,
            shift: false,
        },
    );
    assert!((key_shift_of(&next_deck_updated(&evt)) - 16.0).abs() < 1e-6);
}

#[test]
fn keyboard_scale_tables_match_spec() {
    assert_eq!(
        keyboard_scale_degrees(KeyboardScale::Major),
        [0, 2, 4, 5, 7, 9, 11, 12]
    );
    assert_eq!(
        keyboard_scale_degrees(KeyboardScale::Minor),
        [0, 2, 3, 5, 7, 8, 10, 12]
    );
    assert_eq!(
        keyboard_scale_degrees(KeyboardScale::Pentatonic),
        [0, 2, 4, 7, 9, 12, 14, 16]
    );
}
