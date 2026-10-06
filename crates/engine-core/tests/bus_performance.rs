//! Integration: performance cmds (cue / loop / beat jump / unload) on the bus.

mod common;

use common::{recv_evt_kind, short_tone_fixture, source_with_bpm, SHORT_TONE_LEN_MS};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use library_core::{AudioSource, FileAudioSource, TrackId, TrackMetadata};
use omnibus::Filter;

fn encode_loop_in(position_ms: i32) -> Vec<u8> {
    encode_cmd_body(&CmdBody::LoopIn { position_ms }).unwrap()
}

fn encode_loop_out(position_ms: i32) -> Vec<u8> {
    encode_cmd_body(&CmdBody::LoopOut { position_ms }).unwrap()
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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetAutoLoop,
            encode_cmd_body(&CmdBody::SetAutoLoop { beats: 4.0 }).unwrap(),
        )
        .expect("auto loop");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let region = active_loop.expect("loop region");
    assert!(region.active);
    assert!(region.out_ms > region.in_ms);
}

#[test]
fn set_auto_loop_snaps_in_to_nearest_beat_even_when_quantize_off() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize off");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    // Fixture is 250 ms; 120 BPM → 500 ms/beat. 100 ms snaps down to 0.
    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::Seek,
            encode_cmd_body(&CmdBody::Seek { position_ms: 100 }).unwrap(),
        )
        .expect("seek");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetAutoLoop,
            encode_cmd_body(&CmdBody::SetAutoLoop { beats: 1.0 }).unwrap(),
        )
        .expect("auto loop");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { active_loop, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let region = active_loop.expect("loop region");
    assert_eq!(region.in_ms, 0);
    assert!(region.out_ms > region.in_ms);
}

#[test]
fn set_quantize_and_cue_point_roundtrip() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize");
    let q = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { quantize, .. } = decode_evt_body(q.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(!quantize);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetCuePoint,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("cue");
    let cue = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { cue_point_ms, .. } = decode_evt_body(cue.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(cue_point_ms.is_some());

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(0))
        .expect("loop in");
    let loop_in = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        active_loop,
        pending_loop_in_ms,
        ..
    } = decode_evt_body(loop_in.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(active_loop.is_none());
    assert!(pending_loop_in_ms.is_some());

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::BeatJump,
            encode_cmd_body(&CmdBody::BeatJump { beats: 1.0 }).unwrap(),
        )
        .expect("jump");
    let jumped = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated { position_ms, .. } =
        decode_evt_body(jumped.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated after beat jump");
    };
    // Quantize is off, so no snapping: 1 beat at 120 BPM = 60/120 s = 500 ms,
    // jumped forward from position 0. (A seek is not clamped to the 0.25 s
    // fixture length — see `DspDeck::seek_ms_inner`.)
    assert_eq!(position_ms, Some(500));
}

#[test]
fn loop_in_preserves_exact_position_when_quantize_off() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize off");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(123))
        .expect("loop in");
    let loop_in = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        active_loop,
        pending_loop_in_ms,
        ..
    } = decode_evt_body(loop_in.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(active_loop.is_none());
    assert_eq!(pending_loop_in_ms, Some(123));
}

#[test]
fn loop_in_snaps_when_quantize_on() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    // 120 BPM → 500 ms/beat; 200 ms → nearest beat 0 (fixture is 250 ms).
    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(200))
        .expect("loop in");
    let loop_in = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        pending_loop_in_ms, ..
    } = decode_evt_body(loop_in.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pending_loop_in_ms, Some(0));
}

#[test]
fn loop_out_completes_pending_and_activates() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize off");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(40))
        .expect("loop in");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopOut, encode_loop_out(200))
        .expect("loop out");
    let loop_out = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        active_loop,
        pending_loop_in_ms,
        ..
    } = decode_evt_body(loop_out.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let region = active_loop.expect("active after out");
    assert!(region.active);
    assert_eq!(region.in_ms, 40);
    assert_eq!(region.out_ms, 200);
    assert!(pending_loop_in_ms.is_none());
}

