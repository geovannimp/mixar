use super::*;
use audio_core::{BusConfig, BusId, ChannelMapping, DeviceId, LoadableAudio, LoadedAudio};
use engine_dsp::DeckEqGains;
use library::PreparedTrackPlayback;
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
use std::path::PathBuf;
use std::sync::Arc;

fn test_source(loudness_lufs: Option<f64>) -> AudioSource {
    AudioSource::File(FileAudioSource::new(
        TrackId::new("test.wav"),
        PathBuf::from("/no/such/file.wav"),
        TrackMetadata {
            loudness_lufs,
            ..Default::default()
        },
    ))
}

fn prepared_test_source(loudness_lufs: Option<f64>) -> PreparedTrackPlayback {
    let source = test_source(loudness_lufs);
    let track_id = source.id().clone();
    PreparedTrackPlayback {
        track_id: track_id.clone(),
        source,
        audio: Arc::new(LoadedAudio {
            samples: vec![],
            sample_rate: 48_000,
            channels: 2,
            source_id: track_id.to_string(),
        }),
        stems: None,
        loudness_lufs,
    }
}

fn null_engine(num_decks: usize) -> Engine {
    Engine::new(EngineConfig {
        backend: "null".into(),
        num_decks,
        ..Default::default()
    })
    .unwrap()
}

fn started_null_engine() -> Engine {
    let mut engine = null_engine(2);
    engine.start().unwrap();
    engine
}

#[test]
fn native_fader_ownership_changes_only_rearm_the_affected_deck() {
    let mut engine = null_engine(2);
    engine.set_fader_start_config(FaderStartConfig {
        enabled: true,
        ..Default::default()
    });
    for deck in [0, 1] {
        assert_eq!(engine.fader_start_action(deck, 0.0), None);
    }

    engine.set_fader_start_native(0, true);
    engine.set_fader_start_native(0, false);

    assert_eq!(
        engine.fader_start_action(1, 0.5),
        Some(FaderStartAction::Play)
    );
    assert_eq!(engine.fader_start_action(0, 0.5), None);
    assert_eq!(
        engine.fader_start_action(0, 0.0),
        Some(FaderStartAction::PauseAndCue)
    );
}

#[test]
fn unchanged_native_fader_ownership_preserves_software_latches() {
    let mut engine = null_engine(2);
    engine.set_fader_start_config(FaderStartConfig {
        enabled: true,
        ..Default::default()
    });
    for deck in [0, 1] {
        assert_eq!(engine.fader_start_action(deck, 0.0), None);
        engine.set_fader_start_native(deck, false);
        assert_eq!(
            engine.fader_start_action(deck, 0.5),
            Some(FaderStartAction::Play)
        );
    }
}

#[test]
fn native_fader_ownership_ignores_unknown_decks_without_resetting_latches() {
    let mut engine = started_null_engine();
    engine.set_fader_start_config(FaderStartConfig {
        enabled: true,
        ..Default::default()
    });
    let deck_count = engine.fader_start_native.len();
    assert_eq!(engine.fader_start_action(0, 0.0), None);

    for deck in [2, u16::MAX] {
        engine.set_fader_start_native(deck, true);
        engine.set_fader_start_native(deck, false);
    }

    assert_eq!(engine.fader_start_native.len(), deck_count);
    assert!(engine.fader_start_native.iter().all(|native| !native));
    assert_eq!(
        engine.fader_start_action(0, 0.5),
        Some(FaderStartAction::Play)
    );
}

#[test]
fn four_deck_engine_accepts_native_ownership_for_decks_three_and_four() {
    let mut engine = null_engine(4);
    engine.set_fader_start_config(FaderStartConfig {
        enabled: true,
        ..Default::default()
    });

    for deck in [2, 3] {
        engine.set_fader_start_native(deck, true);
        assert!(engine.fader_start_native[usize::from(deck)]);
        assert_eq!(engine.fader_start_action(deck, 0.0), None);
        assert_eq!(engine.fader_start_action(deck, 0.5), None);
    }

    engine.set_fader_start_native(3, false);
    assert_eq!(engine.fader_start_action(3, 0.0), None);
    assert_eq!(
        engine.fader_start_action(3, 0.5),
        Some(FaderStartAction::Play)
    );
}

#[test]
fn fader_actions_reject_ids_outside_the_configured_deck_count() {
    for num_decks in [2, 4] {
        let mut engine = null_engine(num_decks);
        engine.set_fader_start_config(FaderStartConfig {
            enabled: true,
            ..Default::default()
        });
        for deck in 0..num_decks as u16 {
            assert_eq!(engine.fader_start_action(deck, 0.0), None);
        }
        for deck in [num_decks as u16, u16::MAX] {
            assert_eq!(engine.fader_start_action(deck, 0.0), None);
            assert_eq!(engine.fader_start_action(deck, 0.9), None);
            assert_eq!(engine.fader_start_action(deck, 0.0), None);
        }
        for deck in 0..num_decks as u16 {
            assert_eq!(
                engine.fader_start_action(deck, 0.9),
                Some(FaderStartAction::Play)
            );
        }
    }
}

#[test]
fn effective_fader_config_changes_rearm_every_deck() {
    let initial = FaderStartConfig {
        enabled: true,
        ..Default::default()
    };
    for changed in [
        FaderStartConfig {
            threshold_pct: 20,
            ..initial
        },
        FaderStartConfig {
            enabled: false,
            ..initial
        },
        FaderStartConfig {
            on_non_zero: crate::fader_start::OnNonZeroAction::TogglePlay,
            ..initial
        },
        FaderStartConfig {
            on_zero: crate::fader_start::OnZeroAction::Pause,
            ..initial
        },
    ] {
        let mut engine = null_engine(4);
        engine.set_fader_start_config(initial);
        for deck in 0..4 {
            assert_eq!(engine.fader_start_action(deck, 0.0), None);
        }
        engine.set_fader_start_config(changed);
        for deck in 0..4 {
            assert_eq!(engine.fader_start_action(deck, 0.9), None);
        }
        // Re-enabling also seeds instead of acting on the previous open latch.
        engine.set_fader_start_config(initial);
        for deck in 0..4 {
            assert_eq!(engine.fader_start_action(deck, 0.0), None);
            assert_eq!(
                engine.fader_start_action(deck, 0.9),
                Some(FaderStartAction::Play)
            );
        }
    }
}

