//! Integration: SetKeyLock publishes key_lock on DeckUpdated.

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
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
            Ok(Some(event)) => {
                if *event.kind() == kind {
                    return (*event).clone();
                }
            }
            Ok(None) => {}
            Err(e) => panic!("recv: {e}"),
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

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
                    TrackId::new("keylock.wav"),
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
fn set_key_lock_publishes_enabled_flag() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyLock,
            encode_cmd_body(&CmdBody::SetKeyLock { enabled: true }).unwrap(),
        )
        .expect("set key lock");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { key_lock, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(key_lock);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetKeyLock,
            encode_cmd_body(&CmdBody::SetKeyLock { enabled: false }).unwrap(),
        )
        .expect("clear key lock");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { key_lock, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(!key_lock);
}
