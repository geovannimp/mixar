//! Integration: trigger hot cue + recall saved loop on the bus.

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
            engine.load_track(0, source_with_bpm("hotcue.wav", 120.0))?;
            Ok(())
        })
        .expect("load");
    session
}

#[test]
fn trigger_hot_cue_seeks_and_plays() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::TriggerHotCue,
            encode_cmd_body(&CmdBody::TriggerHotCue { position_secs: 0.5 }).unwrap(),
        )
        .expect("trigger");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        position_secs,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(playing);
    let pos = position_secs.expect("position");
    assert!(pos >= 0.0);
}

#[test]
fn recall_saved_loop_activates_and_plays() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::RecallSavedLoop,
            encode_cmd_body(&CmdBody::RecallSavedLoop {
                in_secs: 0.0,
                out_secs: 1.0,
            })
            .unwrap(),
        )
        .expect("recall");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        playing,
        active_loop,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(playing);
    let region = active_loop.expect("loop");
    assert!((region.in_secs - 0.0).abs() < 1e-6);
    assert!((region.out_secs - 1.0).abs() < 1e-6);
    assert!(region.active);
}