#[test]
fn equivalent_normalized_fader_config_preserves_latches() {
    let mut engine = null_engine(2);
    let config = FaderStartConfig {
        enabled: true,
        threshold_pct: 20,
        ..Default::default()
    };
    engine.set_fader_start_config(config);
    assert_eq!(engine.fader_start_action(0, 0.0), None);
    engine.set_fader_start_config(FaderStartConfig {
        threshold_pct: u8::MAX,
        ..config
    });
    assert_eq!(
        engine.fader_start_action(0, 0.9),
        Some(FaderStartAction::Play)
    );
}

#[test]
fn track_lifecycle_and_pause_and_cue_preserve_physical_fader_latches() {
    let mut engine = started_null_engine();
    engine.set_fader_start_config(FaderStartConfig {
        enabled: true,
        ..Default::default()
    });
    assert_eq!(engine.fader_start_action(0, 0.0), None);
    engine
        .load_prepared_track(0, prepared_test_source(None))
        .unwrap();
    assert_eq!(
        engine.fader_start_action(0, 0.9),
        Some(FaderStartAction::Play)
    );
    assert_eq!(
        engine.fader_start_action(0, 0.0),
        Some(FaderStartAction::PauseAndCue)
    );
    // This is the transport sequence used by the PauseAndCue dispatcher.
    engine.pause(0).unwrap();
    engine.seek_deck(0, 0).unwrap();
    assert_eq!(
        engine.fader_start_action(0, 0.9),
        Some(FaderStartAction::Play)
    );
    engine.unload_deck(0).unwrap();
    engine
        .load_prepared_track(0, prepared_test_source(None))
        .unwrap();
    assert_eq!(
        engine.fader_start_action(0, 0.0),
        Some(FaderStartAction::PauseAndCue)
    );
    engine.stop().unwrap();
}

#[test]
fn four_deck_engine_exposes_deck_four_and_rejects_deck_five() {
    let mut engine = null_engine(4);
    engine.start().unwrap();

    engine.set_deck_volume(3, 0.25).unwrap();
    assert_eq!(with_channel(&engine, 3, |channel| channel.volume()), 0.25);
    assert!(engine.set_deck_volume(4, 0.5).is_err());
    engine.stop().unwrap();
}

/// Inspect the mixer channel that backs `deck_id`, bypassing the public API so delegation
/// tests observe mixer-owned state directly instead of trusting the same setter/getter pair.
fn with_channel<T>(
    engine: &Engine,
    deck_id: usize,
    f: impl FnOnce(&engine_dsp::MixerChannel) -> T,
) -> T {
    let dsp_engine = engine.dsp_engine.as_ref().expect("engine running");
    let dsp = dsp_engine.lock().unwrap();
    let channel = dsp
        .mixer()
        .channel(deck_id)
        .expect("mixer channel must exist for a valid deck id");
    f(channel)
}

/// Inspect the DSP deck that backs `deck_id`.
fn with_deck<T>(engine: &Engine, deck_id: usize, f: impl FnOnce(&engine_dsp::Deck) -> T) -> T {
    let dsp_engine = engine.dsp_engine.as_ref().expect("engine running");
    let dsp = dsp_engine.lock().unwrap();
    let deck = dsp
        .deck(deck_id)
        .expect("deck must exist for a valid deck id");
    f(deck)
}

#[test]
fn load_track_sets_channel_loudness_from_metadata() {
    let mut engine = started_null_engine();
    engine.set_normalizer_target(Some(-18.0)).unwrap();
    engine
        .load_prepared_track(0, prepared_test_source(Some(-24.0)))
        .unwrap();

    assert_eq!(with_channel(&engine, 0, |c| c.loudness_lufs()), Some(-24.0));
    assert_eq!(with_channel(&engine, 0, |c| c.auto_gain_db()), 6.0);
    assert_eq!(engine.deck_auto_gain_db(0), Some(6.0));

    engine.stop().unwrap();
}

#[test]
fn load_track_invalid_deck_id_does_not_partially_mutate_state() {
    let mut engine = started_null_engine();
    engine.set_normalizer_target(Some(-18.0)).unwrap();
    engine
        .load_prepared_track(0, prepared_test_source(Some(-21.0)))
        .unwrap();

    let err = engine
        .load_prepared_track(2, prepared_test_source(Some(-27.0)))
        .unwrap_err();
    assert!(err.to_string().contains("Invalid deck ID"));

    assert_eq!(with_channel(&engine, 0, |c| c.loudness_lufs()), Some(-21.0));
    assert_eq!(with_channel(&engine, 0, |c| c.auto_gain_db()), 3.0);

    engine.stop().unwrap();
}

#[test]
fn deck_volume_delegates_to_mixer_channel() {
    let mut engine = started_null_engine();

    engine.set_deck_volume(0, 0.25).unwrap();

    assert_eq!(with_channel(&engine, 0, |c| c.volume()), 0.25);

    engine.stop().unwrap();
}

#[test]
fn deck_headphone_cue_delegates_to_mixer_channel() {
    let mut engine = started_null_engine();

    engine.set_deck_headphone_cue(0, true).unwrap();

    assert!(with_channel(&engine, 0, |c| c.headphone_cue()));

    engine.stop().unwrap();
}

#[test]
fn deck_eq_delegates_to_mixer_channel() {
    let mut engine = started_null_engine();

    engine.set_deck_eq_bands(0, 4.0, -2.0, 1.0).unwrap();

    assert_eq!(
        with_channel(&engine, 0, |c| c.eq_gains()),
        DeckEqGains::clamped(4.0, -2.0, 1.0)
    );

    engine.stop().unwrap();
}

#[test]
fn deck_filter_delegates_to_mixer_channel() {
    let mut engine = started_null_engine();

    engine.set_deck_filter_db(0, 5.0).unwrap();

    assert_eq!(with_channel(&engine, 0, |c| c.filter_db()), 5.0);

    engine.stop().unwrap();
}

