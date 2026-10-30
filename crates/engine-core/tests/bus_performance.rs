//! Integration: performance cmds (cue / loop / beat jump / unload) on the bus.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::BeatGrid;
use bus_api::BeatJump;
use bus_api::Event;
use bus_api::ExitLoop;
use bus_api::LoopIn;
use bus_api::LoopOut;
use bus_api::Seek;
use bus_api::SetAutoLoop;
use bus_api::SetCuePoint;
use bus_api::SetQuantize;
use bus_api::Unload;
use common::recv_error;
use common::recv_updated;
use common::short_tone_fixture;
use common::source_with_bpm;
use common::write_two_second_wav;
use common::SHORT_TONE_LEN_MS;
use engine_core::EngineConfig;
use engine_core::EngineSession;
use library::LibraryConfig;
use library::NewCollection;
use library::WritableLibrary;
use library_core::AudioSource;
use library_core::FileAudioSource;
use library_core::Library;
use library_core::TrackId;
use library_core::TrackMetadata;
use std::sync::Arc;
use std::sync::Mutex;

fn loop_in_cmd(position_ms: i32) -> Event {
    Event::LoopIn(LoopIn {
        deck: 0,
        position_ms,
    })
}

fn loop_out_cmd(position_ms: i32) -> Event {
    Event::LoopOut(LoopOut {
        deck: 0,
        position_ms,
    })
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
            engine.load_track(0, source_with_bpm("perf.wav", 120.0))?;
            Ok(())
        })
        .expect("load");
    session
}

#[test]
fn set_auto_loop_publishes_active_loop() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: 0,
            beats: 4.0,
        }))
        .expect("auto loop");

    let update = recv_updated(&mut evt);
    let region = update.active_loop.expect("loop region");
    assert!(region.active);
    assert!(region.out_ms > region.in_ms);
}

#[test]
fn set_auto_loop_snaps_in_to_nearest_beat_even_when_quantize_off() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize off");
    let _ = recv_updated(&mut evt);

    // Fixture is 250 ms; 120 BPM → 500 ms/beat. 100 ms snaps down to 0.
    session
        .publish_cmd(Event::Seek(Seek {
            deck: 0,
            position_ms: 100,
        }))
        .expect("seek");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
            deck: 0,
            beats: 1.0,
        }))
        .expect("auto loop");
    let update = recv_updated(&mut evt);
    let region = update.active_loop.expect("loop region");
    assert_eq!(region.in_ms, 0);
    assert!(region.out_ms > region.in_ms);
}

#[test]
fn set_quantize_and_cue_point_roundtrip() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize");
    let q = recv_updated(&mut evt);
    assert!(!q.quantize);

    session
        .publish_cmd(Event::SetCuePoint(SetCuePoint { deck: 0 }))
        .expect("cue");
    let cue = recv_updated(&mut evt);
    assert!(cue.cue_point_ms.is_some());

    session.publish_cmd(loop_in_cmd(0)).expect("loop in");
    let loop_in = recv_updated(&mut evt);
    assert!(loop_in.active_loop.is_none());
    assert!(loop_in.pending_loop_in_ms.is_some());

    session
        .publish_cmd(Event::BeatJump(BeatJump {
            deck: 0,
            beats: 1.0,
        }))
        .expect("jump");
    let jumped = recv_updated(&mut evt);
    // Quantize is off, so no snapping: 1 beat at 120 BPM = 60/120 s = 500 ms,
    // jumped forward from position 0. (A seek is not clamped to the 0.25 s
    // fixture length — see `DspDeck::seek_ms_inner`.)
    assert_eq!(jumped.position_ms, Some(500));
}

#[test]
fn loop_in_preserves_exact_position_when_quantize_off() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize off");
    let _ = recv_updated(&mut evt);

    session.publish_cmd(loop_in_cmd(123)).expect("loop in");
    let loop_in = recv_updated(&mut evt);
    assert!(loop_in.active_loop.is_none());
    assert_eq!(loop_in.pending_loop_in_ms, Some(123));
}

#[test]
fn loop_in_snaps_when_quantize_on() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    // 120 BPM → 500 ms/beat; 200 ms → nearest beat 0 (fixture is 250 ms).
    session.publish_cmd(loop_in_cmd(200)).expect("loop in");
    let loop_in = recv_updated(&mut evt);
    assert_eq!(loop_in.pending_loop_in_ms, Some(0));
}

fn phase_grid(first_beat_secs: f32) -> BeatGrid {
    BeatGrid {
        beats: vec![first_beat_secs],
        downbeats: vec![first_beat_secs],
        bpm: 120.0,
    }
}

