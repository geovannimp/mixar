//! Integration: SetKeyLock publishes key_lock on DeckUpdated.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_updated;
use common::short_tone_fixture;
use engine_api::Event;
use engine_api::SetKeyLock;
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
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetKeyLock(SetKeyLock {
            deck: 0,
            enabled: true,
        }))
        .expect("set key lock");

    let update = recv_updated(&mut evt);
    assert_eq!(update.deck, 0);
    assert!(update.key_lock);

    session
        .publish_cmd(Event::SetKeyLock(SetKeyLock {
            deck: 0,
            enabled: false,
        }))
        .expect("clear key lock");

    let update = recv_updated(&mut evt);
    assert_eq!(update.deck, 0);
    assert!(!update.key_lock);
}
