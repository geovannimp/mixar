//! Integration: sampler trigger/end on the bus.

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

fn sample_source() -> AudioSource {
    AudioSource::File(FileAudioSource::new(
        TrackId::new("sample.wav"),
        short_tone_fixture(),
        TrackMetadata::default(),
    ))
}

fn null_session_with_sample() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.set_deck_pad_mode(0, PadMode::Sampler)?;
            engine.assign_sampler_slot(0, 0, sample_source(), "tone".into(), None)?;
            Ok(())
        })
        .expect("assign");
    session
}

#[test]
fn trigger_and_end_sampler_roundtrip() {
    let session = null_session_with_sample();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadPress,
            encode_cmd_body(&CmdBody::SamplerPadPress {
                slot: 0,
                shift: false,
            })
            .unwrap(),
        )
        .expect("trigger");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { pad_mode, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pad_mode, PadMode::Sampler);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadRelease,
            encode_cmd_body(&CmdBody::SamplerPadRelease { slot: 0 }).unwrap(),
        )
        .expect("end");

    let event = recv_evt_kind(&evt, Kind::Updated);
    assert_eq!(*event.kind(), Kind::Updated);
}

#[test]
fn sampler_pad_press_release_without_sampler_mode() {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.assign_sampler_slot(0, 0, sample_source(), "tone".into(), None)?;
            Ok(())
        })
        .expect("assign");

    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadPress,
            encode_cmd_body(&CmdBody::SamplerPadPress {
                slot: 0,
                shift: false,
            })
            .unwrap(),
        )
        .expect("press");
    let event = recv_evt_kind(&evt, Kind::Updated);
    assert_eq!(*event.kind(), Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadRelease,
            encode_cmd_body(&CmdBody::SamplerPadRelease { slot: 0 }).unwrap(),
        )
        .expect("release");
    let event = recv_evt_kind(&evt, Kind::Updated);
    assert_eq!(*event.kind(), Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadPress,
            encode_cmd_body(&CmdBody::SamplerPadPress {
                slot: 0,
                shift: true,
            })
            .unwrap(),
        )
        .expect("clear");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadPress,
            encode_cmd_body(&CmdBody::SamplerPadPress {
                slot: 0,
                shift: false,
            })
            .unwrap(),
        )
        .expect("empty press");
    let event = recv_evt_kind(&evt, Kind::Error);
    let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("decode") else {
        panic!("expected Error");
    };
    assert!(
        !message.is_empty(),
        "cleared slot should fail trigger: {message}"
    );
}
