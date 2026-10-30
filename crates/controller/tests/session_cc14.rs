//! Session tests for cc14 pairing.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use bus_api::{Event, SetVolume};
use controller::{load_bundle, ActionPublish, MappingSession};
use std::path::PathBuf;

struct Capture {
    cmds: Vec<Event>,
}

impl ActionPublish for Capture {
    fn publish_cmd(&mut self, event: Event) {
        self.cmds.push(event);
    }
    fn publish_evt(&mut self, _event: Event) {}
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn cc14_waits_for_both_bytes_then_publishes_volume() {
    let bundle = load_bundle(&fixture("cc14-volume")).unwrap();
    let mut session = MappingSession::from_bundle(bundle).unwrap();
    let mut bus = Capture { cmds: Vec::new() };
    struct NullMidi;
    impl controller::MidiOut for NullMidi {
        fn send(&mut self, _bytes: &[u8]) {}
    }
    let mut midi = NullMidi;

    // MSB alone — no publish
    session.handle_midi(&[0xB0, 0x13, 0x40], &mut bus, &mut midi);
    assert!(bus.cmds.is_empty());

    // LSB completes pair
    session.handle_midi(&[0xB0, 0x33, 0x00], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    let Event::SetVolume(SetVolume {
        deck: 0,
        volume,
        soft_takeover: false,
    }) = &bus.cmds[0]
    else {
        panic!("expected SetVolume, got {:?}", bus.cmds[0]);
    };
    let expected = (0x40u16 << 7) as f32 / 16383.0;
    assert!((volume - expected).abs() < 1e-5);
}
