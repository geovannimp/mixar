//! Integration: slip mode publishes state and catches up on loop exit.

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
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
        match sub.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(Some(event)) => {
                if *event.kind() == kind {
                    return (*event).clone();
                }
            }
            Ok(None) => {}
            Err(e) => panic!("recv: {e}"),
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

fn null_session_with_loaded_deck() -> EngineSession {
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
                    TrackId::new("slip.wav"),
                    short_tone_fixture(),
                    TrackMetadata {
                        bpm: Some(120.0),
                        ..Default::default()
                    },
                )),
            )
        })
        .expect("load");
    session
}

#[test]
fn set_slip_publishes_enabled_flag() {
    let session = null_session_with_loaded_deck();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetSlip,
            encode_cmd_body(&CmdBody::SetSlip { enabled: true }).unwrap(),
        )
        .expect("set slip");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        slip_enabled,
        slip_shadow_position_ms,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert!(slip_enabled);
    assert!(slip_shadow_position_ms.is_some());
}

#[test]
fn exit_loop_with_slip_catches_up() {
    let session = null_session_with_loaded_deck();

    session
        .with_engine(|engine| {
            engine.set_deck_slip(0, true)?;
            engine.play(0)?;
            engine.set_deck_auto_loop(0, 0.25)?;
            Ok(())
        })
        .expect("setup");

    std::thread::sleep(Duration::from_millis(400));

    session
        .with_engine(|engine| engine.clear_deck_loop(0))
        .expect("exit loop");

    let (position_ms, shadow_ms) = session
        .with_engine(|engine| {
            Ok((
                engine.deck_playback_ms(0).map(|(p, _)| p),
                engine.deck_slip_shadow_position_ms(0),
            ))
        })
        .expect("readback");

    let pos = position_ms.expect("position");
    let shadow = shadow_ms.expect("shadow");
    assert!(
        (pos - shadow).abs() <= 30,
        "loop exit should catch up: pos={pos} shadow={shadow}"
    );
}
