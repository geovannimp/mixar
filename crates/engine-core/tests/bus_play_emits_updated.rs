//! Integration test: cmd bus Play → evt bus response.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::Play;
use bus_api::SetCrossfader;
use common::recv_error;
use common::recv_snapshot;
use common::recv_updated;
use common::short_tone_fixture;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library_core::AudioSource;
use library_core::FileAudioSource;

fn null_config() -> EngineConfig {
    EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    }
}

#[test]
fn play_on_empty_deck_publishes_track_error() {
    let session = EngineSession::new(null_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .publish_cmd(Event::Play(Play { deck: 0 }))
        .expect("publish");
    let error = recv_error(&mut evt);
    assert_eq!(error.deck, Some(0));
    let lower = error.message.to_lowercase();
    assert!(
        lower.contains("track") || lower.contains("load"),
        "expected track/load error, got: {}",
        error.message
    );
}

#[test]
fn play_with_track_loaded_publishes_updated_playing() {
    let session = EngineSession::new(null_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
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
    session
        .publish_cmd(Event::Play(Play { deck: 0 }))
        .expect("publish");
    let update = recv_updated(&mut evt);
    assert_eq!(update.deck, 0);
    assert!(update.playing);
}

#[test]
fn set_crossfader_publishes_status() {
    let session = EngineSession::new(null_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .publish_cmd(Event::SetCrossfader(SetCrossfader {
            position: 0.75,
            soft_takeover: false,
        }))
        .expect("publish");
    let snapshot = recv_snapshot(&mut evt);
    assert!((snapshot.status.crossfader - 0.75).abs() < f32::EPSILON);
}
