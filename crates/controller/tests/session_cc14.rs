//! Session tests for cc14 pairing.

use controller::{load_bundle, BusPublish, MappingSession};
use engine_api::{CmdBody, Kind, Origin};
use std::path::PathBuf;

struct Capture {
    cmds: Vec<(Origin, Kind, CmdBody)>,
}

impl BusPublish for Capture {
    fn publish(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        self.cmds.push((origin, kind, body));
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

    // MSB alone — no publish
    session.handle_midi(&[0xB0, 0x13, 0x40], &mut bus);
    assert!(bus.cmds.is_empty());

    // LSB completes pair
    session.handle_midi(&[0xB0, 0x33, 0x00], &mut bus);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::SetVolume);
    let CmdBody::SetVolume { volume } = &bus.cmds[0].2 else {
        panic!("expected SetVolume");
    };
    let expected = ((0x40u16 << 7) | 0x00) as f32 / 16383.0;
    assert!((volume - expected).abs() < 1e-5);
}
