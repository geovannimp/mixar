//! Integration: the stem-EQ deck toggle, its cmds, and the EQ routing it drives.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use anyhow::Result;
use bus_api::EqBand;
use bus_api::Event;
use bus_api::SetEqBand;
use bus_api::SetStemEqBand;
use bus_api::SetStemEqEnabled;
use bus_api::ToggleStemEq;
use common::recv_updated;
use common::recv_updated_where;
use engine_core::AnalysisDurationMode;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use event_bus::Subscriber;
use library::LibraryConfig;
use library::LibraryManager;
use library_core::AudioSource;
use library_core::FileAudioSource;
use std::sync::Arc;
use std::sync::Mutex;

fn null_session() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
}

/// Session holding one native `.stem.mp4` loaded on deck 0, so `stems_ready` is true.
fn stem_session(frame_amp: f32) -> Result<(EngineSession, Subscriber<Event>)> {
    use codec::encode_stem_mp4;
    use codec::StemAtom;
    use codec::StemMuxFormat;

    let dir = tempfile::tempdir()?;
    let stem_path = dir.path().join("library.stem.mp4");
    let rate: u32 = 48_000;
    let frames = (rate / 10) as usize;
    let tone = |amp: f32| -> Vec<f32> {
        (0..frames * 2)
            .map(|i| amp * ((i / 2) as f32 * 0.01).sin())
            .collect()
    };
    let mix = tone(0.1);
    let stems = [
        tone(frame_amp),
        tone(frame_amp),
        tone(frame_amp),
        tone(frame_amp),
    ];
    encode_stem_mp4(
        &stem_path,
        StemMuxFormat::Opus,
        rate,
        &mix,
        [&stems[0], &stems[1], &stems[2], &stems[3]],
        &StemAtom::default_ni(),
    )?;

    let config = EngineConfig {
        backend: "null".to_string(),
        analysis_duration: AnalysisDurationMode::Complete,
        ..Default::default()
    };
    let lib = Arc::new(Mutex::new(LibraryManager::open_in_memory(
        LibraryConfig::default(),
    )?));
    let imported = lib.lock().unwrap().import_file_path(&stem_path)?;
    let session = EngineSession::new_with_library(config, Arc::clone(&lib))?;
    let mut evt = session.subscribe_evt_all()?;
    session.with_engine(|engine| engine.start())?;
    session.with_engine(|engine| engine.load_track_from_library(0, imported.id()))?;
    // `stems_ready` gates the routing, so a silently buffer-less load must fail
    // here. Draining past the load update also keeps pre-load snapshots out of
    // the later receives.
    let loaded = recv_updated_where(&mut evt, |update| update.stems_ready);
    assert_eq!(loaded.deck, 0);
    Ok((session, evt))
}

/// Engine with live (ring-fed) stems attached on deck 0, coverage published.
fn live_session() -> Result<(
    EngineSession,
    Arc<audio_core::LiveStemRing>,
    Subscriber<Event>,
)> {
    let session = null_session();
    let evt = session.subscribe_evt_all()?;
    session.with_engine(|engine| {
        engine.load_track(
            0,
            AudioSource::File(FileAudioSource::from_path(common::short_tone_fixture())),
        )
    })?;
    let ring = Arc::new(audio_core::LiveStemRing::new(48_000, 1 << 19));
    let stems = vec![vec![0.0f32; 4096 * 2]; 4];
    ring.publish(0, 4096, &[&stems[0], &stems[1], &stems[2], &stems[3]]);
    session.with_engine(|engine| engine.attach_live_deck_stems(0, Arc::clone(&ring)))?;
    Ok((session, ring, evt))
}

#[test]
fn set_stem_eq_band_publishes_snapshot_and_leaves_channel_eq() {
    let session = null_session();
    let mut evt = session.subscribe_evt_all().expect("sub");
    session
        .publish_cmd(Event::SetStemEqBand(SetStemEqBand {
            deck: 0,
            band: EqBand::High,
            gain: 0.0,
            soft_takeover: false,
        }))
        .expect("publish");
    let update = recv_updated(&mut evt);
    assert!(
        (update.stem_eq[0] - 0.5).abs() < 0.01,
        "low untouched: {:?}",
        update.stem_eq
    );
    assert!(
        (update.stem_eq[1] - 0.5).abs() < 0.01,
        "mid untouched: {:?}",
        update.stem_eq
    );
    assert!(
        (update.stem_eq[2] - 0.0).abs() < 0.01,
        "high set: {:?}",
        update.stem_eq
    );
    assert!(
        (update.eq.high - 0.5).abs() < 0.01,
        "channel EQ must stay neutral: {:?}",
        update.eq
    );
}

#[test]
fn stem_eq_toggle_without_stems_leaves_eq_on_the_channel() -> Result<()> {
    let session = null_session();
    let mut evt = session.subscribe_evt_all()?;
    session.publish_cmd(Event::SetStemEqEnabled(SetStemEqEnabled {
        deck: 0,
        enabled: true,
    }))?;
    let update = recv_updated_where(&mut evt, |update| update.stem_eq_enabled);
    assert!(update.stem_eq_enabled, "intent is recorded");
    assert!(!update.stems_ready, "no stems on an empty deck");

    // The knobs must stay audible: EQ lands on the channel, never on hidden stems.
    session.publish_cmd(Event::SetEqBand(SetEqBand {
        deck: 0,
        band: EqBand::Low,
        gain: 1.0,
        soft_takeover: false,
    }))?;
    let update = recv_updated_where(&mut evt, |update| update.stem_eq_enabled);
    assert!(
        (update.eq.low - 1.0).abs() < 0.01,
        "channel low follows the knob: {:?}",
        update.eq
    );
    assert_eq!(update.stem_eq, vec![0.5; 3]);
    Ok(())
}