#[test]
fn deck_gain_trim_delegates_to_mixer_channel() {
    let mut engine = started_null_engine();

    engine.set_deck_gain_trim_db(0, 1.5).unwrap();

    assert_eq!(with_channel(&engine, 0, |c| c.gain_trim_db()), 1.5);

    engine.stop().unwrap();
}

#[test]
fn deck_level_snapshot_reads_peaks_from_mixer_channel_not_deck() {
    let mut engine = started_null_engine();

    let track_id = TrackId::new("tone.wav");
    let prepared = PreparedTrackPlayback {
        track_id: track_id.clone(),
        source: AudioSource::File(FileAudioSource::new(
            track_id.clone(),
            PathBuf::from("/no/such/tone.wav"),
            TrackMetadata::default(),
        )),
        audio: Arc::new(LoadedAudio {
            samples: vec![0.5f32; 48_000 * 2],
            sample_rate: 48_000,
            channels: 2,
            source_id: track_id.to_string(),
        }),
        stems: None,
        loudness_lufs: None,
    };
    engine.load_prepared_track(0, prepared).unwrap();
    engine.play(0).unwrap();

    // The null backend never drains the pre-filled ring buffer, so the background
    // producer thread never calls `DspEngine::process`; drive one render cycle directly
    // through the same lock the producer thread uses.
    {
        let dsp_engine = engine.dsp_engine.as_ref().unwrap();
        let mut dsp = dsp_engine.lock().unwrap();
        let mut output_buses = std::collections::HashMap::new();
        output_buses.insert(BusId::new("master"), vec![0.0; 1024]);
        dsp.process(512, &mut output_buses).unwrap();
    }

    let snapshot_peak = engine
        .deck_level_snapshot()
        .into_iter()
        .find(|(deck_id, _, _)| *deck_id == 0)
        .map(|(_, peak_l, _)| peak_l);

    assert!(
        snapshot_peak.unwrap_or(0.0) > 0.0,
        "level snapshot should report a non-zero pre-fader peak once a channel processes audio"
    );

    // A constant-amplitude tone yields the same peak on every render cycle, so this
    // comparison is robust to how many cycles the producer thread ran in the background.
    let channel_peak = with_channel(&engine, 0, |c| c.level_peaks().peak_l);
    assert_eq!(
        snapshot_peak,
        Some(channel_peak),
        "deck_level_snapshot must mirror mixer channel peaks, not deck-owned state"
    );

    engine.stop().unwrap();
}

#[test]
fn unload_deck_clears_loudness_but_preserves_manual_controls() {
    let mut engine = started_null_engine();
    engine.set_normalizer_target(Some(-18.0)).unwrap();
    engine
        .load_prepared_track(0, prepared_test_source(Some(-24.0)))
        .unwrap();
    engine.set_deck_volume(0, 0.4).unwrap();
    engine.set_deck_eq_bands(0, 2.0, -1.0, 3.0).unwrap();
    engine.set_deck_filter_db(0, 5.0).unwrap();
    engine.set_deck_gain_trim_db(0, 1.5).unwrap();
    engine.set_deck_headphone_cue(0, true).unwrap();

    engine.unload_deck(0).unwrap();

    assert_eq!(
        with_channel(&engine, 0, |c| c.loudness_lufs()),
        None,
        "loudness must clear on unload"
    );
    assert_eq!(
        with_channel(&engine, 0, |c| c.auto_gain_db()),
        0.0,
        "auto gain must reset on unload"
    );
    assert_eq!(with_channel(&engine, 0, |c| c.volume()), 0.4);
    assert_eq!(with_channel(&engine, 0, |c| c.filter_db()), 5.0);
    assert_eq!(with_channel(&engine, 0, |c| c.gain_trim_db()), 1.5);
    assert!(with_channel(&engine, 0, |c| c.headphone_cue()));
    assert_eq!(
        with_channel(&engine, 0, |c| c.eq_gains()),
        DeckEqGains::clamped(2.0, -1.0, 3.0)
    );

    engine.stop().unwrap();
}

#[test]
fn test_engine_creation() {
    let config = EngineConfig::default();
    let engine = Engine::new(config);
    assert!(engine.is_ok());
}

#[test]
fn test_engine_deck_operations() {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();

    assert!(engine.play(0).is_err());
    assert!(engine.pause(0).is_err());
    assert!(engine
        .load_prepared_track(0, prepared_test_source(None))
        .is_err());

    engine.start().unwrap();

    assert!(engine.play(0).is_ok());
    assert!(engine.pause(0).is_ok());
    let missing = FileAudioSource::from_path("test.mp3").load();
    assert!(missing.is_err());
    assert!(missing.unwrap_err().to_string().contains("not found"));

    assert!(engine.play(2).is_err());
    assert!(engine
        .load_prepared_track(2, prepared_test_source(None))
        .is_err());

    engine.stop().unwrap();
}

#[test]
fn test_engine_device_operations() {
    let config = EngineConfig::default();
    let engine = Engine::new(config).unwrap();

    let devices = engine.list_devices();
    assert!(devices.is_ok());

    let default_device = engine.default_device();
    assert!(default_device.is_ok());
}

#[test]
fn test_engine_bus_operations() {
    let mut config = EngineConfig::default();
    config.buses.push(BusConfig::new(
        BusId::new("master".to_string()),
        "Master Bus".to_string(),
        DeviceId::new("default".to_string()),
        ChannelMapping::new(1, 2),
    ));

    let mut engine = Engine::new(config).unwrap();

    let master_bus_id = BusId::new("master".to_string());

    let bus_config = engine.get_bus_config(&master_bus_id);
    assert!(bus_config.is_some());

    let new_config = BusConfig::new(
        master_bus_id.clone(),
        "Updated Master".to_string(),
        DeviceId::new("default".to_string()),
        ChannelMapping::new(1, 2),
    )
    .with_sample_rate(Some(48_000))
    .with_low_latency(true);
    assert!(engine.update_bus_config(&master_bus_id, new_config).is_ok());

    let updated = engine.get_bus_config(&master_bus_id).expect("bus");
    assert_eq!(updated.sample_rate, Some(48_000));
    assert!(updated.low_latency);

    // A zero rate cannot open a stream and must not be stored.
    let zero_rate = BusConfig::new(
        master_bus_id.clone(),
        "Updated Master".to_string(),
        DeviceId::new("default".to_string()),
        ChannelMapping::new(1, 2),
    )
    .with_sample_rate(Some(0));
    assert!(engine.update_bus_config(&master_bus_id, zero_rate).is_err());
    assert_eq!(
        engine
            .get_bus_config(&master_bus_id)
            .expect("bus")
            .sample_rate,
        Some(48_000)
    );
}

