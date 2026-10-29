//! Integration: pad mode + loop roll cmds on the bus.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_updated;
use common::source_with_bpm;
use engine_api::BeatJumpPadPress;
use engine_api::BeginLoopRoll;
use engine_api::EndLoopRoll;
use engine_api::Event;
use engine_api::LoopRollPadPress;
use engine_api::LoopRollPadRelease;
use engine_api::PadMode;
use engine_api::PadPress;
use engine_api::SetAutoLoop;
use engine_api::SetPadMode;
use engine_core::EngineConfig;
use engine_core::EngineSession;

fn null_session_loaded() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(0, source_with_bpm("pads.wav", 120.0))?;
            Ok(())
        })
        .expect("load");
    session
}

#[test]
fn set_pad_mode_publishes_updated() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetPadMode(SetPadMode {
            deck: 0,
            mode: PadMode::LoopRoll,
        }))
        .expect("set pad mode");

    let update = recv_updated(&mut evt);
    assert_eq!(update.pad_mode, PadMode::LoopRoll);
}

#[test]
fn begin_and_end_loop_roll_clears_when_no_prior() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::BeginLoopRoll(BeginLoopRoll {
            deck: 0,
            beats: 4.0,
        }))
        .expect("begin roll");

    let update = recv_updated(&mut evt);
    let region = update.active_loop.expect("roll loop");
    assert!(region.active);
    assert!(region.out_ms > region.in_ms);

    session
        .publish_cmd(Event::EndLoopRoll(EndLoopRoll { deck: 0 }))
        .expect("end roll");

    let update = recv_updated(&mut evt);
    assert!(update.active_loop.is_none());
}

#[test]
fn auto_loop_quarter_beat_preserves_length() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: 0,
            beats: 0.25,
        }))
        .expect("auto loop");

    let update = recv_updated(&mut evt);
    let region = update.active_loop.expect("loop");
    let len_ms = region.out_ms - region.in_ms;
    // 120 BPM => 1 beat = 500ms; 0.25 beat = 125ms
    assert!(
        (len_ms - 125).abs() <= 2,
        "expected ~125ms loop, got {len_ms}ms (in={}, out={})",
        region.in_ms,
        region.out_ms
    );
}

#[test]
fn end_loop_roll_restores_prior_active_loop() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: 0,
            beats: 8.0,
        }))
        .expect("auto loop");
    let update = recv_updated(&mut evt);
    let prior = update.active_loop.expect("prior loop");

    session
        .publish_cmd(Event::BeginLoopRoll(BeginLoopRoll {
            deck: 0,
            beats: 1.0,
        }))
        .expect("begin roll");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(Event::EndLoopRoll(EndLoopRoll { deck: 0 }))
        .expect("end roll");

    let update = recv_updated(&mut evt);
    let restored = update.active_loop.expect("restored loop");
    assert_eq!(restored.in_ms, prior.in_ms);
    assert_eq!(restored.out_ms, prior.out_ms);
    assert!(restored.active);
}

#[test]
fn loop_roll_pad_press_release_roundtrip() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::LoopRollPadPress(LoopRollPadPress {
            deck: 0,
            slot: 2,
        }))
        .expect("press");

    let update = recv_updated(&mut evt);
    let region = update.active_loop.expect("roll loop");
    assert!(region.active);
    assert!(region.out_ms > region.in_ms);

    session
        .publish_cmd(Event::LoopRollPadRelease(LoopRollPadRelease {
            deck: 0,
            slot: 2,
        }))
        .expect("release");

    let update = recv_updated(&mut evt);
    assert!(update.active_loop.is_none());
}

#[test]
fn beat_jump_pad_press_moves_playhead() {
    let session = null_session_loaded();
    session
        .with_engine(|engine| {
            engine.seek_deck(0, 0)?;
            Ok(())
        })
        .expect("seek");
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::BeatJumpPadPress(BeatJumpPadPress {
            deck: 0,
            slot: 0,
        }))
        .expect("press");

    let update = recv_updated(&mut evt);
    let pos = update.position_ms.expect("position");
    // +1 beat at 120 BPM = 500ms
    assert!(
        (pos - 500).abs() <= 2,
        "expected ~500ms after +1 beat jump, got {pos}"
    );
}

#[test]
fn pad_press_follows_engine_pad_mode() {
    let session = null_session_loaded();
    session
        .with_engine(|engine| {
            engine.seek_deck(0, 0)?;
            Ok(())
        })
        .expect("seek");
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetPadMode(SetPadMode {
            deck: 0,
            mode: PadMode::BeatJump,
        }))
        .expect("set pad mode");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(Event::PadPress(PadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }))
        .expect("press");

    let update = recv_updated(&mut evt);
    let pos = update.position_ms.expect("position");
    assert!(
        (pos - 500).abs() <= 2,
        "PadPress in beat-jump mode should jump +1 beat, got {pos}"
    );
}
