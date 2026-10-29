//! Shipped DDJ-400 bundle: Keyboard / Key Shift pad banks, mode buttons, LEDs.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::path::Path;

use bus_api::Event;
use bus_api::KeyShiftPadPress;
use bus_api::KeyboardPadPress;
use bus_api::KeyboardPadRelease;
use bus_api::PadMode;
use bus_api::SetCueMix;
use bus_api::SetCueVolume;
use bus_api::SetMasterVolume;
use bus_api::SetPadMode;
use bus_api::ToggleMasterCue;

struct CaptureBus {
    cmds: Vec<Event>,
}

impl controller::ActionPublish for CaptureBus {
    fn publish_cmd(&mut self, event: Event) {
        self.cmds.push(event);
    }
    fn publish_evt(&mut self, _event: Event) {}
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
    assert_eq!(
        bus.cmds.last(),
        Some(&Event::KeyboardPadPress(KeyboardPadPress {
            deck: 0,
            slot: 0,
            shift: false
        }))
    );

    s.handle_midi(&[0x87, 0x40, 0x00], &mut bus, &mut midi);
    assert_eq!(
        bus.cmds.last(),
        Some(&Event::KeyboardPadRelease(KeyboardPadRelease {
            deck: 0,
            slot: 0
        }))
    );

    // Key Shift bank press: ch8 note 0x70 = key_shift_pad_1 (offset 0).
    s.handle_midi(&[0x97, 0x70, 0x7F], &mut bus, &mut midi);
    assert_eq!(
        bus.cmds.last(),
        Some(&Event::KeyShiftPadPress(KeyShiftPadPress {
            deck: 0,
            slot: 0,
            shift: false
        }))
    );

    // Shift bank: keyboard ch9 note 0x40 → KeyboardPadPress { shift: true }.
    s.handle_midi(&[0x98, 0x40, 0x7F], &mut bus, &mut midi);
    assert_eq!(
        bus.cmds.last(),
        Some(&Event::KeyboardPadPress(KeyboardPadPress {
            deck: 0,
            slot: 0,
            shift: true
        }))
    );

    // Shift bank: key-shift ch9 note 0x70 → KeyShiftPadPress { shift: true }.
    s.handle_midi(&[0x98, 0x70, 0x7F], &mut bus, &mut midi);
    assert_eq!(
        bus.cmds.last(),
        Some(&Event::KeyShiftPadPress(KeyShiftPadPress {
            deck: 0,
            slot: 0,
            shift: true
        }))
    );
}

#[test]
fn key_shift_pad_lights_led_and_reset_clears() {
    let mut s = session();
    let mut bus = new_bus();
    let mut midi = new_midi();

    // Enter the Key Shift page so the bank LEDs are driven.
    s.handle_midi(&[0x90, 0x6F, 0x7F], &mut bus, &mut midi);
    assert!(matches!(
        bus.cmds.last(),
        Some(Event::SetPadMode(SetPadMode { .. }))
    ));
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
    assert_eq!(
        bus.cmds.last(),
        Some(&Event::SetPadMode(SetPadMode {
            deck: 0,
            mode: PadMode::Keyboard
        }))
    );

    s.handle_midi(&[0x80, 0x69, 0x00], &mut bus, &mut midi);
    s.handle_midi(&[0x90, 0x6F, 0x7F], &mut bus, &mut midi);
    assert_eq!(
        bus.cmds.last(),
        Some(&Event::SetPadMode(SetPadMode {
            deck: 0,
            mode: PadMode::KeyShift
        }))
    );
}

#[test]
fn master_mixer_controls_dispatch() {
    let mut s = session();
    let mut bus = new_bus();
    let mut midi = new_midi();

    // MASTER CUE (ch7 / note 0x63) → ToggleMasterCue + LED on/off
    s.handle_midi(&[0x96, 0x63, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0], Event::ToggleMasterCue(ToggleMasterCue {}));
    assert_eq!(midi.frames.len(), 1, "MASTER CUE LED should turn on");
    assert_eq!(midi.frames[0], vec![0x96, 0x63, 0x7F]);

    s.handle_midi(&[0x86, 0x63, 0x00], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1, "note-off must not publish");
    assert_eq!(midi.frames.len(), 1, "note-off must not send MIDI");

    s.handle_midi(&[0x96, 0x63, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 2);
    assert_eq!(bus.cmds[1], Event::ToggleMasterCue(ToggleMasterCue {}));
    assert_eq!(midi.frames.len(), 2, "MASTER CUE LED should turn off");
    assert_eq!(midi.frames[1], vec![0x96, 0x63, 0x00]);

    // HEADPHONES MIXING cc14 0x0C/0x2C → SetCueMix soft-takeover (regression)
    bus.cmds.clear();
    s.handle_midi(&[0xB6, 0x0C, 0x40], &mut bus, &mut midi);
    assert!(bus.cmds.is_empty(), "MSB alone must not publish");
    s.handle_midi(&[0xB6, 0x2C, 0x00], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    let Event::SetCueMix(SetCueMix {
        soft_takeover: true,
        ..
    }) = bus.cmds[0]
    else {
        panic!("expected SetCueMix soft_takeover, got {:?}", bus.cmds[0]);
    };

    // MASTER LEVEL cc14 0x08/0x28 → SetMasterVolume soft-takeover
    bus.cmds.clear();
    s.handle_midi(&[0xB6, 0x08, 0x40], &mut bus, &mut midi);
    assert!(bus.cmds.is_empty(), "MSB alone must not publish");
    s.handle_midi(&[0xB6, 0x28, 0x00], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    let Event::SetMasterVolume(SetMasterVolume {
        soft_takeover: true,
        ..
    }) = bus.cmds[0]
    else {
        panic!(
            "expected SetMasterVolume soft_takeover, got {:?}",
            bus.cmds[0]
        );
    };

    // HEADPHONES LEVEL cc14 0x0D/0x2D → SetCueVolume soft_takeover
    bus.cmds.clear();
    s.handle_midi(&[0xB6, 0x0D, 0x40], &mut bus, &mut midi);
    assert!(bus.cmds.is_empty(), "MSB alone must not publish");
    s.handle_midi(&[0xB6, 0x2D, 0x00], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    let Event::SetCueVolume(SetCueVolume {
        soft_takeover: true,
        ..
    }) = bus.cmds[0]
    else {
        panic!("expected SetCueVolume soft_takeover, got {:?}", bus.cmds[0]);
    };
}
