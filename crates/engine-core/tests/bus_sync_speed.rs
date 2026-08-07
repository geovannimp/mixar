//! Integration: sync/master + speed follow on the cmd/evt bus.

use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin, SyncMode};
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
        let event = sub
            .recv_timeout(remaining.min(Duration::from_millis(50)))
            .expect("recv")
            .expect("event");
        if *event.kind() == kind {
            return (*event).clone();
        }
    }
    panic!("timeout waiting for evt kind {kind:?}");
}

fn short_tone_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/fixtures/short-tone.wav")
}

fn source_with_bpm(id: &str, bpm: f64) -> AudioSource {
    AudioSource::File(FileAudioSource::new(
        TrackId::new(id),
        short_tone_fixture(),
        TrackMetadata {
            bpm: Some(bpm),
            ..Default::default()
        },
    ))
}

fn null_session_with_synced_decks() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
        .with_engine(|engine| {
            engine.load_track(0, source_with_bpm("master.wav", 120.0))?;
            engine.load_track(1, source_with_bpm("slave.wav", 100.0))?;
            Ok(())
        })
        .expect("load");
    session
}

#[test]
fn toggle_sync_publishes_tempo_mode_and_matched_speed() {
    let session = null_session_with_synced_decks();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(1),
            Kind::ToggleSync,
            encode_cmd_body(&CmdBody::ToggleSync { beat_sync: false }).unwrap(),
        )
        .expect("toggle");

    let event = recv_evt_kind(&evt, Kind::Updated);
    let EvtBody::DeckUpdated {
        id,
        sync_mode,
        speed,
        ..
    } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(id, 1);
    assert_eq!(sync_mode, SyncMode::Tempo);
    // Master 120 @ 1.0 → slave 100 needs 1.2×
    assert!((speed - 1.2).abs() < 0.01);
}

#[test]
fn master_speed_change_updates_synced_slave() {
    let session = null_session_with_synced_decks();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(1),
            Kind::ToggleSync,
            encode_cmd_body(&CmdBody::ToggleSync { beat_sync: false }).unwrap(),
        )
        .expect("toggle");
    let _ = recv_evt_kind(&evt, Kind::Updated);

    session
        .publish_cmd(
            Origin::Deck(0),
            Kind::SetSpeed,
            encode_cmd_body(&CmdBody::SetSpeed { speed: 1.05, soft_takeover: false }).unwrap(),
        )
        .expect("speed");

    // Master update first, then slave follow.
    let mut slave_speed = None;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = evt
            .recv_timeout(remaining.min(Duration::from_millis(50)))
            .expect("recv")
            .expect("event");
        if *event.kind() != Kind::Updated {
            continue;
        }
        let EvtBody::DeckUpdated { id, speed, .. } =
            decode_evt_body(event.payload()).expect("decode")
        else {
            continue;
        };
        if id == 1 {
            slave_speed = Some(speed);
            break;
        }
    }
    let slave_speed = slave_speed.expect("slave Updated");
    // Master effective 120 * 1.05 = 126 → slave 100 → 1.26×
    assert!((slave_speed - 1.26).abs() < 0.01);
}

#[test]
fn set_master_deck_publishes_status_with_master_deck() {
    let session = null_session_with_synced_decks();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    session
        .publish_cmd(
            Origin::Deck(1),
            Kind::SetMasterDeck,
            encode_cmd_body(&CmdBody::Empty).unwrap(),
        )
        .expect("master");

    let event = recv_evt_kind(&evt, Kind::Status);
    let EvtBody::EngineStatus { status } = decode_evt_body(event.payload()).expect("decode") else {
        panic!("expected EngineStatus");
    };
    assert_eq!(status.master_deck, 1);
}
