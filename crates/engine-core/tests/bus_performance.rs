//! Integration: performance cmds (cue / loop / beat jump / unload) on the bus.

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
        let event = sub
            .recv_timeout(remaining.min(Duration::from_millis(50)))
            .expect("recv")
            .expect("event");
        if *event.kind() == kind {
            return (*event).clone();
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

fn source_with_bpm(id: &str, bpm: f64) -> AudioSource {
    AudioSource::File(FileAudioSource::new(
        TrackId::new(id),
        short_tone_fixture(),
        TrackMetadata {
            bpm: Some(bpm),
            ..Default::default()
        },
    ))
}

fn null_session_loaded() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(0, source_with_bpm("perf.wav", 120.0))?;
            Ok(())
        })
        .expect("load");
    session
}

#[test]
fn set_auto_loop_publishes_active_loop() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetAutoLoop,
            encode_cmd_body(&CmdBody::SetAutoLoop { beats: 4 }).unwrap(),
        )
        .expect("auto loop");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let region = active_loop.expect("loop region");
    assert!(region.active);
    assert!(region.out_ms > region.in_ms);
}

#[test]
fn set_quantize_and_cue_point_roundtrip() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize");
    let q = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { quantize, .. } = decode_evt_body(q.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(!quantize);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetCuePoint,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("cue");
    let cue = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { cue_point_ms, .. } = decode_evt_body(cue.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(cue_point_ms.is_some());

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::LoopIn,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("loop in");
    let loop_in = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(loop_in.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(active_loop.is_some());

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::BeatJump,
            encode_cmd_body(&CmdBody::BeatJump { beats: 1 }).unwrap(),
        )
        .expect("jump");
    let _ = recv_evt_kind(&evt, Kind::Updated);
}

#[test]
fn unload_clears_duration() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::Unload,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("unload");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        duration_ms,
        cue_point_ms,
        active_loop,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(duration_ms.is_none());
    assert!(cue_point_ms.is_none() || cue_point_ms == Some(0));
    assert!(active_loop.is_none());
}
