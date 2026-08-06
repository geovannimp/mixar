//! Session tests for cc14 pairing.

use controller::{load_bundle, ActionPublish, MappingSession};
use engine_api::{CmdBody, Kind, Origin};
use std::path::PathBuf;

struct Capture {
    cmds: Vec<(Origin, Kind, CmdBody)>,
}

impl ActionPublish for Capture {
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
    assert_eq!(bus.cmds[0].1, Kind::SetVolume);
    let CmdBody::SetVolume { volume } = &bus.cmds[0].2 else {
        panic!("expected SetVolume");
    };
    let expected = (0x40u16 << 7) as f32 / 16383.0;
    assert!((volume - expected).abs() < 1e-5);
}
