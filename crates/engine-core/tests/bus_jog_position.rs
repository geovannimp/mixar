//! Vinyl jog while paused must publish Position so UI playhead/time track live.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::JogTouch;
use bus_api::JogTurn;
use common::recv_position;
use common::recv_updated;
use common::short_tone_fixture;
use common::SHORT_TONE_LEN_MS;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library_core::AudioSource;
use library_core::FileAudioSource;

#[test]
fn paused_vinyl_jog_touch_publishes_position_before_release() {
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
            engine.seek_deck(0, 80)?;
            Ok(())
        })
        .expect("load");

    session
        .publish_cmd(Event::JogTouch(JogTouch {
            deck: 0,
            touching: true,
        }))
        .expect("touch");
    let touch = recv_updated(&mut evt);
    assert!(!touch.playing);
    assert!(touch.jog_touching);

    // Scratch ticks are Silent (no DeckUpdated). UI must still get Position while
    // touched — otherwise waveform/time only jump on jog-touch release.
    session
        .publish_cmd(Event::JogTurn(JogTurn { deck: 0, delta: 90 }))
        .expect("turn");

    let position = recv_position(&mut evt);
    let position_ms = position.position_ms;
    // Contract under test: a Position evt is emitted at all while the deck is
    // jog-touched (`recv_position` panics on timeout) and carries a playhead
    // inside the 0.25 s fixture. The wheel's rate math is asserted separately by
    // engine-dsp's `paused_vinyl_jog_advances_position_ms`, which owns it.
    assert!(
        (0..=SHORT_TONE_LEN_MS).contains(&position_ms),
        "Position must be within the {SHORT_TONE_LEN_MS} ms fixture, got {position_ms}"
    );
}
