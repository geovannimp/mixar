//! Integration: sampler trigger/end on the bus.

mod common;

use common::{recv_evt_kind, short_tone_fixture};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin, PadMode};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
use omnibus::Filter;

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
    // `pad_mode` mirrors engine state (the fixture configured Sampler). The
    // sampler voice has no wire representation, so this is the strongest check
    // available: the press produced a deck snapshot still in sampler mode.
    assert_eq!(pad_mode, PadMode::Sampler);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadRelease,
            encode_cmd_body(&CmdBody::SamplerPadRelease { slot: 0 }).unwrap(),
        )
        .expect("end");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { pad_mode, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated on release");
    };
    assert_eq!(pad_mode, PadMode::Sampler);
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
    // No `set_deck_pad_mode` in this fixture, so the deck stays on its default
    // HotCue pads: the press must NOT have been routed to the sampler.
    let EvtBody::DeckUpdated { pad_mode, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pad_mode, PadMode::HotCue);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SamplerPadRelease,
            encode_cmd_body(&CmdBody::SamplerPadRelease { slot: 0 }).unwrap(),
        )
        .expect("release");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { pad_mode, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated on release");
    };
    assert_eq!(pad_mode, PadMode::HotCue);

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
    // Anchor to the slot index as well as the semantic word: a bare
    // `contains("0")` would match any message carrying a 0 (e.g. a frame count),
    // while the engine's message is `"Sampler slot {slot} is empty"`.
    assert!(
        message.contains("slot 0") && message.contains("empty"),
        "cleared slot should fail trigger with an empty-slot error for slot 0, got: {message}"
    );
}
