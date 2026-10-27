//! Shipped DDJ-400 bundle: on_init Mixxx handshake, then declarative LED sweep.

#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::path::Path;

use engine_api::{CmdBody, Kind, Origin};

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

#[test]
fn on_init_emits_leds_then_pc_mode_sysex() {
    let b = controller::load_bundle(Path::new("../../mappings/ddj-400")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus { cmds: vec![] };
    let mut midi = CaptureMidi { frames: vec![] };
    s.on_init(&mut bus, &mut midi).unwrap();

    // Script Mixxx PioneerDDJ400.init order first…
    let mixxx = [
        vec![0x9F, 0x00, 0x7F],
        vec![0x9F, 0x01, 0x7F],
        vec![
            0xF0, 0x00, 0x40, 0x05, 0x00, 0x00, 0x02, 0x06, 0x00, 0x03, 0x01, 0xF7,
        ],
    ];
    assert!(
        midi.frames.len() > mixxx.len(),
        "declarative LED sweep must follow the Mixxx handshake"
    );
    assert_eq!(&midi.frames[..mixxx.len()], &mixxx);

    // …then the session force-sweeps map.toml outputs (default hot-cue page lit).
    assert!(
        midi.frames[mixxx.len()..]
            .iter()
            .any(|f| f.as_slice() == [0x90, 0x1B, 0x7F]),
        "deck 1 hot-cue page lamp must be asserted after SysEx"
    );
}