#[test]
fn loop_out_without_pending_or_active_errors() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopOut, encode_loop_out(1000))
        .expect("loop out cmd");
    let event = recv_evt_kind(&evt, Kind::Error);
    let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("decode") else {
        panic!("expected Error");
    };
    assert!(
        message.contains("Set Loop In before Loop Out"),
        "unexpected: {message}"
    );
}

#[test]
fn exit_loop_clears_pending() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(40))
        .expect("loop in");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::ExitLoop,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("exit");
    let exit = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        active_loop,
        pending_loop_in_ms,
        ..
    } = decode_evt_body(exit.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(active_loop.is_none());
    assert!(pending_loop_in_ms.is_none());
}

#[test]
fn pending_loop_in_survives_quantize_toggle() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(200))
        .expect("loop in");
    let pending = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        pending_loop_in_ms, ..
    } = decode_evt_body(pending.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pending_loop_in_ms, Some(0));

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize off");
    let after = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        pending_loop_in_ms, ..
    } = decode_evt_body(after.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pending_loop_in_ms, Some(0));
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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize off");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(50))
        .expect("loop in");
    let pending = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        pending_loop_in_ms, ..
    } = decode_evt_body(pending.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pending_loop_in_ms, Some(50));

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: true }).unwrap(),
        )
        .expect("quantize on");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    // 200 → nearest beat 250.
    session
        .publish_cmd(Origin::Deck(0), Kind::LoopOut, encode_loop_out(200))
        .expect("loop out");
    let loop_out = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        active_loop,
        pending_loop_in_ms,
        ..
    } = decode_evt_body(loop_out.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let region = active_loop.expect("active");
    assert_eq!(region.in_ms, 50);
    assert_eq!(region.out_ms, 250);
    assert!(pending_loop_in_ms.is_none());
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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(100))
        .expect("loop in");
    let event = recv_evt_kind(&evt, Kind::Error);
    let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("decode") else {
        panic!("expected Error");
    };
    assert!(
        message.contains("Track BPM is required for loop in"),
        "unexpected: {message}"
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
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize off");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(123))
        .expect("loop in");
    let loop_in = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        pending_loop_in_ms, ..
    } = decode_evt_body(loop_in.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(pending_loop_in_ms, Some(123));
}

#[test]
fn loop_in_clamps_position_to_media_range() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetQuantize,
            encode_cmd_body(&CmdBody::SetQuantize { enabled: false }).unwrap(),
        )
        .expect("quantize off");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(Origin::Deck(0), Kind::LoopIn, encode_loop_in(1_000_000))
        .expect("loop in");
    let loop_in = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        pending_loop_in_ms,
        duration_ms,
        ..
    } = decode_evt_body(loop_in.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    let duration = duration_ms.expect("duration");
    // Pin the fixture length instead of comparing two fields of the same event
    // (a bug that truncated both to the same wrong value would have passed).
    assert_eq!(duration, SHORT_TONE_LEN_MS, "fixture length");
    assert_eq!(
        pending_loop_in_ms,
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
        let evt = session
            .evt_bus()
            .subscribe(Filter::Any, Filter::Any)
            .expect("sub");

        session
            .publish_cmd(
                Origin::Deck(0),
                Kind::SetAutoLoop,
                encode_cmd_body(&CmdBody::SetAutoLoop { beats: 4.0 }).unwrap(),
            )
            .expect("auto loop");
        let event = recv_evt_kind(&evt, Kind::Error);
        let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("decode") else {
            panic!("expected Error for {id}");
        };
        assert!(
            message.contains("Track BPM is required for auto loop"),
            "unexpected for {id}: {message}"
        );
    }
}

#[test]
fn unload_clears_duration() {
    let session = null_session_loaded();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::Unload,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("unload");
    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        duration_ms,
        cue_point_ms,
        active_loop,
        pending_loop_in_ms,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(duration_ms.is_none());
    assert_eq!(
        cue_point_ms, None,
        "unload must clear the cue point along with the track"
    );
    assert!(active_loop.is_none());
    assert!(pending_loop_in_ms.is_none());
}