#[test]
fn stem_eq_toggle_routes_eq_band_to_stem_eq() -> Result<()> {
    let (session, mut evt) = stem_session(0.2)?;
    session.publish_cmd(Event::SetStemEqEnabled(SetStemEqEnabled {
        deck: 0,
        enabled: true,
    }))?;
    let update = recv_updated_where(&mut evt, |update| update.stem_eq_enabled);
    assert!(update.stem_eq_enabled);
    assert!(update.stems_ready);

    session.publish_cmd(Event::SetEqBand(SetEqBand {
        deck: 0,
        band: EqBand::Low,
        gain: 0.2,
        soft_takeover: false,
    }))?;
    let update = recv_updated_where(&mut evt, |update| update.stem_eq_enabled);
    assert!(
        (update.stem_eq[0] - 0.2).abs() < 0.01,
        "stem low: {:?}",
        update.stem_eq
    );
    assert!(
        (update.eq.low - 0.5).abs() < 0.01,
        "channel low must stay neutral: {:?}",
        update.eq
    );
    Ok(())
}

/// Live stems are only audible while the producer's coverage covers the
/// playhead: that is when the knobs must drive the stem EQ.
#[test]
fn stem_eq_routes_to_the_stem_eq_while_live_coverage_covers_the_playhead() -> Result<()> {
    let (session, _ring, mut evt) = live_session()?;
    session.publish_cmd(Event::SetStemEqEnabled(SetStemEqEnabled {
        deck: 0,
        enabled: true,
    }))?;
    let update = recv_updated_where(&mut evt, |update| update.stem_eq_enabled);
    assert!(
        update.stems_audible,
        "published coverage covers the playhead"
    );

    session.publish_cmd(Event::SetEqBand(SetEqBand {
        deck: 0,
        band: EqBand::Low,
        gain: 0.2,
        soft_takeover: false,
    }))?;
    let update = recv_updated_where(&mut evt, |update| update.stem_eq_enabled);
    assert!(
        (update.stem_eq[0] - 0.2).abs() < 0.01,
        "stem low: {:?}",
        update.stem_eq
    );
    assert!(
        (update.eq.low - 0.5).abs() < 0.01,
        "channel low must stay neutral: {:?}",
        update.eq
    );
    Ok(())
}

/// A live producer that falls behind leaves the deck on its dry mix. Stem gains
/// are inaudible there, so the knobs must move the channel EQ instead of a
/// silent path — this is the "live stem EQ does nothing" failure.
#[test]
fn stem_eq_falls_back_to_the_channel_while_live_stems_are_starved() -> Result<()> {
    let (session, ring, mut evt) = live_session()?;
    session.publish_cmd(Event::SetStemEqEnabled(SetStemEqEnabled {
        deck: 0,
        enabled: true,
    }))?;
    recv_updated_where(&mut evt, |update| update.stem_eq_enabled);

    // Producer re-epochs / drops coverage: the deck now plays the dry mix.
    ring.reset(0);
    session.publish_cmd(Event::SetEqBand(SetEqBand {
        deck: 0,
        band: EqBand::Mid,
        gain: 0.1,
        soft_takeover: false,
    }))?;
    let update = recv_updated_where(&mut evt, |update| update.stem_eq_enabled);
    assert!(
        (update.eq.mid - 0.1).abs() < 0.01,
        "channel mid follows the knob while starved: {:?}",
        update.eq
    );
    assert!(
        (update.stem_eq[1] - 0.5).abs() < 0.01,
        "stem EQ must stay neutral: {:?}",
        update.stem_eq
    );
    Ok(())
}

#[test]
fn toggle_stem_eq_flips_the_per_deck_flag() -> Result<()> {
    let session = null_session();
    let mut evt = session.subscribe_evt_all()?;
    session.publish_cmd(Event::ToggleStemEq(ToggleStemEq { deck: 1 }))?;
    let update = recv_updated_where(&mut evt, |update| {
        update.deck == 1 && update.stem_eq_enabled
    });
    assert_eq!(update.deck, 1);

    session.publish_cmd(Event::ToggleStemEq(ToggleStemEq { deck: 1 }))?;
    let update = recv_updated_where(&mut evt, |update| update.deck == 1);
    assert!(!update.stem_eq_enabled);
    Ok(())
}

#[test]
fn non_finite_stem_eq_gain_leaves_bands_neutral() -> Result<()> {
    let session = null_session();
    let mut evt = session.subscribe_evt_all()?;
    session.publish_cmd(Event::SetStemEqBand(SetStemEqBand {
        deck: 0,
        band: EqBand::Mid,
        gain: f32::NAN,
        soft_takeover: false,
    }))?;
    let update = recv_updated(&mut evt);
    assert_eq!(update.stem_eq, vec![0.5; 3]);
    Ok(())
}
