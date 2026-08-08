//! Integration: SetTempoRange publishes tempo_range on DeckUpdated.

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
                    TrackId::new("range.wav"),
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
fn set_tempo_range_keeps_speed_and_publishes_fraction() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetSpeed,
            encode_cmd_body(&CmdBody::SetSpeed {
                speed: 0.25,
                soft_takeover: false,
            })
            .unwrap(),
        )
        .expect("set speed");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetTempoRange,
            encode_cmd_body(&CmdBody::SetTempoRange { tempo_range: 0.123 }).unwrap(),
        )
        .expect("set range");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        tempo_range, speed, ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(
        (tempo_range - 0.123).abs() < 1e-5,
        "tempo_range={tempo_range}"
    );
    assert!((speed - 0.25).abs() < 1e-5, "speed={speed}");
}

#[test]
fn set_tempo_range_rejects_non_positive_and_non_finite() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    for bad in [0.0_f32, -0.06, f32::NAN, f32::INFINITY] {
        session
            .publish_cmd(
                Origin::Deck(0),
                Kind::SetTempoRange,
                encode_cmd_body(&CmdBody::SetTempoRange { tempo_range: bad }).unwrap(),
            )
            .expect("publish");
        let event = recv_evt_kind(&evt, Kind::Error);
        assert_eq!(*event.kind(), Kind::Error);
        let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("decode") else {
            panic!("expected Error body for {bad}");
        };
        assert!(
            message.contains("tempo_range"),
            "message={message} bad={bad}"
        );
    }
}
