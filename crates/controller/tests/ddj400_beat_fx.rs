//! Shipped DDJ-400 Beat FX section (channel 5) → engine Beat FX cmds + lamp bytes.
//!
//! Each test drives the same raw MIDI bytes a DDJ-400 sends and asserts the exact
//! published cmds and LED frames, so a wrong note number, a stale snapshot read or
//! an inverted lamp shows up here rather than on hardware.

use std::path::Path;

use controller::{load_bundle, ActionPublish, MappingSession, MidiOut};
use engine_api::{CmdBody, FxKind, FxSlotState, Kind, Origin};

/// Beat FX section status bytes: note-on, note-off, CC (channel 5).
const BEATFX: u8 = 0x94;
const BEATFX_OFF: u8 = 0x84;
const BEATFX_CC: u8 = 0xB4;

/// Beat FX notes (device.toml `[master]`).
const ON_OFF: u8 = 0x47;
const ON_OFF_SHIFT: u8 = 0x43;
const SELECT: u8 = 0x63;
const SELECT_SHIFT: u8 = 0x64;
const BEAT_LEFT: u8 = 0x4A;
const BEAT_RIGHT: u8 = 0x4B;
const CH_1: u8 = 0x10;
const CH_2: u8 = 0x11;
const CH_MASTER: u8 = 0x14;

/// LEVEL/DEPTH knob MSB (the mapping uses the 7-bit byte, like Mixxx).
const LEVEL: u8 = 0x02;
/// The knob also sends an LSB; it is deliberately not declared, so it must be inert.
const LEVEL_UNUSED_LSB: u8 = 0x22;

struct CaptureBus {
    cmds: Vec<(Origin, Kind, CmdBody)>,
}

impl ActionPublish for CaptureBus {
    fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        self.cmds.push((origin, kind, body));
    }
    fn publish_library(
        &mut self,
        _origin: library_api::Origin,
        _kind: library_api::Kind,
        _body: library_api::EvtBody,
    ) {
    }
}

struct CaptureMidi {
    frames: Vec<Vec<u8>>,
}

impl MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

/// Session as the host leaves it right after attach, with the LED cache warm so
/// later assertions see only real transitions.
fn attached() -> (MappingSession, CaptureBus, CaptureMidi) {
    let bundle = load_bundle(Path::new("../../mappings/ddj-400")).unwrap();
    let mut session = MappingSession::from_bundle(bundle).unwrap();
    let mut bus = CaptureBus { cmds: vec![] };
    let mut midi = CaptureMidi { frames: vec![] };
    session.on_init(&mut bus, &mut midi).unwrap();
    bus.cmds.clear();
    midi.frames.clear();
    (session, bus, midi)
}

/// Frames sorted, so assertions do not depend on section iteration order.
fn sorted(midi: &CaptureMidi) -> Vec<[u8; 3]> {
    let mut out: Vec<[u8; 3]> = midi
        .frames
        .iter()
        .filter(|f| f.len() == 3)
        .map(|f| [f[0], f[1], f[2]])
        .collect();
    out.sort_unstable();
    out
}

fn sorted_of(frames: &[[u8; 3]]) -> Vec<[u8; 3]> {
    let mut out = frames.to_vec();
    out.sort_unstable();
    out
}

fn press(session: &mut MappingSession, bus: &mut CaptureBus, midi: &mut CaptureMidi, note: u8) {
    session.handle_midi(&[BEATFX, note, 0x7F], bus, midi);
}

fn release(session: &mut MappingSession, bus: &mut CaptureBus, midi: &mut CaptureMidi, note: u8) {
    session.handle_midi(&[BEATFX_OFF, note, 0x00], bus, midi);
}

fn boot_fx() -> FxSlotState {
    FxSlotState {
        kind: FxKind::Echo,
        enabled: false,
        wet: 0.5,
        params: [0.5, 0.5, 0.0],
    }
}

/// The command the ON/OFF button publishes.
fn slot_enabled(enabled: bool) -> (Origin, Kind, CmdBody) {
    (
        Origin::Mixer,
        Kind::SetFxUnitSlotEnabled,
        CmdBody::SetFxUnitSlotEnabled { slot: 0, enabled },
    )
}

/// The command FX SELECT publishes.
fn slot_kind(kind: FxKind) -> (Origin, Kind, CmdBody) {
    (
        Origin::Mixer,
        Kind::SetFxUnitSlot,
        CmdBody::SetFxUnitSlot { slot: 0, kind },
    )
}

#[test]
fn on_off_toggles_the_shared_unit_and_lights_both_lamps() {
    let (mut s, mut bus, mut midi) = attached();

    press(&mut s, &mut bus, &mut midi, ON_OFF);
    assert_eq!(bus.cmds, vec![slot_enabled(true)]);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[[BEATFX, ON_OFF, 0x7F], [BEATFX, ON_OFF_SHIFT, 0x7F]]),
        "both ON/OFF lamp banks mirror the armed state"
    );

    // Release publishes nothing, then the next press toggles back off.
    bus.cmds.clear();
    midi.frames.clear();
    release(&mut s, &mut bus, &mut midi, ON_OFF);
    assert!(bus.cmds.is_empty(), "releasing ON/OFF must not publish");

    press(&mut s, &mut bus, &mut midi, ON_OFF);
    assert_eq!(bus.cmds, vec![slot_enabled(false)]);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[[BEATFX, ON_OFF, 0x00], [BEATFX, ON_OFF_SHIFT, 0x00]])
    );
}