#[test]
fn update_bus_config_rejects_a_rate_clash_on_a_shared_device() {
    let mut config = EngineConfig {
        backend: "null".into(),
        ..Default::default()
    };
    config.buses.push(
        BusConfig::new(
            BusId::new("master".to_string()),
            "Master".to_string(),
            DeviceId::new("default".to_string()),
            ChannelMapping::new(1, 2),
        )
        .with_sample_rate(Some(48_000)),
    );
    config.buses.push(BusConfig::new(
        BusId::new("cue".to_string()),
        "Cue".to_string(),
        DeviceId::new("default".to_string()),
        ChannelMapping::new(3, 4),
    ));

    let mut engine = Engine::new(config).unwrap();
    let cue_bus_id = BusId::new("cue".to_string());
    let clash = BusConfig::new(
        cue_bus_id.clone(),
        "Cue".to_string(),
        DeviceId::new("default".to_string()),
        ChannelMapping::new(3, 4),
    )
    .with_sample_rate(Some(44_100));

    let error = engine.update_bus_config(&cue_bus_id, clash).unwrap_err();
    assert!(
        error.to_string().contains("one stream runs at one rate"),
        "{error}"
    );
    // The rejected update is not stored.
    assert_eq!(
        engine.get_bus_config(&cue_bus_id).expect("bus").sample_rate,
        None
    );
}

