//! Integration: slip mode publishes state and catches up on loop exit.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::JogTouch;
use bus_api::SetSlip;
use common::recv_position;
use common::recv_updated;
use common::short_tone_fixture;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library_core::AudioSource;
use library_core::FileAudioSource;
use library_core::TrackId;
use library_core::TrackMetadata;
use std::time::Duration;

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
                    TrackId::new("slip.wav"),
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
fn set_slip_publishes_enabled_flag_without_shadow() {
    let session = null_session_with_loaded_deck();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetSlip(SetSlip {
            deck: 0,
            enabled: true,
        }))
        .expect("set slip");

    let update = recv_updated(&mut evt);
    assert!(update.slip_enabled);
    assert!(
        update.slip_shadow_position_ms.is_none(),
        "enabling slip must not arm the cursor"
    );
}

#[test]
fn exit_loop_with_slip_catches_up() {
    let session = null_session_with_loaded_deck();

    session
        .with_engine(|engine| {
            engine.set_deck_slip(0, true)?;
            engine.play(0)?;
            engine.set_deck_auto_loop(0, 0.25)?;
            Ok(())
        })
        .expect("setup");

    std::thread::sleep(Duration::from_millis(400));

    let shadow_ms = session
        .with_engine(|engine| Ok(engine.deck_slip_shadow_position_ms(0)))
        .expect("readback");
    let shadow = shadow_ms.expect("loop activation arms the slip cursor");

    session
        .with_engine(|engine| engine.clear_deck_loop(0))
        .expect("exit loop");

    let position_ms = session
        .with_engine(|engine| Ok(engine.deck_playback_ms(0).map(|(p, _)| p)))
        .expect("readback");

    let pos = position_ms.expect("position");
    assert!(
        (pos - shadow).abs() <= 30,
        "loop exit should catch up: pos={pos} shadow={shadow}"
    );
    let cleared = session
        .with_engine(|engine| Ok(engine.deck_slip_shadow_position_ms(0)))
        .expect("readback");
    assert!(cleared.is_none(), "exit loop must clear the slip cursor");
}

#[test]
fn slip_shadow_publishes_only_during_a_manual_action() {
    let session = null_session_with_loaded_deck();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .with_engine(|engine| {
            engine.set_deck_slip(0, true)?;
            engine.play(0)?;
            Ok(())
        })
        .expect("setup");

    // No sleep: `recv_position` blocks until the first Position tick arrives,
    // which is a few ms into the 250 ms fixture — well inside track end, so
    // the deck is still Playing when the jog touch lands and can arm.
    //
    // Plain playback: Position ticks carry no slip cursor.
    let position = recv_position(&mut evt);
    let idle = position.slip_shadow_position_ms;
    assert!(
        idle.is_none(),
        "idle playback must publish no slip cursor: {idle:?}"
    );

    // Jog touch arms the cursor …
    session
        .publish_cmd(Event::JogTouch(JogTouch {
            deck: 0,
            touching: true,
        }))
        .expect("touch");
    let touch = recv_updated(&mut evt);
    assert!(
        touch.slip_shadow_position_ms.is_some(),
        "jog touch must arm the slip cursor"
    );

    // … and release catches up and clears it.
    session
        .publish_cmd(Event::JogTouch(JogTouch {
            deck: 0,
            touching: false,
        }))
        .expect("release");
    let release = recv_updated(&mut evt);
    assert!(
        release.slip_shadow_position_ms.is_none(),
        "jog release must clear the slip cursor"
    );
}

#[test]
fn recall_saved_loop_rejects_invalid_region_without_seeking() {
    let session = null_session_with_loaded_deck();

    session
        .with_engine(|engine| {
            engine.play(0)?;
            engine.seek_deck(0, 40)?;
            engine.pause(0)?;
            Ok(())
        })
        .expect("setup");

    let bad = session.with_engine(|engine| engine.recall_deck_saved_loop(0, 1000, 500));
    assert!(
        bad.is_err(),
        "an inverted region must error before any transport change"
    );

    let position_ms = session
        .with_engine(|engine| Ok(engine.deck_playback_ms(0).map(|(p, _)| p)))
        .expect("readback");
    let pos = position_ms.expect("position");
    assert!(
        (pos - 40).abs() <= 5,
        "a failed recall must leave the playhead parked at the seek: pos={pos}"
    );
}
