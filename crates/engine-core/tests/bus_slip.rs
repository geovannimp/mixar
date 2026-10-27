//! Integration: slip mode publishes state and catches up on loop exit.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::{recv_evt_kind, short_tone_fixture};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
use omnibus::Filter;
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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetSlip,
            encode_cmd_body(&CmdBody::SetSlip { enabled: true }).unwrap(),
        )
        .expect("set slip");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        slip_enabled,
        slip_shadow_position_ms,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(slip_enabled);
    assert!(
        slip_shadow_position_ms.is_none(),
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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .with_engine(|engine| {
            engine.set_deck_slip(0, true)?;
            engine.play(0)?;
            Ok(())
        })
        .expect("setup");

    // No sleep: `recv_evt_kind` blocks until the first Position tick arrives,
    // which is a few ms into the 250 ms fixture — well inside track end, so
    // the deck is still Playing when the jog touch lands and can arm.
    //
    // Plain playback: Position ticks carry no slip cursor.
    let position = recv_evt_kind(&evt, Kind::Position);
    let EvtBody::Position {
        slip_shadow_position_ms: idle,
        ..
    } = decode_evt_body(position.payload()).expect("decode")
    else {
        panic!("expected Position");
    };
    assert!(
        idle.is_none(),
        "idle playback must publish no slip cursor: {idle:?}"
    );

    // Jog touch arms the cursor …
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::JogTouch,
            encode_cmd_body(&CmdBody::JogTouch { touching: true }).unwrap(),
        )
        .expect("touch");
    let touch = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        slip_shadow_position_ms: armed,
        ..
    } = decode_evt_body(touch.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(armed.is_some(), "jog touch must arm the slip cursor");

    // … and release catches up and clears it.
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::JogTouch,
            encode_cmd_body(&CmdBody::JogTouch { touching: false }).unwrap(),
        )
        .expect("release");
    let release = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        slip_shadow_position_ms: cleared,
        ..
    } = decode_evt_body(release.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(cleared.is_none(), "jog release must clear the slip cursor");
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