#[test]
fn shift_on_off_disarms_without_touching_wet() {
    let (mut s, mut bus, mut midi) = attached();

    press(&mut s, &mut bus, &mut midi, ON_OFF);
    bus.cmds.clear();
    midi.frames.clear();

    press(&mut s, &mut bus, &mut midi, ON_OFF_SHIFT);
    assert_eq!(
        bus.cmds,
        vec![slot_enabled(false)],
        "SHIFT + ON/OFF is disable-only"
    );
    assert_eq!(
        sorted(&midi),
        sorted_of(&[[BEATFX, ON_OFF, 0x00], [BEATFX, ON_OFF_SHIFT, 0x00]])
    );
}

#[test]
fn fx_select_cycles_echo_and_reverb() {
    let (mut s, mut bus, mut midi) = attached();

    press(&mut s, &mut bus, &mut midi, SELECT);
    assert_eq!(
        bus.cmds,
        vec![slot_kind(FxKind::Reverb)],
        "the unit boots on Echo, so the first press selects Reverb"
    );

    bus.cmds.clear();
    release(&mut s, &mut bus, &mut midi, SELECT);
    press(&mut s, &mut bus, &mut midi, SELECT);
    assert_eq!(
        bus.cmds,
        vec![slot_kind(FxKind::Echo)],
        "the cycle has no Off entry"
    );

    // SHIFT walks the cycle backwards.
    bus.cmds.clear();
    release(&mut s, &mut bus, &mut midi, SELECT);
    press(&mut s, &mut bus, &mut midi, SELECT_SHIFT);
    assert_eq!(bus.cmds, vec![slot_kind(FxKind::Reverb)]);
}

#[test]
fn fx_select_keeps_the_unit_armed() {
    let (mut s, mut bus, mut midi) = attached();

    press(&mut s, &mut bus, &mut midi, ON_OFF);
    bus.cmds.clear();
    midi.frames.clear();

    // `FxSlot::set_kind` reloads the knobs but keeps the slot armed, so the
    // mirror must not darken the lamps (they would flicker until the echo).
    press(&mut s, &mut bus, &mut midi, SELECT);
    assert_eq!(bus.cmds, vec![slot_kind(FxKind::Reverb)]);
    assert!(
        sorted(&midi).is_empty(),
        "selecting an effect must not touch the armed lamps: {:?}",
        sorted(&midi)
    );
}

#[test]
fn beat_buttons_step_the_echo_division_and_clamp() {
    let (mut s, mut bus, mut midi) = attached();
    let last = 6.0f32;

    // Boot division is 1/2 beat (norm 0.5 = index 3 of 7).
    press(&mut s, &mut bus, &mut midi, BEAT_RIGHT);
    assert_eq!(
        bus.cmds,
        vec![(
            Origin::Mixer,
            Kind::SetFxUnitParam,
            CmdBody::SetFxUnitParam {
                slot: 0,
                param: 0,
                value: 4.0 / last,
            },
        )]
    );

    // Three more steps reach the longest division…
    let mut last_value = 4.0 / last;
    for expected in [5.0 / last, 1.0] {
        bus.cmds.clear();
        release(&mut s, &mut bus, &mut midi, BEAT_RIGHT);
        press(&mut s, &mut bus, &mut midi, BEAT_RIGHT);
        let [(_, _, CmdBody::SetFxUnitParam { value, .. })] = bus.cmds[..] else {
            panic!("expected one SetFxUnitParam, got {:?}", bus.cmds);
        };
        assert!((value - expected).abs() < 1e-6, "value {value}");
        last_value = value;
    }
    assert!((last_value - 1.0).abs() < 1e-6);

    // …and the next press is swallowed at the top of the table.
    bus.cmds.clear();
    release(&mut s, &mut bus, &mut midi, BEAT_RIGHT);
    press(&mut s, &mut bus, &mut midi, BEAT_RIGHT);
    assert!(bus.cmds.is_empty(), "clamped at the longest division");

    // BEAT only edits Echo: after selecting Reverb the buttons are inert.
    press(&mut s, &mut bus, &mut midi, SELECT);
    bus.cmds.clear();
    release(&mut s, &mut bus, &mut midi, BEAT_RIGHT);
    press(&mut s, &mut bus, &mut midi, BEAT_RIGHT);
    assert!(bus.cmds.is_empty(), "division has no meaning for Reverb");
}

