use engine_api::{CmdBody, Kind, Origin};
use std::path::Path;

use controller::MidiOut;

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
impl MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

struct FakePort {
    out: CaptureMidi,
}
impl controller::MidiPort for FakePort {
    fn send(&mut self, bytes: &[u8]) -> Result<(), controller::MidiPortError> {
        self.out.send(bytes);
        Ok(())
    }
}

#[test]
fn fake_port_midi_in_and_led_out() {
    let b = controller::load_bundle(Path::new("tests/fixtures/valid-minimal")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus { cmds: vec![] };
    let mut port = FakePort {
        out: CaptureMidi { frames: vec![] },
    };

    s.handle_midi(&[0x90, 0x0B, 0x7F], &mut bus);
    assert_eq!(bus.cmds[0].1, Kind::Play);

    s.on_deck_playing(0, true, &mut port.out);
    assert_eq!(port.out.frames[0], vec![0x90, 0x0C, 0x7F]);

    let _ = controller::MidiPort::send(&mut port, &[0x90, 0x00, 0x00]);
    assert_eq!(port.out.frames.len(), 2);
}
