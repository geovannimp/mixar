#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::Pause;
use bus_api::Play;
use common::recv_updated;
use common::short_tone_fixture;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library_core::AudioSource;
use library_core::FileAudioSource;
use std::thread;
use std::time::Duration;

#[test]
fn pause_preserves_playback_position() {
    let session = EngineSession::new(EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    })
    .expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
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

    session
        .publish_cmd(Event::Play(Play { deck: 0 }))
        .expect("play");
    let _ = recv_updated(&mut evt);

    // Let the null backend advance a bit.
    thread::sleep(Duration::from_millis(80));
    session
        .publish_cmd(Event::Pause(Pause { deck: 0 }))
        .expect("pause");
    let update = recv_updated(&mut evt);
    assert!(!update.playing);
    let pos = update.position_ms.expect("position");
    assert!(
        pos >= 40,
        "pause should not reset to start, got position_ms={pos}"
    );
}
