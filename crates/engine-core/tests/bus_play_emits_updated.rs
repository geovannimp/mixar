//! Integration test: cmd bus Play → evt bus response.

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};
use omnibus::Filter;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn recv_evt_kind(
    sub: &omnibus::BusReceiver<Origin, Kind, std::sync::Arc<[u8]>>,
    kind: Kind,
) -> omnibus::Event<Origin, Kind, std::sync::Arc<[u8]>> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = sub
            .recv_timeout(remaining.min(Duration::from_millis(50)))
            .expect("recv")
            .expect("event");
        if *event.kind() == kind {
            return (*event).clone();
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

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
    assert_eq!(*event.kind(), Kind::Error);
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
    assert_eq!(*event.kind(), Kind::Updated);
    let EvtBody::DeckUpdated { id, playing, .. } =
        decode_evt_body(event.payload()).expect("decode evt body")
    else {
        panic!("expected DeckUpdated body");
    };
    assert_eq!(id, 0);
    assert!(playing);
    assert!(session.revision() > 0);
}
