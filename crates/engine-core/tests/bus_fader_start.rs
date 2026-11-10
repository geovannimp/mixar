//! Integration: channel fader start/stop from SetVolume.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::{Play, Seek, SetCuePoint, SetVolume};
use common::{recv_updated, short_tone_fixture};
use engine_core::fader_start::FaderStartConfig;
use engine_core::{AudioConfig, EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};

fn enabled_config() -> EngineConfig {
    EngineConfig {
        backend: "null".to_string(),
        audio: Some(AudioConfig {
            resampler_quality: None,
            sampler_strip_route: None,
            default_tempo_range: None,
            tempo_range_steps: None,
            default_key_lock: None,
            vinyl_brake_pause: None,
            vinyl_brake_stop_secs: None,
            default_crossfader_curve: None,
            default_channel_fader_curve: None,
            default_tempo_curve: None,
            default_fx_level_curve: None,
            crossfader_behavior: None,
            curve_library: None,
            fader_start: Some(FaderStartConfig {
                enabled: true,
                ..Default::default()
            }),
        }),
        ..Default::default()
    }
}

fn disabled_config() -> EngineConfig {
    EngineConfig {
        backend: "null".to_string(),
        audio: Some(AudioConfig {
            resampler_quality: None,
            sampler_strip_route: None,
            default_tempo_range: None,
            tempo_range_steps: None,
            default_key_lock: None,
            vinyl_brake_pause: None,
            vinyl_brake_stop_secs: None,
            default_crossfader_curve: None,
            default_channel_fader_curve: None,
            default_tempo_curve: None,
            default_fx_level_curve: None,
            crossfader_behavior: None,
            curve_library: None,
            fader_start: Some(FaderStartConfig {
                enabled: false,
                ..Default::default()
            }),
        }),
        ..Default::default()
    }
}

fn load_short_tone(session: &EngineSession) {
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::from_path(short_tone_fixture())),
            )?;
            Ok(())
        })
        .expect("load");
}

fn publish(session: &EngineSession, event: Event) {
    session.publish_cmd(event).expect("publish");
}

#[test]
fn fader_leaving_zero_plays_and_returning_pauses() {
    let session = EngineSession::new(enabled_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    load_short_tone(&session);

    // Seed the latch at zero.
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let first = recv_updated(&mut evt);
    assert!(!first.playing);

    // Move up: deck starts playing.
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.5,
            soft_takeover: false,
        }),
    );
    let up = recv_updated(&mut evt);
    assert!(up.playing);

    // Move back down: deck pauses.
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let down = recv_updated(&mut evt);
    assert!(!down.playing);
}

#[test]
fn disabled_config_never_touches_transport() {
    let session = EngineSession::new(disabled_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    load_short_tone(&session);

    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let _ = recv_updated(&mut evt);

    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.5,
            soft_takeover: false,
        }),
    );
    let up = recv_updated(&mut evt);
    assert!(!up.playing);
    assert!((up.volume - 0.5).abs() < f32::EPSILON);
}

#[test]
fn fader_start_without_a_track_is_a_silent_no_op() {
    let session = EngineSession::new(enabled_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");

    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let _ = recv_updated(&mut evt);

    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.5,
            soft_takeover: false,
        }),
    );
    let up = recv_updated(&mut evt);
    assert!(!up.playing);
}

#[test]
fn pause_and_cue_returns_to_the_cue_point() {
    let session = EngineSession::new(enabled_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    load_short_tone(&session);

    // Set a cue point at 5000 ms.
    publish(
        &session,
        Event::Seek(Seek {
            deck: 0,
            position_ms: 5000,
        }),
    );
    let _ = recv_updated(&mut evt);
    publish(&session, Event::SetCuePoint(SetCuePoint { deck: 0 }));
    let cue = recv_updated(&mut evt);
    assert_eq!(cue.cue_point_ms, Some(5000));

    // Play from 9000 ms.
    publish(
        &session,
        Event::Seek(Seek {
            deck: 0,
            position_ms: 9000,
        }),
    );
    let _ = recv_updated(&mut evt);
    publish(&session, Event::Play(Play { deck: 0 }));
    let _ = recv_updated(&mut evt);

    // Seed, cross up (no-op while already playing), then cross down.
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let _ = recv_updated(&mut evt);
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.5,
            soft_takeover: false,
        }),
    );
    let _ = recv_updated(&mut evt);
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let down = recv_updated(&mut evt);
    assert!(!down.playing);
    assert_eq!(down.position_ms, Some(5000));
}

#[test]
fn pause_and_cue_returns_to_the_loaded_cue() {
    let session = EngineSession::new(enabled_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    load_short_tone(&session);

    // Play from 9000 ms without setting an explicit cue point.
    // The freshly loaded track already has a temporary cue at the start (0 ms).
    publish(
        &session,
        Event::Seek(Seek {
            deck: 0,
            position_ms: 9000,
        }),
    );
    let _ = recv_updated(&mut evt);
    publish(&session, Event::Play(Play { deck: 0 }));
    let playing = recv_updated(&mut evt);
    assert!(playing.playing);

    // Seed, cross up (no-op while already playing), then cross down.
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let _ = recv_updated(&mut evt);
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.5,
            soft_takeover: false,
        }),
    );
    let _ = recv_updated(&mut evt);
    publish(
        &session,
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.0,
            soft_takeover: false,
        }),
    );
    let down = recv_updated(&mut evt);
    assert!(!down.playing);
    assert_eq!(down.position_ms, Some(0));
}
