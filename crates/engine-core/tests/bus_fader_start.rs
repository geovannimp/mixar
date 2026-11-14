//! Integration: channel fader start/stop from SetVolume.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::{FaderStart, FaderStop, Play, Seek, SetCuePoint, SetFaderStartNative, SetVolume};
use common::{recv_updated, short_tone_fixture};
use engine_core::fader_start::FaderStartConfig;
use engine_core::{AudioConfig, EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource};
use std::time::{Duration, Instant};

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

fn started_session(config: EngineConfig) -> (EngineSession, common::TestReceiver) {
    let session = EngineSession::new(config).expect("session");
    let evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    load_short_tone(&session);
    (session, evt)
}

fn assert_native_events_silent(session: &EngineSession, evt: &mut common::TestReceiver) {
    publish(session, Event::FaderStart(FaderStart { deck: 0 }));
    publish(session, Event::FaderStop(FaderStop { deck: 0 }));
    assert_no_deck_updated(evt, Duration::from_millis(250));
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

/// A deck that silently emits no `DeckUpdated` for `timeout` (native/disabled
/// paths publish nothing instead of an error or a spurious update).
fn assert_no_deck_updated(sub: &mut common::TestReceiver, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return;
        }
        match sub.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(None) => {}
            Ok(Some(event)) => {
                if matches!(event, Event::DeckUpdated(_)) {
                    panic!("expected no DeckUpdated, got {event:?}");
                }
            }
            Err(error) => panic!("evt bus disconnected: {error}"),
        }
    }
}

#[test]
fn native_events_run_the_configured_actions() {
    let session = EngineSession::new(enabled_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    load_short_tone(&session);

    publish(&session, Event::FaderStart(FaderStart { deck: 0 }));
    let started = recv_updated(&mut evt);
    assert!(started.playing, "FaderStart on_non_zero must play the deck");

    publish(&session, Event::FaderStop(FaderStop { deck: 0 }));
    let stopped = recv_updated(&mut evt);
    assert!(!stopped.playing, "FaderStop on_zero must pause the deck");
    assert_eq!(stopped.position_ms, Some(0));
}

#[test]
fn native_deck_ignores_the_software_threshold() {
    let session = EngineSession::new(enabled_config()).expect("session");
    let mut evt = session.subscribe_evt_all().expect("sub");
    session.with_engine(|engine| engine.start()).expect("start");
    load_short_tone(&session);

    publish(
        &session,
        Event::SetFaderStartNative(SetFaderStartNative {
            deck: 0,
            native: true,
        }),
    );
    // Silent: it clears the latch and only flips the per-deck flag.
    assert_no_deck_updated(&mut evt, Duration::from_millis(200));

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
            volume: 0.8,
            soft_takeover: false,
        }),
    );
    let up = recv_updated(&mut evt);
    assert!(
        !up.playing,
        "native deck must ignore the software fader threshold"
    );

    publish(&session, Event::FaderStart(FaderStart { deck: 0 }));
    let playing = recv_updated(&mut evt);
    assert!(playing.playing, "controller-native FaderStart must play");
}

#[test]
fn native_events_respect_a_disabled_config() {
    let (session, mut evt) = started_session(disabled_config());
    assert_native_events_silent(&session, &mut evt);
}

#[test]
fn native_events_respect_nothing_actions() {
    let mut config = enabled_config();
    config.audio.as_mut().unwrap().fader_start = Some(FaderStartConfig {
        enabled: true,
        on_non_zero: engine_core::fader_start::OnNonZeroAction::Nothing,
        on_zero: engine_core::fader_start::OnZeroAction::Nothing,
        ..Default::default()
    });
    let (session, mut evt) = started_session(config);
    assert_native_events_silent(&session, &mut evt);
}