#[test]
fn loop_in_snaps_to_first_beat_phase_when_quantize_on() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_two_second_wav(&wav);

    let mut library = library::LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = library::LibraryBuses::new();
    library.set_buses(buses.clone());
    let track_id = {
        let folder = library
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        library.sync_collection(Some(&folder.id)).unwrap();
        let tracks = library.list_collection_tracks(&folder.id).unwrap();
        let id = tracks[0].id().clone();
        library
            .save_track_beat_grid(&id, 120.0, 0.1)
            .expect("save beat grid");
        id
    };
    let library = Arc::new(Mutex::new(library));

    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session =
        EngineSession::new_with_library_bus(config, library, buses.bus()).expect("engine session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    track_id.clone(),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            Ok(())
        })
        .expect("load");

    let mut evt = session.subscribe_evt_all().expect("sub");

    // Hydrated phase 100 ms; 200 → nearest beat 100 (not 0).
    session.publish_cmd(loop_in_cmd(200)).expect("loop in");
    let loop_in = recv_updated(&mut evt);
    assert_eq!(loop_in.pending_loop_in_ms, Some(100));
}

#[test]
fn loop_out_snaps_to_grid_phase_when_quantize_on() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("song.wav");
    write_two_second_wav(&wav);

    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    TrackId::new("phase-loop.wav"),
                    wav.clone(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )?;
            let changed = engine.apply_track_beat_grid("phase-loop.wav", &phase_grid(0.1));
            assert!(
                !changed.is_empty(),
                "beat grid must apply to loaded deck 0 (track phase-loop.wav)"
            );
            Ok(())
        })
        .expect("load");

    let mut evt = session.subscribe_evt_all().expect("sub");

    session.publish_cmd(loop_in_cmd(200)).expect("loop in");
    let _ = recv_updated(&mut evt);

    // Phase 100: beats at 100, 600, 1100…; raw 700 → 600.
    session.publish_cmd(loop_out_cmd(700)).expect("loop out");
    let loop_out = recv_updated(&mut evt);
    let region = loop_out.active_loop.expect("active loop");
    assert_eq!(region.in_ms, 100);
    assert_eq!(region.out_ms, 600);
}

#[test]
fn beat_grid_changed_updates_quantize_phase() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    // Phase 0 initially: 200 → 0.
    session.publish_cmd(loop_in_cmd(200)).expect("loop in");
    let first = recv_updated(&mut evt);
    assert_eq!(first.pending_loop_in_ms, Some(0));

    session
        .with_engine(|engine| {
            let changed = engine.apply_track_beat_grid("perf.wav", &phase_grid(0.1));
            assert!(
                !changed.is_empty(),
                "beat grid must apply to loaded deck 0 (track perf.wav)"
            );
            Ok(())
        })
        .expect("apply grid");

    session
        .publish_cmd(loop_in_cmd(200))
        .expect("loop in after phase");
    let second = recv_updated(&mut evt);
    assert_eq!(second.pending_loop_in_ms, Some(100));
}

#[test]
fn loop_out_completes_pending_and_activates() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize off");
    let _ = recv_updated(&mut evt);

    session.publish_cmd(loop_in_cmd(40)).expect("loop in");
    let _ = recv_updated(&mut evt);

    session.publish_cmd(loop_out_cmd(200)).expect("loop out");
    let loop_out = recv_updated(&mut evt);
    let region = loop_out.active_loop.expect("active after out");
    assert!(region.active);
    assert_eq!(region.in_ms, 40);
    assert_eq!(region.out_ms, 200);
    assert!(loop_out.pending_loop_in_ms.is_none());
}

#[test]
fn loop_out_without_pending_or_active_errors() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(loop_out_cmd(1000))
        .expect("loop out cmd");
    let error = recv_error(&mut evt);
    assert!(
        error.message.contains("Set Loop In before Loop Out"),
        "unexpected: {}",
        error.message
    );
}

#[test]
fn exit_loop_clears_pending() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session.publish_cmd(loop_in_cmd(40)).expect("loop in");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(Event::ExitLoop(ExitLoop { deck: 0 }))
        .expect("exit");
    let exit = recv_updated(&mut evt);
    assert!(exit.active_loop.is_none());
    assert!(exit.pending_loop_in_ms.is_none());
}

#[test]
fn pending_loop_in_survives_quantize_toggle() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session.publish_cmd(loop_in_cmd(200)).expect("loop in");
    let pending = recv_updated(&mut evt);
    assert_eq!(pending.pending_loop_in_ms, Some(0));

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize off");
    let after = recv_updated(&mut evt);
    assert_eq!(after.pending_loop_in_ms, Some(0));
}

