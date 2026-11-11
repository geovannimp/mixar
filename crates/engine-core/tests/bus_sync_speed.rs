//! Integration: sync/master + speed follow on the cmd/evt bus.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
mod common;

use bus_api::Event;
use bus_api::SetMasterDeck;
use bus_api::SetSpeed;
use bus_api::SetTempoRange;
use bus_api::SyncMode;
use bus_api::ToggleSync;
use common::recv_snapshot;
use common::recv_updated;
use common::recv_updated_where;
use common::source_with_bpm;
use engine_core::EngineConfig;
use engine_core::EngineSession;

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
    // Master 120 BPM at unity → slave 100 BPM mirrors 120 BPM (1.2×). The
    // default ±6% fader cannot express that, so `speed` saturates; the actual
    // playback ratio must still track the master.
    assert!(
        (update.playback_ratio - 1.2).abs() < 1e-3,
        "slave should mirror the master tempo, ratio={}",
        update.playback_ratio
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
            // Slower than unity: norm 0.8 with the default ±6% → ratio
            // 1 + (0.5 − 0.8)·0.12 = 0.964.
            speed: 0.8,
            soft_takeover: false,
        }))
        .expect("speed");

    // Master update first, then slave follow. Filter on the slave's deck id via
    // the shared helper so this wait reports timeouts/disconnects identically to
    // every other receive in the suite.
    let update = recv_updated_where(&mut evt, |update| update.deck == 1);
    // Slave target 0.964 × 120/100 = 1.1568, well outside the default ±6%
    // fader. Asserting the actual ratio means deleting `apply_tempo_sync`
    // fails this test even though the fader saturates.
    let ratio = update.playback_ratio;
    assert!(
        (ratio - 1.1568).abs() < 1e-3,
        "slave should follow the master's new tempo, slave_ratio={ratio}"
    );
}

#[test]
fn tempo_sync_stays_locked_across_tempo_range_changes() {
    let session = null_session_with_synced_decks();
    let mut evt = session.subscribe_evt_all().expect("sub");

    session
        .publish_cmd(Event::ToggleSync(ToggleSync {
            deck: 1,
            beat_sync: false,
        }))
        .expect("toggle");
    let _ = recv_updated(&mut evt);

    // The slave's own fader span is display-only: it must not change the ratio
    // needed to mirror the master.
    for range in [0.06_f32, 0.10, 0.25] {
        session
            .publish_cmd(Event::SetTempoRange(SetTempoRange {
                deck: 1,
                tempo_range: range,
            }))
            .expect("range");
        let update = recv_updated_where(&mut evt, |update| update.deck == 1);
        assert!(
            (update.playback_ratio - 1.2).abs() < 1e-3,
            "range {range}: slave should still mirror the master, ratio={}",
            update.playback_ratio
        );
    }

    // The master's span does change its effective tempo; the slave follows it.
    session
        .publish_cmd(Event::SetSpeed(SetSpeed {
            deck: 0,
            speed: 0.0,
            soft_takeover: false,
        }))
        .expect("speed");
    let _ = recv_updated_where(&mut evt, |update| update.deck == 0);
    let slave = recv_updated_where(&mut evt, |update| update.deck == 1);
    assert!(
        (slave.playback_ratio - 1.272).abs() < 1e-3,
        "master at ±6% should give 1.06× → slave 1.272×, got {}",
        slave.playback_ratio
    );

    session
        .publish_cmd(Event::SetTempoRange(SetTempoRange {
            deck: 0,
            tempo_range: 0.10,
        }))
        .expect("range");
    let _ = recv_updated_where(&mut evt, |update| update.deck == 0);
    let slave = recv_updated_where(&mut evt, |update| update.deck == 1);
    assert!(
        (slave.playback_ratio - 1.32).abs() < 1e-3,
        "master at ±10% should give 1.10× → slave 1.32×, got {}",
        slave.playback_ratio
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
