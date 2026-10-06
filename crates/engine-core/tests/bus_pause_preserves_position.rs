mod common;

use common::{recv_evt_kind, short_tone_fixture};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};
use omnibus::Filter;
use std::thread;
use std::time::Duration;

#[test]
fn pause_preserves_playback_position() {
    let session = EngineSession::new(EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    })
    .expect("session");
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
            engine.seek_deck(0, 50)?;
            Ok(())
        })
        .expect("load");

    let empty = encode_cmd_body(&CmdBody::Empty).unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::Play, empty.clone())
        .expect("play");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    // Let the null backend advance a bit.
    thread::sleep(Duration::from_millis(80));
    session
        .publish_cmd(Origin::Deck(0), Kind::Pause, empty)
        .expect("pause");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        position_ms,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(!playing);
    let pos = position_ms.expect("position");
    assert!(
        pos >= 40,
        "pause should not reset to start, got position_ms={pos}"
    );
}