#[test]
fn set_bus_device_updates_master_config() {
    let config = EngineConfig {
        backend: "null".into(),
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
    engine
        .set_bus_device(BusId::new("master"), DeviceId::new("null-device"), [3, 4])
        .unwrap();
    let bus = engine.get_bus_config(&BusId::new("master")).unwrap();
    assert_eq!(bus.channels.left, 3);
    assert_eq!(bus.channels.right, 4);
    assert_eq!(bus.device.as_str(), "null-device");
}

#[test]
fn set_bus_device_rejects_overlap_on_same_device() {
    let config = EngineConfig {
        backend: "null".into(),
        buses: vec![
            BusConfig::new(
                BusId::new("master"),
                "Master".into(),
                DeviceId::new("null-device"),
                ChannelMapping::new(1, 2),
            ),
            BusConfig::new(
                BusId::new("cue"),
                "Preview".into(),
                DeviceId::new("null-device"),
                ChannelMapping::new(3, 4),
            ),
        ],
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
    let err = engine
        .set_bus_device(BusId::new("cue"), DeviceId::new("null-device"), [2, 3])
        .unwrap_err();
    assert!(err.to_string().contains("overlaps"));
}

#[test]
fn set_bus_channel_mapping_accepts_mono() {
    let config = EngineConfig {
        backend: "null".into(),
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
    engine
        .set_bus_channel_mapping(
            BusId::new("master"),
            DeviceId::new("null-device"),
            ChannelMapping::mono(1),
        )
        .unwrap();
    engine
        .set_bus_channel_mapping(
            BusId::new("cue"),
            DeviceId::new("null-device"),
            ChannelMapping::mono(2),
        )
        .unwrap();
    let master = engine.get_bus_config(&BusId::new("master")).unwrap();
    assert!(master.channels.is_mono());
    assert_eq!(master.channels.left, 1);
    let cue = engine.get_bus_config(&BusId::new("cue")).unwrap();
    assert!(cue.channels.is_mono());
    assert_eq!(cue.channels.left, 2);
}

#[test]
fn test_engine_producer_consumer_architecture() {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();

    assert!(engine.start().is_ok());
    assert!(engine.stop().is_ok());
}

#[test]
fn stem_eq_toggle_round_trips_and_keeps_band_positions() {
    let mut engine = started_null_engine();
    engine.set_deck_stem_eq_enabled(0, true).unwrap();
    assert_eq!(engine.deck_stem_eq_enabled(0), Some(true));
    engine
        .set_deck_stem_eq_band(0, bus_api::EqBand::Low, 0.0)
        .unwrap();

    // Off keeps the stored positions: the DSP ignores them, then restores them.
    engine.set_deck_stem_eq_enabled(0, false).unwrap();
    assert_eq!(engine.deck_stem_eq_enabled(0), Some(false));
    assert_eq!(engine.deck_stem_eq(0), Some([0.0, 0.5, 0.5]));
    engine.set_deck_stem_eq_enabled(0, true).unwrap();
    assert_eq!(engine.deck_stem_eq_enabled(0), Some(true));
    assert_eq!(engine.deck_stem_eq(0), Some([0.0, 0.5, 0.5]));

    // Unknown deck ids are rejected instead of silently ignored.
    assert!(engine.set_deck_stem_eq_enabled(9, true).is_err());
    assert_eq!(engine.deck_stem_eq_enabled(9), None);
    engine.stop().unwrap();
}

#[test]
fn unload_pushes_the_stem_eq_toggle_into_the_dsp_deck() {
    let mut engine = started_null_engine();
    engine
        .load_track(
            0,
            AudioSource::File(FileAudioSource::from_path(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../samples/fixtures/short-tone.wav"),
            )),
        )
        .unwrap();
    engine.set_deck_stem_eq_enabled(0, true).unwrap();
    assert_eq!(engine.deck_stem_eq_enabled(0), Some(true));
    assert_eq!(engine.deck_stem_eq_armed(0), Some(true), "DSP mirror armed");

    // Unload resets the toggle; the DSP mirror must follow, or the knobs and the
    // ST key would claim stem EQ while the deck plays the plain mix.
    engine.unload_deck(0).unwrap();
    assert_eq!(engine.deck_stem_eq_enabled(0), Some(false));
    assert_eq!(
        engine.deck_stem_eq_armed(0),
        Some(false),
        "DSP mirror reset"
    );
}

fn linear_crossfader_group() -> curve_core::CurveGroup {
    use curve_core::{CurveGroup, CurvePoint, CurveSpec, SegmentKind};
    let curve = |y0: f32, y1: f32| {
        CurveSpec::new(
            vec![CurvePoint::new(0.0, y0), CurvePoint::new(1.0, y1)],
            vec![SegmentKind::Linear],
        )
        .expect("valid linear curve")
    };
    CurveGroup::new(vec![curve(1.0, 0.0), curve(0.0, 1.0)]).expect("valid two-channel group")
}

#[test]
fn crossfader_preset_and_curve_set_install_without_restart() {
    use crate::curve_presets::CrossfaderPreset;
    use audio_core::CrossfaderCurveBehavior;

    let mut engine = started_null_engine();
    engine.set_crossfader(0.5).unwrap();

    let (gain_a, gain_b) = engine.resolved_crossfader_gains().unwrap();
    assert!((gain_a - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    assert!((gain_a - gain_b).abs() < 1e-6);

    engine
        .set_crossfader_preset(CrossfaderPreset::Linear)
        .unwrap();
    let (gain_a, gain_b) = engine.resolved_crossfader_gains().unwrap();
    assert!((gain_a - 0.5).abs() < 1e-4);
    assert!((gain_b - 0.5).abs() < 1e-4);

    // Behavior survives a preset swap and applies to the new tables.
    engine
        .set_crossfader_behavior(CrossfaderCurveBehavior::hamster())
        .unwrap();
    engine
        .set_crossfader_preset(CrossfaderPreset::FastCut { cut: 0.15 })
        .unwrap();
    assert!(engine.crossfader_behavior().unwrap().hamster);
    engine.set_crossfader(0.0).unwrap();
    let (gain_a, gain_b) = engine.resolved_crossfader_gains().unwrap();
    assert!(gain_a.abs() < 1e-6, "hamster swaps the open side");
    assert!((gain_b - 1.0).abs() < 1e-6);

    // A curve set installs without restarting audio and morphs per block.
    engine
        .set_crossfader_curve_set(vec![linear_crossfader_group(), linear_crossfader_group()])
        .unwrap();
    engine.set_crossfader(0.5).unwrap();
    engine.set_crossfader_curve_morph(0.5).unwrap();
    assert_eq!(engine.crossfader_curve_morph(), Some(0.5));
    let (gain_a, gain_b) = engine.resolved_crossfader_gains().unwrap();
    assert!((gain_a - 0.5).abs() < 1e-4);
    assert!((gain_b - 0.5).abs() < 1e-4);

    engine.stop().unwrap();
}

/// Null-backend engine started with explicit audio defaults.
fn started_null_engine_with_audio(audio: crate::config::AudioConfig) -> Engine {
    let config = EngineConfig {
        backend: "null".into(),
        audio: Some(audio),
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
    engine.start().unwrap();
    engine
}

fn linear_curve_spec() -> bus_api::CurveSpecSpec {
    bus_api::CurveSpecSpec {
        points: vec![
            bus_api::CurvePointSpec { x: 0.0, y: 1.0 },
            bus_api::CurvePointSpec { x: 1.0, y: 0.0 },
        ],
        segments: vec![bus_api::CurveSegmentSpec {
            kind: bus_api::CurveSegmentKind::Linear,
            rate: 0.0,
            c1x: 0.0,
            c1y: 0.0,
            c2x: 0.0,
            c2y: 0.0,
        }],
    }
}

/// A two-channel custom group with the given A-side endpoint values.
fn custom_group(id: &str, a_start: f32, a_end: f32) -> bus_api::CurveGroupSpec {
    let mut deck_a = linear_curve_spec();
    deck_a.points[0].y = a_start;
    deck_a.points[1].y = a_end;
    let deck_b = bus_api::CurveSpecSpec {
        points: vec![
            bus_api::CurvePointSpec {
                x: 0.0,
                y: 1.0 - a_start,
            },
            bus_api::CurvePointSpec {
                x: 1.0,
                y: 1.0 - a_end,
            },
        ],
        segments: deck_a.segments.clone(),
    };
    bus_api::CurveGroupSpec {
        id: id.to_string(),
        display_name: id.to_string(),
        curves: vec![deck_a, deck_b],
    }
}

/// A one-channel custom group whose (x, y) points all use linear segments.
fn one_channel_group(id: &str, points: &[(f32, f32)]) -> bus_api::CurveGroupSpec {
    bus_api::CurveGroupSpec {
        id: id.to_string(),
        display_name: id.to_string(),
        curves: vec![bus_api::CurveSpecSpec {
            points: points
                .iter()
                .map(|(x, y)| bus_api::CurvePointSpec { x: *x, y: *y })
                .collect(),
            segments: vec![
                bus_api::CurveSegmentSpec {
                    kind: bus_api::CurveSegmentKind::Linear,
                    rate: 0.0,
                    c1x: 0.0,
                    c1y: 0.0,
                    c2x: 0.0,
                    c2y: 0.0,
                };
                points.len() - 1
            ],
        }],
    }
}

fn empty_audio() -> crate::config::AudioConfig {
    crate::config::AudioConfig {
        resampler_quality: Some("medium".into()),
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
        fader_start: None,
    }
}

#[test]
fn installing_a_preset_by_value_updates_the_status_selection() {
    use crate::curve_presets::CrossfaderPreset;

    let mut engine = started_null_engine();
    engine
        .set_crossfader_preset(CrossfaderPreset::Linear)
        .unwrap();
    let status = engine.engine_status_snapshot().unwrap().crossfader_curve;
    assert_eq!(status.id, "linear");
    assert_eq!(status.display_name, "Linear");
    engine.stop().unwrap();
}

#[test]
fn configured_default_curve_and_behavior_apply_on_start() {
    let mut audio = empty_audio();
    audio.default_crossfader_curve = Some("Linear".into());
    audio.crossfader_behavior = Some(bus_api::CrossfaderBehaviorSetting {
        hamster: true,
        disabled: false,
    });
    let mut engine = started_null_engine_with_audio(audio);
    engine.set_crossfader(0.0).unwrap();

    // Linear with hamster swaps the ends: A is silent at the far left.
    let (gain_a, gain_b) = engine.resolved_crossfader_gains().unwrap();
    assert!(gain_a.abs() < 1e-4);
    assert!((gain_b - 1.0).abs() < 1e-4);

    let status = engine.engine_status_snapshot().unwrap().crossfader_curve;
    assert_eq!(status.id, "linear", "aliases normalize to snake_case ids");
    assert!(status.hamster);
    assert!(!status.disabled);
    assert!(status.morph.abs() < 1e-6);

    engine.stop().unwrap();
}

#[test]
fn unknown_default_curve_falls_back_to_constant_power() {
    let mut audio = empty_audio();
    audio.default_crossfader_curve = Some("no_such_curve".into());
    let mut engine = started_null_engine_with_audio(audio);
    engine.set_crossfader(0.5).unwrap();
    let (gain_a, gain_b) = engine.resolved_crossfader_gains().unwrap();
    assert!((gain_a - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
    assert!((gain_a - gain_b).abs() < 1e-6);
    assert_eq!(
        engine.engine_status_snapshot().unwrap().crossfader_curve.id,
        bus_api::DEFAULT_CROSSFADER_CURVE_ID
    );
    engine.stop().unwrap();
}

#[test]
fn default_channel_fader_curve_is_linear_and_preserves_volume() {
    let mut engine = started_null_engine();
    assert_eq!(
        engine.channel_fader_curve_selection().id,
        bus_api::DEFAULT_CHANNEL_FADER_CURVE_ID
    );

    engine.set_deck_volume(0, 0.25).unwrap();
    let gain = with_channel(&engine, 0, |channel| channel.fader_gain());
    assert!((gain - 0.25).abs() < 1e-6, "linear fader gain: {gain}");

    engine.stop().unwrap();
}

#[test]
fn configured_custom_channel_fader_curve_applies_on_start() {
    let mut audio = empty_audio();
    audio.default_channel_fader_curve = Some("taper".into());
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        groups: vec![one_channel_group(
            "taper",
            &[(0.0, 0.0), (0.5, 0.25), (1.0, 1.0)],
        )],
        sets: vec![],
    });
    let mut engine = started_null_engine_with_audio(audio);

    assert_eq!(
        engine.channel_fader_curve_selection().id,
        "taper",
        "the configured default is the active selection"
    );
    engine.set_deck_volume(0, 0.5).unwrap();
    let gain = with_channel(&engine, 0, |channel| channel.fader_gain());
    assert!((gain - 0.25).abs() < 1e-4, "custom fader gain: {gain}");

    engine.stop().unwrap();
}

#[test]
fn custom_channel_fader_curve_changes_without_restart() {
    let mut engine = started_null_engine();
    engine.set_deck_volume(0, 0.5).unwrap();
    assert!((with_channel(&engine, 0, |channel| channel.fader_gain()) - 0.5).abs() < 1e-6);

    engine
        .set_curve_library(&bus_api::CurveLibrarySpec {
            groups: vec![one_channel_group(
                "taper",
                &[(0.0, 0.0), (0.5, 0.25), (1.0, 1.0)],
            )],
            sets: vec![],
        })
        .unwrap();
    assert_eq!(engine.select_channel_fader_curve("taper").unwrap(), None);

    assert!(
        engine.engine_status_snapshot().is_some(),
        "engine is running"
    );
    let gain = with_channel(&engine, 0, |channel| channel.fader_gain());
    assert!((gain - 0.25).abs() < 1e-4, "runtime fader gain: {gain}");
    assert_eq!(engine.channel_fader_curve_selection().id, "taper");

    engine.stop().unwrap();
}

#[test]
fn invalid_channel_fader_curves_fall_back_to_linear_with_a_notice() {
    // A two-channel group is valid for the crossfader but not a scalar fader.
    let mut audio = empty_audio();
    audio.default_channel_fader_curve = Some("pair".into());
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        groups: vec![custom_group("pair", 1.0, 0.0)],
        sets: vec![],
    });
    let mut engine = started_null_engine_with_audio(audio);
    assert_eq!(
        engine.channel_fader_curve_selection().id,
        bus_api::DEFAULT_CHANNEL_FADER_CURVE_ID
    );

    engine.set_deck_volume(0, 0.5).unwrap();
    let notice = engine.select_channel_fader_curve("pair").unwrap();
    assert!(notice.expect("notice").contains("needs 1 curves"));
    assert!((with_channel(&engine, 0, |channel| channel.fader_gain()) - 0.5).abs() < 1e-6);

    let two_channel = curve_core::ResolvedCurveGroup::single(
        &curve_core::CurveGroup::new(vec![
            curve_core::presets::linear(),
            curve_core::presets::linear(),
        ])
        .unwrap(),
    )
    .unwrap();
    assert!(engine.set_channel_fader_curve(two_channel).is_err());
    assert!((with_channel(&engine, 0, |channel| channel.fader_gain()) - 0.5).abs() < 1e-6);

    engine.stop().unwrap();
}

#[test]
fn default_tempo_curve_is_linear_and_preserves_tempo() {
    let mut engine = started_null_engine();
    assert_eq!(
        engine.tempo_curve_selection(0).expect("deck 0").id,
        bus_api::DEFAULT_TEMPO_CURVE_ID
    );

    for (speed, expected) in [
        (0.0, 1.06),
        (0.25, 1.03),
        (0.5, 1.0),
        (0.75, 0.97),
        (1.0, 0.94),
    ] {
        engine.set_deck_speed(0, speed).unwrap();
        let ratio = with_deck(&engine, 0, |deck| deck.playback_ratio());
        assert!(
            (ratio - expected).abs() < 1e-5,
            "speed {speed}: ratio {ratio} != {expected}"
        );
    }

    engine.stop().unwrap();
}

#[test]
fn configured_custom_tempo_curve_applies_on_start() {
    let mut audio = empty_audio();
    audio.default_tempo_curve = Some("taper".into());
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        groups: vec![one_channel_group(
            "taper",
            &[(0.0, 1.0), (0.25, 0.85), (0.5, 0.5), (1.0, 0.0)],
        )],
        sets: vec![],
    });
    let mut engine = started_null_engine_with_audio(audio);

    assert_eq!(
        engine.tempo_curve_selection(0).expect("deck 0").id,
        "taper",
        "the configured default is the active selection"
    );
    engine.set_deck_speed(0, 0.25).unwrap();
    let ratio = with_deck(&engine, 0, |deck| deck.playback_ratio());
    assert!((ratio - 1.042).abs() < 1e-4, "custom tempo ratio: {ratio}");

    engine.stop().unwrap();
}

#[test]
fn custom_tempo_curve_keeps_the_center_detent_and_range() {
    let mut audio = empty_audio();
    audio.default_tempo_curve = Some("offcenter".into());
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        // The curve's own midpoint is 0.8, not 0.5. The centered tempo mapping
        // must still put unity at the fader center and reach both signed ends.
        groups: vec![one_channel_group(
            "offcenter",
            &[(0.0, 1.0), (0.5, 0.8), (1.0, 0.0)],
        )],
        sets: vec![],
    });
    let mut engine = started_null_engine_with_audio(audio);

    for (speed, expected) in [(0.0, 1.06), (0.5, 1.0), (1.0, 0.94)] {
        engine.set_deck_speed(0, speed).unwrap();
        let ratio = with_deck(&engine, 0, |deck| deck.playback_ratio());
        assert!(
            (ratio - expected).abs() < 1e-5,
            "speed {speed}: ratio {ratio} != {expected}"
        );
    }

    engine.stop().unwrap();
}

