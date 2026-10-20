//! Integration: FX unit cmds (slot kind/enable/wet/param, deck routing) → Updated evts.

mod common;

use common::{recv_evt_kind, recv_evt_where, TestReceiver};
use engine_api::{
    decode_evt_body, encode_cmd_body, CmdBody, EvtBody, FxKind, FxSlotState, Kind, Origin,
    FX_SLOT_COUNT,
};
use engine_core::{EngineConfig, EngineSession};
use omnibus::Filter;

fn null_session() -> EngineSession {
    let config = EngineConfig {
        backend: "null".to_string(),
        ..Default::default()
    };
    let session = EngineSession::new(config).expect("session");
    session.with_engine(|engine| engine.start()).expect("start");
    session
}

fn deck_updated(evt: &TestReceiver, deck_id: u16) -> EvtBody {
    let event = recv_evt_where(evt, Kind::Updated, |event| {
        matches!(
            decode_evt_body(event.payload()),
            Ok(EvtBody::DeckUpdated { id, .. }) if id == deck_id
        )
    });
    decode_evt_body(event.payload()).expect("decode evt body")
}

/// FX slots carried by the next `DeckUpdated` for `deck_id`.
fn deck_fx(evt: &TestReceiver, deck_id: u16) -> [FxSlotState; FX_SLOT_COUNT] {
    match deck_updated(evt, deck_id) {
        EvtBody::DeckUpdated { fx, .. } => fx,
        other => panic!("expected DeckUpdated, got {other:?}"),
    }
}

/// Assert the next evt is an `Error` whose message contains `needle`.
fn expect_error_contains(evt: &TestReceiver, needle: &str) {
    let event = recv_evt_kind(evt, Kind::Error);
    let EvtBody::Error { message } = decode_evt_body(event.payload()).expect("decode") else {
        panic!("expected Error");
    };
    assert!(message.contains(needle), "unexpected message: {message}");
}

fn publish(session: &EngineSession, origin: Origin, kind: Kind, body: &CmdBody) {
    session
        .publish_cmd(origin, kind, encode_cmd_body(body).unwrap())
        .expect("publish");
}

#[test]
fn set_fx_unit_slot_publishes_the_kind_for_every_deck() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    // Reverb differs from the slot's loaded default (Echo), so this proves the
    // command reached the unit instead of restating the initial state.
    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitSlot,
        &CmdBody::SetFxUnitSlot {
            slot: 0,
            kind: FxKind::Reverb,
        },
    );

    // The unit is shared: every deck republishes the same slot state.
    for deck_id in [0u16, 1u16] {
        let fx = deck_fx(&evt, deck_id);
        assert_eq!(fx.len(), FX_SLOT_COUNT);
        assert_eq!(fx[0].kind, FxKind::Reverb, "deck {deck_id} slot 0");
        assert!(!fx[0].enabled, "selecting an effect leaves the slot off");
        assert!((fx[0].wet - 0.5).abs() < 1e-6, "default wet: {}", fx[0].wet);
        // The newly loaded effect's knobs reset to its defaults.
        assert!((fx[0].params[0] - 0.5).abs() < 1e-6, "{}", fx[0].params[0]);
        assert_eq!(fx[1].kind, FxKind::Off);
        assert_eq!(fx[2].kind, FxKind::Off);
    }
}

#[test]
fn fresh_unit_carries_echo_unarmed() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    // A deck-scoped command republishes that deck without touching the FX config,
    // which is the state the bar renders before its first interaction.
    publish(
        &session,
        Origin::Deck(0),
        Kind::SetDeckFxRouted,
        &CmdBody::SetDeckFxRouted { routed: false },
    );

    let fx = deck_fx(&evt, 0);
    assert_eq!(fx[0].kind, FxKind::Echo, "the FX select has no off entry");
    assert!(!fx[0].enabled, "a fresh unit is loaded but not armed");
    // Echo's default TIME is 1/2 beat (norm 0.5 of the beat-division table).
    assert!((fx[0].params[0] - 0.5).abs() < 1e-6, "{}", fx[0].params[0]);
    assert_eq!(fx[1].kind, FxKind::Off);
    assert_eq!(fx[2].kind, FxKind::Off);
}

