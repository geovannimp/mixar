//! Integration: pad mode + loop roll cmds on the bus.

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin, PadMode};
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
            engine.load_track(0, source_with_bpm("pads.wav", 120.0))?;
            Ok(())
        })
        .expect("load");
    session
}

#[test]
fn set_pad_mode_publishes_updated() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetPadMode,
            encode_cmd_body(&CmdBody::SetPadMode {
                mode: PadMode::LoopRoll,
            })
            .unwrap(),
        )
        .expect("set pad mode");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { pad_mode, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pad_mode, PadMode::LoopRoll);
}

#[test]
fn begin_and_end_loop_roll_clears_when_no_prior() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::BeginLoopRoll,
            encode_cmd_body(&CmdBody::BeginLoopRoll { beats: 4.0 }).unwrap(),
        )
        .expect("begin roll");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let region = active_loop.expect("roll loop");
    assert!(region.active);
    assert!(region.out_ms > region.in_ms);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::EndLoopRoll,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("end roll");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(active_loop.is_none());
}

#[test]
fn auto_loop_quarter_beat_preserves_length() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetAutoLoop,
            encode_cmd_body(&CmdBody::SetAutoLoop { beats: 0.25 }).unwrap(),
        )
        .expect("auto loop");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let region = active_loop.expect("loop");
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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetAutoLoop,
            encode_cmd_body(&CmdBody::SetAutoLoop { beats: 8.0 }).unwrap(),
        )
        .expect("auto loop");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let prior = active_loop.expect("prior loop");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::BeginLoopRoll,
            encode_cmd_body(&CmdBody::BeginLoopRoll { beats: 1.0 }).unwrap(),
        )
        .expect("begin roll");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::EndLoopRoll,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("end roll");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let restored = active_loop.expect("restored loop");
    assert_eq!(restored.in_ms, prior.in_ms);
    assert_eq!(restored.out_ms, prior.out_ms);
    assert!(restored.active);
}