#[test]
fn tempo_curve_can_change_without_restart() {
    let mut engine = started_null_engine();
    engine.set_deck_speed(0, 0.25).unwrap();
    assert!((with_deck(&engine, 0, |deck| deck.playback_ratio()) - 1.03).abs() < 1e-5);

    engine
        .set_curve_library(&bus_api::CurveLibrarySpec {
            groups: vec![one_channel_group(
                "taper",
                &[(0.0, 1.0), (0.25, 0.85), (0.5, 0.5), (1.0, 0.0)],
            )],
            sets: vec![],
        })
        .unwrap();
    assert_eq!(engine.select_tempo_curve(0, "taper").unwrap(), None);
    assert_eq!(engine.tempo_curve_selection(0).expect("deck 0").id, "taper");
    assert!((with_deck(&engine, 0, |deck| deck.playback_ratio()) - 1.042).abs() < 1e-4);

    engine.stop().unwrap();
}

#[test]
fn invalid_tempo_curves_fall_back_to_linear_with_a_notice() {
    let mut audio = empty_audio();
    audio.default_tempo_curve = Some("pair".into());
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        groups: vec![custom_group("pair", 1.0, 0.0)],
        sets: vec![],
    });
    let mut engine = started_null_engine_with_audio(audio);
    assert_eq!(
        engine.tempo_curve_selection(0).expect("deck 0").id,
        bus_api::DEFAULT_TEMPO_CURVE_ID
    );

    let notice = engine.select_tempo_curve(0, "pair").unwrap();
    assert!(notice.expect("notice").contains("needs 1 curves"));
    engine.set_deck_speed(0, 0.5).unwrap();
    assert!((with_deck(&engine, 0, |deck| deck.playback_ratio()) - 1.0).abs() < 1e-6);
    assert_eq!(
        engine.tempo_curve_selection(0).expect("deck 0").id,
        bus_api::DEFAULT_TEMPO_CURVE_ID
    );

    engine.stop().unwrap();
}

