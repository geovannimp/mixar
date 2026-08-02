use std::path::Path;

use engine_api::{CmdBody, Kind, Origin};

struct CaptureBus {
    cmds: Vec<(Origin, Kind, CmdBody)>,
}
impl controller::BusPublish for CaptureBus {
    fn publish(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        self.cmds.push((origin, kind, body));
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
fn on_init_calls_midi_out() {
    let b = controller::load_bundle(Path::new("tests/fixtures/with-script")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus { cmds: vec![] };
    let mut midi = CaptureMidi { frames: vec![] };
    s.on_init(&mut bus, &mut midi).unwrap();
    assert_eq!(midi.frames, vec![vec![0x90, 0x01, 0x7F]]);
}