#[test]
fn set_fx_unit_wet_and_param_reach_every_deck() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitSlot,
        &CmdBody::SetFxUnitSlot {
            slot: 1,
            kind: FxKind::Reverb,
        },
    );
    let _ = deck_updated(&evt, 0);
    let _ = deck_updated(&evt, 1);

    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitWet,
        &CmdBody::SetFxUnitWet {
            slot: 1,
            wet: 0.75,
            soft_takeover: false,
        },
    );
    let _ = deck_updated(&evt, 0);
    let _ = deck_updated(&evt, 1);

    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitParam,
        &CmdBody::SetFxUnitParam {
            slot: 1,
            param: 0,
            value: 0.9,
        },
    );

    for deck_id in [0u16, 1u16] {
        let fx = deck_fx(&evt, deck_id);
        assert_eq!(fx[1].kind, FxKind::Reverb);
        assert!((fx[1].wet - 0.75).abs() < 1e-6, "wet {}", fx[1].wet);
        assert!(
            (fx[1].params[0] - 0.9).abs() < 1e-6,
            "size {}",
            fx[1].params[0]
        );
    }
}

#[test]
fn set_deck_fx_routed_publishes_deck_only() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Origin::Deck(1),
        Kind::SetDeckFxRouted,
        &CmdBody::SetDeckFxRouted { routed: true },
    );

    // Routing is deck-scoped: the first Updated evt is deck 1, not a fan-out.
    let event = recv_evt_kind(&evt, Kind::Updated);
    assert_eq!(*event.origin(), Origin::Deck(1));
    let EvtBody::DeckUpdated { id, fx_routed, .. } =
        decode_evt_body(event.payload()).expect("decode")
    else {
        panic!("expected DeckUpdated");
    };
    assert_eq!(id, 1);
    assert!(fx_routed, "deck 1 is routed through the FX unit");

    // Deck 0 keeps its own routing state.
    publish(
        &session,
        Origin::Deck(0),
        Kind::SetDeckFxRouted,
        &CmdBody::SetDeckFxRouted { routed: false },
    );
    let EvtBody::DeckUpdated { fx_routed, .. } = deck_updated(&evt, 0) else {
        panic!("expected DeckUpdated for deck 0");
    };
    assert!(!fx_routed, "deck 0 routing is independent");
}

#[test]
fn set_fx_unit_param_rejects_out_of_range() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    // Slot 0's effect exposes two knobs, so index 2 is out of range.
    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitParam,
        &CmdBody::SetFxUnitParam {
            slot: 0,
            param: 2,
            value: 0.5,
        },
    );

    expect_error_contains(&evt, "Invalid FX parameter: 2");
}

#[test]
fn set_fx_unit_slot_rejects_out_of_range_slot() {
    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");

    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitSlotEnabled,
        &CmdBody::SetFxUnitSlotEnabled {
            slot: 3,
            enabled: true,
        },
    );

    expect_error_contains(&evt, "Invalid FX slot: 3");
}

#[test]
fn set_fx_unit_wet_soft_takeover_blocks_until_near() {
    use std::time::{Duration, Instant};

    let session = null_session();
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    let wet = |session: &EngineSession| {
        session
            .with_engine(|engine| Ok(engine.fx_unit_wet(0)))
            .expect("engine")
            .expect("slot 0")
    };

    // Never-seen latch: the first soft-takeover value is adopted (controller dumps).
    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitWet,
        &CmdBody::SetFxUnitWet {
            slot: 0,
            wet: 0.9,
            soft_takeover: true,
        },
    );
    let _ = deck_updated(&evt, 0);
    let _ = deck_updated(&evt, 1);
    assert!(
        (wet(&session) - 0.9).abs() < 1e-6,
        "adopted {}",
        wet(&session)
    );

    // A host hard set clears the latch and moves the unit.
    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitWet,
        &CmdBody::SetFxUnitWet {
            slot: 0,
            wet: 0.0,
            soft_takeover: false,
        },
    );
    let _ = deck_updated(&evt, 0);
    let _ = deck_updated(&evt, 1);
    assert!(wet(&session).abs() < 1e-6, "hard set {}", wet(&session));

    // A far-away knob move is swallowed: no DeckUpdated and no state change.
    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitWet,
        &CmdBody::SetFxUnitWet {
            slot: 0,
            wet: 0.9,
            soft_takeover: true,
        },
    );
    let deadline = Instant::now() + Duration::from_millis(150);
    while Instant::now() < deadline {
        if let Some(event) = evt.recv_timeout(Duration::from_millis(50)).expect("recv") {
            assert_ne!(*event.kind(), Kind::Updated, "blocked wet must stay silent");
        }
    }
    assert!(
        wet(&session).abs() < 1e-6,
        "blocked wet moved the unit: {}",
        wet(&session)
    );

    // Within the catch threshold the knob takes over and latches.
    publish(
        &session,
        Origin::Mixer,
        Kind::SetFxUnitWet,
        &CmdBody::SetFxUnitWet {
            slot: 0,
            wet: 0.02,
            soft_takeover: true,
        },
    );
    let fx = deck_fx(&evt, 0);
    assert!((fx[0].wet - 0.02).abs() < 1e-6, "wet {}", fx[0].wet);
}