#[test]
fn default_fx_level_curve_is_linear_and_preserves_wet() {
    let mut engine = started_null_engine();
    assert_eq!(
        engine.fx_level_curve_selection().id,
        bus_api::DEFAULT_FX_LEVEL_CURVE_ID
    );

    engine.set_fx_unit_wet(0, 0.25).unwrap();
    assert!((engine.fx_unit_wet(0).expect("slot 0") - 0.25).abs() < 1e-6);
    assert!((engine.fx_unit_level(0).expect("slot 0") - 0.25).abs() < 1e-6);

    engine.stop().unwrap();
}

#[test]
fn configured_custom_fx_level_curve_applies_on_start() {
    let mut audio = empty_audio();
    audio.default_fx_level_curve = Some("taper".into());
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        groups: vec![one_channel_group(
            "taper",
            &[(0.0, 0.0), (0.5, 0.25), (1.0, 1.0)],
        )],
        sets: vec![],
    });
    let mut engine = started_null_engine_with_audio(audio);

    assert_eq!(
        engine.fx_level_curve_selection().id,
        "taper",
        "the configured default is the active selection"
    );
    engine.set_fx_unit_wet(0, 0.5).unwrap();
    assert!(
        (engine.fx_unit_wet(0).expect("slot 0") - 0.25).abs() < 1e-4,
        "custom level: {:?}",
        engine.fx_unit_wet(0)
    );
    assert!(
        (engine.fx_unit_level(0).expect("slot 0") - 0.5).abs() < 1e-6,
        "the raw control position is preserved"
    );

    engine.stop().unwrap();
}

#[test]
fn custom_fx_level_curve_changes_without_restart() {
    let mut engine = started_null_engine();
    engine.set_fx_unit_wet(0, 0.5).unwrap();
    assert!((engine.fx_unit_wet(0).expect("slot 0") - 0.5).abs() < 1e-6);

    engine
        .set_curve_library(&bus_api::CurveLibrarySpec {
            groups: vec![one_channel_group(
                "taper",
                &[(0.0, 0.0), (0.5, 0.25), (1.0, 1.0)],
            )],
            sets: vec![],
        })
        .unwrap();
    assert_eq!(engine.select_fx_level_curve("taper").unwrap(), None);
    assert_eq!(engine.fx_level_curve_selection().id, "taper");

    assert!(
        engine.engine_status_snapshot().is_some(),
        "engine is running"
    );
    assert!(
        (engine.fx_unit_wet(0).expect("slot 0") - 0.25).abs() < 1e-4,
        "runtime level: {:?}",
        engine.fx_unit_wet(0)
    );

    engine.stop().unwrap();
}

