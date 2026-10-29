//! Integration: sync/master + speed follow on the cmd/evt bus.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use common::recv_snapshot;
use common::recv_updated;
use common::recv_updated_where;
use common::source_with_bpm;
use engine_api::Event;
use engine_api::SetMasterDeck;
use engine_api::SetSpeed;
use engine_api::SyncMode;
use engine_api::ToggleSync;
use engine_core::AudioConfig;
use engine_core::EngineConfig;
use engine_core::EngineSession;

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
            vinyl_brake_pause: None,
            vinyl_brake_stop_secs: None,
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
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::ToggleSync(ToggleSync {
            deck: 1,
            beat_sync: false,
        }))
        .expect("toggle");

    let update = recv_updated(&mut evt);
    assert_eq!(update.deck, 1);
    assert_eq!(update.sync_mode, SyncMode::Tempo);
    // Master 120 BPM at unity → slave 100 BPM needs ratio 1.2×.
    // playback_ratio_to_norm(1.2, ±0.25) = 0.5 − (1.2−1.0)/0.5 = 0.1.
    assert!(
        (update.speed - 0.1).abs() < 1e-3,
        "slave should be synced to the master tempo, speed={}",
        update.speed
    );
}

#[test]
fn master_speed_change_updates_synced_slave() {
    let session = null_session_with_synced_decks();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::ToggleSync(ToggleSync {
            deck: 1,
            beat_sync: false,
        }))
        .expect("toggle");
    let _ = recv_updated(&mut evt);

    session
        .publish_cmd(Event::SetSpeed(SetSpeed {
            deck: 0,
            // Slower than unity: norm 0.8 with ±0.25 → ratio
            // 1 + (0.5 − 0.8)·0.5 = 0.85, so the slave target
            // 0.85 × 120/100 = 1.02 stays inside the fader span.
            speed: 0.8,
            soft_takeover: false,
        }))
        .expect("speed");

    // Master update first, then slave follow. Filter on the slave's deck id via
    // the shared helper so this wait reports timeouts/disconnects identically to
    // every other receive in the suite.
    let update = recv_updated_where(&mut evt, |update| update.deck == 1);
    let speed = update.speed;
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
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::SetMasterDeck(SetMasterDeck { deck: 1 }))
        .expect("master");

    let snapshot = recv_snapshot(&mut evt);
    assert_eq!(snapshot.status.master_deck, 1);
}
