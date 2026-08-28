//! Vinyl jog while paused must publish Position so UI playhead/time track live.

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
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match sub.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(Some(event)) if *event.kind() == kind => return (*event).clone(),
            Ok(Some(_)) | Ok(None) | Err(_) => {}
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

#[test]
fn paused_vinyl_jog_touch_publishes_position_before_release() {
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
            engine.seek_deck(0, 80)?;
            Ok(())
        })
        .expect("load");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::JogTouch,
            encode_cmd_body(&CmdBody::JogTouch { touching: true }).unwrap(),
        )
        .expect("touch");
    let touch = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        jog_touching,
        ..
    } = decode_evt_body(touch.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(!playing);
    assert!(jog_touching);

    // Scratch ticks are Silent (no DeckUpdated). UI must still get Position while
    // touched — otherwise waveform/time only jump on jog-touch release.
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::JogTurn,
            encode_cmd_body(&CmdBody::JogTurn { delta: 90 }).unwrap(),
        )
        .expect("turn");

    let pos = recv_evt_kind(&evt, Kind::Position);
    let EvtBody::Position { position_ms } = decode_evt_body(pos.payload()).expect("decode") else {
        panic!("expected Position");
    };
    assert!(position_ms >= 0);
}