#[test]
fn invalid_fx_level_curves_fall_back_to_linear_with_a_notice() {
    // A two-channel group is valid for the crossfader but not a scalar level.
    let mut audio = empty_audio();
    audio.default_fx_level_curve = Some("pair".into());
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        groups: vec![custom_group("pair", 1.0, 0.0)],
        sets: vec![],
    });
    let mut engine = started_null_engine_with_audio(audio);
    assert_eq!(
        engine.fx_level_curve_selection().id,
        bus_api::DEFAULT_FX_LEVEL_CURVE_ID
    );

    engine.set_fx_unit_wet(0, 0.5).unwrap();
    let notice = engine.select_fx_level_curve("pair").unwrap();
    assert!(notice.expect("notice").contains("needs 1 curves"));
    assert!((engine.fx_unit_wet(0).expect("slot 0") - 0.5).abs() < 1e-6);

    let two_channel = curve_core::ResolvedCurveGroup::single(
        &curve_core::CurveGroup::new(vec![
            curve_core::presets::linear(),
            curve_core::presets::linear(),
        ])
        .unwrap(),
    )
    .unwrap();
    assert!(engine.set_fx_level_curve(two_channel).is_err());
    assert!((engine.fx_unit_wet(0).expect("slot 0") - 0.5).abs() < 1e-6);

    engine.stop().unwrap();
}

#[test]
fn curve_library_selects_custom_groups_and_reports_status() {
    let mut engine = started_null_engine();
    engine.set_crossfader(0.0).unwrap();
    let library = bus_api::CurveLibrarySpec {
        groups: vec![
            custom_group("tight", 1.0, 0.0),
            custom_group("flat", 1.0, 1.0),
        ],
        sets: vec![bus_api::CurveSetSpec {
            id: "sweep".into(),
            display_name: "Sweep".into(),
            members: vec![
                bus_api::CurveRefSpec {
                    id: "tight".into(),
                    parameter: None,
                },
                bus_api::CurveRefSpec {
                    id: "linear".into(),
                    parameter: None,
                },
            ],
        }],
    };
    engine.set_curve_library(&library).unwrap();
    assert!(engine.curve_library().group("tight").is_some());

    assert_eq!(engine.select_crossfader_curve("flat").unwrap(), None);
    assert_eq!(
        engine.engine_status_snapshot().unwrap().crossfader_curve.id,
        "flat"
    );
    let (gain_a, _) = engine.resolved_crossfader_gains().unwrap();
    assert!(
        (gain_a - 1.0).abs() < 1e-4,
        "flat group keeps A at full gain"
    );

    // A knob sweep selects the set and moves the morph.
    assert_eq!(
        engine
            .select_crossfader_curve_set("sweep", 0.5, true)
            .unwrap(),
        None
    );
    let status = engine.engine_status_snapshot().unwrap().crossfader_curve;
    assert_eq!(status.id, "sweep");
    assert!((status.morph - 0.5).abs() < 1e-6);

    // Behavior flags persist across selections and land in the status.
    engine
        .apply_crossfader_behavior(bus_api::CrossfaderBehaviorSetting {
            hamster: false,
            disabled: true,
        })
        .unwrap();
    let (gain_a, gain_b) = engine.resolved_crossfader_gains().unwrap();
    assert!((gain_a - 1.0).abs() < 1e-6 && (gain_b - 1.0).abs() < 1e-6);
    assert!(
        engine
            .engine_status_snapshot()
            .unwrap()
            .crossfader_curve
            .disabled
    );

    engine.stop().unwrap();
}

#[test]
fn unknown_curve_ids_fall_back_with_a_notice_message() {
    let mut engine = started_null_engine();
    engine.select_crossfader_curve("linear").unwrap();
    assert_eq!(
        engine.engine_status_snapshot().unwrap().crossfader_curve.id,
        "linear"
    );

    let notice = engine.select_crossfader_curve("nope").unwrap();
    let notice = notice.expect("unknown ids report a notice");
    assert!(notice.contains("nope"), "{notice}");
    assert_eq!(
        engine.engine_status_snapshot().unwrap().crossfader_curve.id,
        bus_api::DEFAULT_CROSSFADER_CURVE_ID
    );

    engine.stop().unwrap();
}

#[test]
fn duplicate_curve_ids_reject_the_library_and_keep_the_previous_one() {
    let mut engine = started_null_engine();
    let library = bus_api::CurveLibrarySpec {
        groups: vec![custom_group("tight", 1.0, 0.0)],
        sets: vec![],
    };
    engine.set_curve_library(&library).unwrap();

    let duplicate = bus_api::CurveLibrarySpec {
        groups: vec![
            custom_group("tight", 1.0, 0.0),
            custom_group("tight", 0.0, 1.0),
        ],
        sets: vec![],
    };
    let error = engine.set_curve_library(&duplicate).unwrap_err();
    assert!(
        error.to_string().to_lowercase().contains("duplicate"),
        "{error}"
    );
    assert!(engine.curve_library().group("tight").is_some());
    assert_eq!(engine.curve_library().groups.len(), 1);

    engine.stop().unwrap();
}

#[test]
fn config_rejects_a_curve_library_with_duplicate_ids() {
    let mut audio = empty_audio();
    audio.curve_library = Some(bus_api::CurveLibrarySpec {
        groups: vec![
            custom_group("same", 1.0, 0.0),
            custom_group("same", 0.0, 1.0),
        ],
        sets: vec![],
    });
    let config = EngineConfig {
        backend: "null".into(),
        audio: Some(audio),
        ..Default::default()
    };
    let error = config.validate().unwrap_err();
    assert!(
        error.to_string().to_lowercase().contains("duplicate"),
        "{error}"
    );
}
