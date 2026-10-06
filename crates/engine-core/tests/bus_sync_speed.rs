//! Integration: sync/master + speed follow on the cmd/evt bus.

mod common;

use common::{recv_evt_kind, recv_evt_where, source_with_bpm};
use engine_api::{decode_evt_body, encode_cmd_body, CmdBody, EvtBody, Kind, Origin, SyncMode};
use engine_core::{AudioConfig, EngineConfig, EngineSession};
use omnibus::Filter;

/// ±25% tempo fader span, wide enough that the 120→100 BPM sync ratio (1.2×)
/// is reachable instead of saturating the default ±6% fader.
const TEST_TEMPO_RANGE: f32 = 0.25;

fn null_session_with_synced_decks() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        audio: Some(AudioConfig {
            resampler_quality: None,
            sampler_strip_route: None,
            default_tempo_range: Some(TEST_TEMPO_RANGE),
            tempo_range_steps: None,
            default_key_lock: None,
        }),
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
    // Master 120 BPM at unity → slave 100 BPM needs ratio 1.2×.
    // playback_ratio_to_norm(1.2, ±0.25) = 0.5 − (1.2−1.0)/0.5 = 0.1.
    assert!(
        (speed - 0.1).abs() < 1e-3,
        "slave should be synced to the master tempo, speed={speed}"
    );
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
            encode_cmd_body(&CmdBody::SetSpeed {
                // Slower than unity: norm 0.8 with ±0.25 → ratio
                // 1 + (0.5 − 0.8)·0.5 = 0.85, so the slave target
                // 0.85 × 120/100 = 1.02 stays inside the fader span.
                speed: 0.8,
                soft_takeover: false,
            })
            .unwrap(),
        )
        .expect("speed");

    // Master update first, then slave follow. Filter on the slave's deck id via
    // the shared helper so this wait reports timeouts/disconnects identically to
    // every other receive in the suite.
    let event = recv_evt_where(&evt, Kind::Updated, |event| {
        matches!(
            decode_evt_body(event.payload()),
            Ok(EvtBody::DeckUpdated { id: 1, .. })
        )
    });
    let EvtBody::DeckUpdated { speed, .. } = decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    // Master ratio 0.85 → slave needs 0.85 × 120/100 = 1.02×.
    // playback_ratio_to_norm(1.02, ±0.25) = 0.5 − 0.02/0.5 = 0.46.
    // Asserting the derived value (not 0.0) means deleting `apply_tempo_sync`
    // now fails this test.
    assert!(
        (speed - 0.46).abs() < 1e-3,
        "slave should follow the master's new tempo, slave_speed={speed}"
    );
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
