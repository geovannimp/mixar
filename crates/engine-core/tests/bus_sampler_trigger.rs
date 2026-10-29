//! Integration: sampler trigger/end on the bus.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_error;
use common::recv_updated;
use common::short_tone_fixture;
use engine_api::AssignSampler;
use engine_api::Event;
use engine_api::PadMode;
use engine_api::SamplerPadPress;
use engine_api::SamplerPadRelease;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library::LibraryConfig;
use library::LibraryManager;
use library_core::AudioSource;
use library_core::FileAudioSource;
use library_core::TrackId;
use library_core::TrackMetadata;
use std::sync::Arc;
use std::sync::Mutex;

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
fn assign_sampler_path_via_bus_assigns_slot() {
    let library = Arc::new(Mutex::new(
        LibraryManager::open_in_memory(LibraryConfig::default()).expect("library"),
    ));
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new_with_library(config, library).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");

    let path = short_tone_fixture().to_string_lossy().into_owned();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::AssignSampler(AssignSampler {
            deck: 0,
            slot: 1,
            path,
        }))
        .expect("assign");

    let _ = recv_updated(&mut evt);
    let assigned = session
        .with_engine(|engine| Ok(engine.sampler_slot_assigned(0, 1)))
        .expect("query");
    assert!(
        assigned,
        "AssignSampler must load pad slot 1 off the engine lock"
    );
}

#[test]
fn trigger_and_end_sampler_roundtrip() {
    let session = null_session_with_sample();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SamplerPadPress(SamplerPadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }))
        .expect("trigger");

    let update = recv_updated(&mut evt);
    // `pad_mode` mirrors engine state (the fixture configured Sampler). The
    // sampler voice has no wire representation, so this is the strongest check
    // available: the press produced a deck snapshot still in sampler mode.
    assert_eq!(update.pad_mode, PadMode::Sampler);

    session
        .publish_cmd(Event::SamplerPadRelease(SamplerPadRelease {
            deck: 0,
            slot: 0,
        }))
        .expect("end");

    let update = recv_updated(&mut evt);
    assert_eq!(update.pad_mode, PadMode::Sampler);
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

    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SamplerPadPress(SamplerPadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }))
        .expect("press");
    let update = recv_updated(&mut evt);
    // `Event::SamplerPadPress` is dispatched straight to
    // `Engine::sampler_pad_press` in `control.rs`; it does not go through the
    // `pad_mode` router, so this asserts only that the sampler command leaves the
    // deck's pad mode untouched (it starts, and stays, on the HotCue default).
    assert_eq!(update.pad_mode, PadMode::HotCue);

    session
        .publish_cmd(Event::SamplerPadRelease(SamplerPadRelease {
            deck: 0,
            slot: 0,
        }))
        .expect("release");
    let update = recv_updated(&mut evt);
    assert_eq!(update.pad_mode, PadMode::HotCue);

    session
        .publish_cmd(Event::SamplerPadPress(SamplerPadPress {
            deck: 0,
            slot: 0,
            shift: true,
        }))
        .expect("clear");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(Event::SamplerPadPress(SamplerPadPress {
            deck: 0,
            slot: 0,
            shift: false,
        }))
        .expect("empty press");
    let error = recv_error(&mut evt);
    // Anchor to the slot index as well as the semantic word: a bare
    // `contains("0")` would match any message carrying a 0 (e.g. a frame count),
    // while the engine's message is `"Sampler slot {slot} is empty"`.
    assert!(
        error.message.contains("slot 0") && error.message.contains("empty"),
        "cleared slot should fail trigger with an empty-slot error for slot 0, got: {}",
        error.message
    );
}
