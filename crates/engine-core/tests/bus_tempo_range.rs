//! Integration: SetTempoRange publishes tempo_range on DeckUpdated.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_error;
use common::recv_updated;
use common::short_tone_fixture;
use engine_api::Event;
use engine_api::SetSpeed;
use engine_api::SetTempoRange;
use engine_core::EngineConfig;
use engine_core::EngineSession;
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
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetSpeed(SetSpeed {
            deck: 0,
            speed: 0.25,
            soft_takeover: false,
        }))
        .expect("set speed");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(Event::SetTempoRange(SetTempoRange {
            deck: 0,
            tempo_range: 0.123,
        }))
        .expect("set range");

    let update = recv_updated(&mut evt);
    let tempo_range = update.tempo_range;
    let speed = update.speed;
    assert!(
        (tempo_range - 0.123).abs() < 1e-5,
        "tempo_range={tempo_range}"
    );
    assert!((speed - 0.25).abs() < 1e-5, "speed={speed}");
}

#[test]
fn set_tempo_range_rejects_non_positive_and_non_finite() {
    let session = null_session_with_loaded_deck();
    let mut evt = session.subscribe_evt_all().expect("sub");

    for bad in [0.0_f32, -0.06, f32::NAN, f32::INFINITY] {
        session
            .publish_cmd(Event::SetTempoRange(SetTempoRange {
                deck: 0,
                tempo_range: bad,
            }))
            .expect("publish");
        let error = recv_error(&mut evt);
        assert!(
            error.message.contains("tempo_range"),
            "message={} bad={bad}",
            error.message
        );
    }
}