#[test]
fn loop_out_uses_quantize_at_out_time_not_in_time() {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    // 480 BPM → 125 ms/beat; fits the 250 ms fixture (beats at 0, 125, 250).
    session
        .with_engine(|engine| {
            engine.load_track(0, source_with_bpm("mid-q.wav", 480.0))?;
            Ok(())
        })
        .expect("load");
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize off");
    let _ = recv_updated(&mut evt);

    session.publish_cmd(loop_in_cmd(50)).expect("loop in");
    let pending = recv_updated(&mut evt);
    assert_eq!(pending.pending_loop_in_ms, Some(50));

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: true,
        }))
        .expect("quantize on");
    let _ = recv_updated(&mut evt);

    // 200 → nearest beat 250.
    session.publish_cmd(loop_out_cmd(200)).expect("loop out");
    let loop_out = recv_updated(&mut evt);
    let region = loop_out.active_loop.expect("active");
    assert_eq!(region.in_ms, 50);
    assert_eq!(region.out_ms, 250);
    assert!(loop_out.pending_loop_in_ms.is_none());
}

#[test]
fn loop_in_without_bpm_publishes_error_when_quantize_on() {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    TrackId::new("no-bpm.wav"),
                    short_tone_fixture(),
                    TrackMetadata::default(),
                )),
            )?;
            Ok(())
        })
        .expect("load");
    let mut evt = session.subscribe_evt_all().expect("sub");

    session.publish_cmd(loop_in_cmd(100)).expect("loop in");
    let error = recv_error(&mut evt);
    assert!(
        error.message.contains("Track BPM is required for loop in"),
        "unexpected: {}",
        error.message
    );
}

#[test]
fn loop_in_without_bpm_ok_when_quantize_off() {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(
                0,
                AudioSource::File(FileAudioSource::new(
                    TrackId::new("no-bpm-exact.wav"),
                    short_tone_fixture(),
                    TrackMetadata::default(),
                )),
            )?;
            Ok(())
        })
        .expect("load");
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize off");
    let _ = recv_updated(&mut evt);

    session.publish_cmd(loop_in_cmd(123)).expect("loop in");
    let loop_in = recv_updated(&mut evt);
    assert_eq!(loop_in.pending_loop_in_ms, Some(123));
}

#[test]
fn loop_in_clamps_position_to_media_range() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetQuantize(SetQuantize {
            deck: 0,
            enabled: false,
        }))
        .expect("quantize off");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(loop_in_cmd(1_000_000))
        .expect("loop in");
    let loop_in = recv_updated(&mut evt);
    let duration = loop_in.duration_ms.expect("duration");
    // Pin the fixture length instead of comparing two fields of the same event
    // (a bug that truncated both to the same wrong value would have passed).
    assert_eq!(duration, SHORT_TONE_LEN_MS, "fixture length");
    assert_eq!(
        loop_in.pending_loop_in_ms,
        Some(SHORT_TONE_LEN_MS),
        "loop-in past the media end clamps to the fixture end"
    );
}

#[test]
fn auto_loop_rejects_zero_and_non_finite_bpm_metadata() {
    for (id, bpm) in [("zero-bpm.wav", Some(0.0)), ("nan-bpm.wav", Some(f64::NAN))] {
        let config = EngineConfig {
            backend: "null".to_string(),
            ..Default::default()
        };
        let session = EngineSession::new(config).expect("session");
        session.with_engine(|engine| engine.start()).expect("start");
        session
            .with_engine(|engine| {
                engine.load_track(
                    0,
                    AudioSource::File(FileAudioSource::new(
                        TrackId::new(id),
                        short_tone_fixture(),
                        TrackMetadata {
                            bpm,
                            ..Default::default()
                        },
                    )),
                )?;
                Ok(())
            })
            .expect("load");
        let mut evt = session.subscribe_evt_all().expect("sub");

        session
            .publish_cmd(Event::SetAutoLoop(SetAutoLoop {
                deck: 0,
                beats: 4.0,
            }))
            .expect("auto loop");
        let error = recv_error(&mut evt);
        assert!(
            error
                .message
                .contains("Track BPM is required for auto loop"),
            "unexpected for {id}: {}",
            error.message
        );
    }
}

#[test]
fn unload_clears_duration() {
    let session = null_session_loaded();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::Unload(Unload { deck: 0 }))
        .expect("unload");
    let update = recv_updated(&mut evt);
    assert!(update.duration_ms.is_none());
    assert_eq!(
        update.cue_point_ms, None,
        "unload must clear the cue point along with the track"
    );
    assert!(update.active_loop.is_none());
    assert!(update.pending_loop_in_ms.is_none());
}
