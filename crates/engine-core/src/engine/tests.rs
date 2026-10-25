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

fn started_null_engine() -> Engine {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
    engine.start().unwrap();
    engine
}

#[test]
fn four_deck_engine_exposes_deck_four_and_rejects_deck_five() {
    let config = EngineConfig {
        backend: "null".into(),
        num_decks: 4,
        ..Default::default()
    };
    let mut engine = Engine::new(config).unwrap();
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
    );
    assert!(engine.update_bus_config(&master_bus_id, new_config).is_ok());
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
