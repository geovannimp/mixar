//! Shipped DDJ-400 bundle: Keyboard / Key Shift pad banks, mode buttons, LEDs.

use std::path::Path;

use engine_api::{CmdBody, Kind, Origin, PadMode};

struct CaptureBus {
    cmds: Vec<(Origin, Kind, CmdBody)>,
}

impl controller::ActionPublish for CaptureBus {
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

impl controller::MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

fn session() -> controller::MappingSession {
    let b = controller::load_bundle(Path::new("../../mappings/ddj-400")).unwrap();
    controller::MappingSession::from_bundle(b).unwrap()
}

fn new_bus() -> CaptureBus {
    CaptureBus { cmds: vec![] }
}

fn new_midi() -> CaptureMidi {
    CaptureMidi { frames: vec![] }
}

#[test]
fn keyboard_and_key_shift_banks_dispatch() {
    let mut s = session();
    let mut bus = new_bus();
    let mut midi = new_midi();

    // Keyboard bank press/release: ch8 note 0x40 = keyboard_pad_1.
    s.handle_midi(&[0x97, 0x40, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::KeyboardPadPress);
    assert!(matches!(
        bus.cmds.last().unwrap().2,
        CmdBody::KeyboardPadPress {
            slot: 0,
            shift: false
        }
    ));

    s.handle_midi(&[0x87, 0x40, 0x00], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::KeyboardPadRelease);
    assert!(matches!(
        bus.cmds.last().unwrap().2,
        CmdBody::KeyboardPadRelease { slot: 0 }
    ));

    // Key Shift bank press: ch8 note 0x70 = key_shift_pad_1 (offset 0).
    s.handle_midi(&[0x97, 0x70, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::KeyShiftPadPress);
    assert!(matches!(
        bus.cmds.last().unwrap().2,
        CmdBody::KeyShiftPadPress {
            slot: 0,
            shift: false
        }
    ));

    // Shift bank: keyboard ch9 note 0x40 → KeyboardPadPress { shift: true }.
    s.handle_midi(&[0x98, 0x40, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::KeyboardPadPress);
    assert!(matches!(
        bus.cmds.last().unwrap().2,
        CmdBody::KeyboardPadPress {
            slot: 0,
            shift: true
        }
    ));

    // Shift bank: key-shift ch9 note 0x70 → KeyShiftPadPress { shift: true }.
    s.handle_midi(&[0x98, 0x70, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::KeyShiftPadPress);
    assert!(matches!(
        bus.cmds.last().unwrap().2,
        CmdBody::KeyShiftPadPress {
            slot: 0,
            shift: true
        }
    ));
}

#[test]
fn key_shift_pad_lights_led_and_reset_clears() {
    let mut s = session();
    let mut bus = new_bus();
    let mut midi = new_midi();

    // Enter the Key Shift page so the bank LEDs are driven.
    s.handle_midi(&[0x90, 0x6F, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::SetPadMode);
    midi.frames.clear();

    // Engine echo: +2 semitones → key_shift_pad_3 (index 2) lit, pad 1 dark.
    s.set_deck_key_shift(0, 2.0, &mut midi);
    assert!(
        midi.frames.contains(&vec![0x97, 0x72, 0x7F]),
        "pad 3 LED on: {:?}",
        midi.frames
    );
    assert!(
        midi.frames.contains(&vec![0x97, 0x70, 0x00]),
        "pad 1 LED off: {:?}",
        midi.frames
    );

    midi.frames.clear();
    // Reset → pad 3 dark.
    s.set_deck_key_shift(0, 0.0, &mut midi);
    assert!(
        midi.frames.contains(&vec![0x97, 0x72, 0x00]),
        "pad 3 LED off: {:?}",
        midi.frames
    );
}

#[test]
fn unchanged_mirror_does_not_repaint_pad_leds() {
    let mut s = session();
    let mut bus = new_bus();
    let mut midi = new_midi();

    // Enter Key Shift mode so the bank LEDs are driven.
    s.handle_midi(&[0x90, 0x6F, 0x7F], &mut bus, &mut midi);
    midi.frames.clear();

    // The first mirror of +2 repaints the bank.
    s.set_deck_key_shift(0, 2.0, &mut midi);
    assert!(!midi.frames.is_empty(), "first mirror must paint the bank");
    midi.frames.clear();

    // A repeat mirror of the same key shift must do no MIDI work.
    s.set_deck_key_shift(0, 2.0, &mut midi);
    assert!(
        midi.frames.is_empty(),
        "unchanged key shift repainted: {:?}",
        midi.frames
    );

    // Re-mirroring the same pages (default is 2) must also stay silent.
    s.set_deck_keyboard_page(0, 2, &mut midi);
    s.set_deck_key_shift_page(0, 2, &mut midi);
    assert!(
        midi.frames.is_empty(),
        "unchanged pages repainted: {:?}",
        midi.frames
    );
}

#[test]
fn mode_buttons_set_pad_mode() {
    let mut s = session();
    let mut bus = new_bus();
    let mut midi = new_midi();

    s.handle_midi(&[0x90, 0x69, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::SetPadMode);
    assert!(matches!(
        bus.cmds.last().unwrap().2,
        CmdBody::SetPadMode {
            mode: PadMode::Keyboard
        }
    ));

    s.handle_midi(&[0x80, 0x69, 0x00], &mut bus, &mut midi);
    s.handle_midi(&[0x90, 0x6F, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.last().unwrap().1, Kind::SetPadMode);
    assert!(matches!(
        bus.cmds.last().unwrap().2,
        CmdBody::SetPadMode {
            mode: PadMode::KeyShift
        }
    ));
}