#[test]
fn beat_left_clamps_at_the_shortest_division() {
    let (mut s, mut bus, mut midi) = attached();

    // 0.5 → 2/6 → 1/6 → 0.0 then clamped.
    for expected in [2.0 / 6.0, 1.0 / 6.0, 0.0] {
        bus.cmds.clear();
        press(&mut s, &mut bus, &mut midi, BEAT_LEFT);
        let [(_, _, CmdBody::SetFxUnitParam { value, .. })] = bus.cmds[..] else {
            panic!("expected one SetFxUnitParam, got {:?}", bus.cmds);
        };
        assert!((value - expected).abs() < 1e-6, "value {value}");
        release(&mut s, &mut bus, &mut midi, BEAT_LEFT);
    }

    bus.cmds.clear();
    press(&mut s, &mut bus, &mut midi, BEAT_LEFT);
    assert!(bus.cmds.is_empty(), "clamped at the shortest division");
}

#[test]
fn level_knob_sets_wet_and_drops_the_unchanged_position_dump() {
    let (mut s, mut bus, mut midi) = attached();

    // One 7-bit CC is a complete move; the knob applies immediately (no catch-up).
    s.handle_midi(&[BEATFX_CC, LEVEL, 0x40], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].0, Origin::Mixer);
    assert_eq!(bus.cmds[0].1, Kind::SetFxUnitWet);
    let CmdBody::SetFxUnitWet {
        slot,
        wet,
        soft_takeover,
    } = bus.cmds[0].2
    else {
        panic!("expected SetFxUnitWet, got {:?}", bus.cmds[0].2);
    };
    assert_eq!(slot, 0);
    assert!(
        !soft_takeover,
        "the knob is authoritative: it must hard-set, not wait to cross the UI value"
    );
    assert!((wet - 0x40 as f32 / 127.0).abs() < 1e-5, "wet {wet}");

    // The unit re-dumps this exact position on every idle heartbeat; re-publishing
    // it would revert a GUI slider change, so the duplicate is dropped.
    bus.cmds.clear();
    s.age_cc_coalesce_for_test("master.beat_fx_level_depth");
    s.handle_midi(&[BEATFX_CC, LEVEL, 0x40], &mut bus, &mut midi);
    assert!(
        bus.cmds.is_empty(),
        "an unchanged absolute position must not publish"
    );

    // A real turn still publishes (the drop above still restarts the coalesce
    // window, exactly like a real publish, so age it before the next move).
    s.age_cc_coalesce_for_test("master.beat_fx_level_depth");
    s.handle_midi(&[BEATFX_CC, LEVEL, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    let CmdBody::SetFxUnitWet { wet, .. } = bus.cmds[0].2 else {
        panic!("expected SetFxUnitWet, got {:?}", bus.cmds[0].2);
    };
    assert!((wet - 1.0).abs() < 1e-6, "wet {wet}");

    // The unused LSB (0x22) is not declared by the mapping and must stay inert.
    bus.cmds.clear();
    s.age_cc_coalesce_for_test("master.beat_fx_level_depth");
    s.handle_midi(&[BEATFX_CC, LEVEL_UNUSED_LSB, 0x00], &mut bus, &mut midi);
    assert!(bus.cmds.is_empty(), "the unused LSB must stay inert");
}

#[test]
fn ch_select_publishes_the_exclusive_routing_pair() {
    let (mut s, mut bus, mut midi) = attached();
    let routed = |deck: u16, on: bool| {
        (
            Origin::Deck(deck),
            Kind::SetDeckFxRouted,
            CmdBody::SetDeckFxRouted { routed: on },
        )
    };

    press(&mut s, &mut bus, &mut midi, CH_1);
    assert_eq!(bus.cmds, vec![routed(0, true), routed(1, false)]);

    bus.cmds.clear();
    release(&mut s, &mut bus, &mut midi, CH_1);
    press(&mut s, &mut bus, &mut midi, CH_2);
    assert_eq!(bus.cmds, vec![routed(0, false), routed(1, true)]);

    bus.cmds.clear();
    release(&mut s, &mut bus, &mut midi, CH_2);
    press(&mut s, &mut bus, &mut midi, CH_MASTER);
    assert_eq!(
        bus.cmds,
        vec![routed(0, true), routed(1, true)],
        "no master FX insert: both decks routed is the master analog"
    );

    // Releasing a CH SELECT button is not a routing change.
    bus.cmds.clear();
    release(&mut s, &mut bus, &mut midi, CH_MASTER);
    assert!(bus.cmds.is_empty());
}

#[test]
fn set_fx_state_drives_the_on_off_lamps() {
    let (mut s, mut _bus, mut midi) = attached();

    s.set_fx_state(
        &FxSlotState {
            enabled: true,
            ..boot_fx()
        },
        &mut midi,
    );
    assert_eq!(
        sorted(&midi),
        sorted_of(&[[BEATFX, ON_OFF, 0x7F], [BEATFX, ON_OFF_SHIFT, 0x7F]])
    );

    midi.frames.clear();
    s.set_fx_state(&boot_fx(), &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[[BEATFX, ON_OFF, 0x00], [BEATFX, ON_OFF_SHIFT, 0x00]])
    );

    // An unchanged state must not re-send (attach replay runs on every reconnect).
    midi.frames.clear();
    s.set_fx_state(&boot_fx(), &mut midi);
    assert!(midi.frames.is_empty());
}
