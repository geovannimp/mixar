#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use bus_api::Event;
use bus_api::TogglePlay;
use std::path::Path;

use controller::MidiOut;

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

fn playing_feedback() -> controller::DeckFeedback {
    controller::DeckFeedback {
        playing: true,
        ..Default::default()
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

    s.handle_midi(&[0x90, 0x0B, 0x7F], &mut bus, &mut port.out);
    assert_eq!(bus.cmds, vec![Event::TogglePlay(TogglePlay { deck: 0 })]);
    assert!(
        port.out.frames.is_empty(),
        "TogglePlay does not light LED until engine Updated"
    );

    s.set_deck_feedback(0, &playing_feedback(), &mut port.out);
    assert_eq!(port.out.frames[0], vec![0x90, 0x0C, 0x7F]);

    s.set_deck_feedback(0, &playing_feedback(), &mut port.out);
    assert_eq!(
        port.out.frames.len(),
        1,
        "duplicate playing must not resend LED"
    );

    let _ = controller::MidiPort::send(&mut port, &[0x90, 0x00, 0x00]);
    assert_eq!(port.out.frames.len(), 2);
}
