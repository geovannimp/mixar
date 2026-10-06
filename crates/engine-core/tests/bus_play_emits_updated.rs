//! Integration test: cmd bus Play → evt bus response.

mod common;

use common::{recv_evt_kind, short_tone_fixture};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};
use omnibus::Filter;

fn null_config() -> EngineConfig {
    EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    }
}

#[test]
fn play_on_empty_deck_publishes_track_error() {
    let session = EngineSession::new(null_config()).expect("session");
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::Play, body)
        .expect("publish");
    let event = recv_evt_kind(&evt, Kind::Error);
    assert_eq!(*event.origin(), Origin::Deck(0));
    let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("decode evt body")
    else {
        panic!("expected Error body");
    };
    let lower = message.to_lowercase();
    assert!(
        lower.contains("track") || lower.contains("load"),
        "expected track/load error, got: {message}"
    );
}

#[test]
fn play_with_track_loaded_publishes_updated_playing() {
    let session = EngineSession::new(null_config()).expect("session");
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    session
        .with_engine(|engine| {
            engine.start()?;
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::from_path(short_tone_fixture())),
            )?;
            Ok(())
        })
        .expect("setup");
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::Play, body)
        .expect("publish");
    let event = recv_evt_kind(&evt, Kind::Updated);
    assert_eq!(*event.origin(), Origin::Deck(0));
    let EvtBody::DeckUpdated { id, playing, .. } =
        decode_evt_body(event.payload()).expect("decode evt body")
    else {
        panic!("expected DeckUpdated body");
    };
    assert_eq!(id, 0);
    assert!(playing);
    assert!(session.revision() > 0);
}

#[test]
fn set_crossfader_publishes_status() {
    let session = EngineSession::new(null_config()).expect("session");
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    let body = encode_cmd_body(&CmdBody::SetCrossfader {
        position: 0.75,
        soft_takeover: false,
    })
    .unwrap();
    session
        .publish_cmd(Origin::Mixer, Kind::SetCrossfader, body)
        .expect("publish");
    let event = recv_evt_kind(&evt, Kind::Status);
    assert_eq!(*event.origin(), Origin::Mixer);
    let EvtBody::EngineStatus { status } =
        decode_evt_body(event.payload()).expect("decode evt body")
    else {
        panic!("expected EngineStatus body");
    };
    assert!((status.crossfader - 0.75).abs() < f32::EPSILON);
    assert!(session.revision() > 0);
}
