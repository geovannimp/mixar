//! Integration: SetKeyShift / SetKeyboardScale publish on DeckUpdated.

mod common;

use common::{recv_evt_kind, short_tone_fixture};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, KeyboardScale, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
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
fn set_key_shift_and_scale_publish_fields() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyShift,
            encode_cmd_body(&CmdBody::SetKeyShift { semitones: 2.0 }).unwrap(),
        )
        .unwrap();
    let body = decode_evt_body(recv_evt_kind(&evt, Kind::Updated).payload()).unwrap();
    let EvtBody::DeckUpdated { key_shift, .. } = body else {
        panic!("DeckUpdated")
    };
    assert!((key_shift - 2.0).abs() < 1e-6);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyboardScale,
            encode_cmd_body(&CmdBody::SetKeyboardScale {
                scale: KeyboardScale::Minor,
            })
            .unwrap(),
        )
        .unwrap();
    let body = decode_evt_body(recv_evt_kind(&evt, Kind::Updated).payload()).unwrap();
    let EvtBody::DeckUpdated { keyboard_scale, .. } = body else {
        panic!("DeckUpdated")
    };
    assert_eq!(keyboard_scale, KeyboardScale::Minor);
}

#[test]
fn unload_resets_key_shift_and_scale() {
    let session = null_session_with_loaded_deck();
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyShift,
            encode_cmd_body(&CmdBody::SetKeyShift { semitones: 12.0 }).unwrap(),
        )
        .unwrap();
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyboardScale,
            encode_cmd_body(&CmdBody::SetKeyboardScale {
                scale: KeyboardScale::Pentatonic,
            })
            .unwrap(),
        )
        .unwrap();

    session.with_engine(|e| e.unload_deck(0)).expect("unload");
    let snap = session
        .with_engine(|e| Ok(e.deck_snapshot(0).expect("snapshot")))
        .expect("snapshot call");
    assert_eq!(snap.key_shift, 0.0);
    assert_eq!(snap.keyboard_scale, KeyboardScale::Major);
}

#[test]
fn load_resets_key_shift_and_scale() {
    let session = null_session_with_loaded_deck();
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyShift,
            encode_cmd_body(&CmdBody::SetKeyShift { semitones: 12.0 }).unwrap(),
        )
        .unwrap();
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyboardScale,
            encode_cmd_body(&CmdBody::SetKeyboardScale {
                scale: KeyboardScale::Pentatonic,
            })
            .unwrap(),
        )
        .unwrap();

    // Loading a second track must not inherit the session shift/scale.
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
    assert_eq!(snap.keyboard_scale, KeyboardScale::Major);
}
