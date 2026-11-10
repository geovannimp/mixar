#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use bus_api::Event;
use std::path::Path;

struct CaptureBus {
    cmds: Vec<Event>,
    evts: Vec<Event>,
}

impl controller::ActionPublish for CaptureBus {
    fn publish_cmd(&mut self, event: Event) {
        self.cmds.push(event);
    }
    fn publish_evt(&mut self, event: Event) {
        self.evts.push(event);
    }
}

struct NullMidi;
impl controller::MidiOut for NullMidi {
    fn send(&mut self, _bytes: &[u8]) {}
}

fn ddj400_session() -> controller::MappingSession {
    let b = controller::load_bundle(Path::new("../../mappings/ddj-400")).unwrap();
    controller::MappingSession::from_bundle(b).unwrap()
}

#[test]
fn ddj400_fader_start_notes_publish_native_events() {
    let mut s = ddj400_session();
    let mut bus = CaptureBus {
        cmds: vec![],
        evts: vec![],
    };
    let mut midi = NullMidi;

    // deck 1 (MIDI channel 1): fader left zero -> FaderStart, fader returned -> FaderStop
    s.handle_midi(&[0x90, 0x66, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(
        bus.cmds[0],
        Event::FaderStart(bus_api::FaderStart { deck: 0 })
    );

    s.handle_midi(&[0x90, 0x52, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 2);
    assert_eq!(
        bus.cmds[1],
        Event::FaderStop(bus_api::FaderStop { deck: 0 })
    );

    // deck 2 (MIDI channel 2)
    s.handle_midi(&[0x91, 0x66, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 3);
    assert_eq!(
        bus.cmds[2],
        Event::FaderStart(bus_api::FaderStart { deck: 1 })
    );

    s.handle_midi(&[0x91, 0x52, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 4);
    assert_eq!(
        bus.cmds[3],
        Event::FaderStop(bus_api::FaderStop { deck: 1 })
    );
}
