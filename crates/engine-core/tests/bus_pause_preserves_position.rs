use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};
use omnibus::Filter;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

fn recv_evt_kind(
    sub: &omnibus::BusReceiver<Origin, Kind, std::sync::Arc<[u8]>>,
    kind: Kind,
) -> omnibus::Event<Origin, Kind, std::sync::Arc<[u8]>> {
    let deadline = Instant::now() + Duration::from_secs(2);
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
            engine.seek_deck(0, 0.05)?;
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
        position_secs,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(!playing);
    let pos = position_secs.expect("position");
    assert!(
        pos >= 0.04,
        "pause should not reset to start, got position_secs={pos}"
    );
}
