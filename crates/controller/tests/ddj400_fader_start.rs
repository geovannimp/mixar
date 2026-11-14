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

#[test]
fn ddj400_native_fader_notes_and_cc14_volume_stay_distinct_and_ordered() {
    for (deck, channel) in [(0, 0), (1, 1)] {
        let mut s = ddj400_session();
        let mut bus = CaptureBus {
            cmds: vec![],
            evts: vec![],
        };
        let mut midi = NullMidi;

        // The shipped 1.4.1 bundle declares native notes separately from the
        // channel fader's CC14 pair (MSB 0x13, LSB 0x33).
        assert!(s
            .handle_midi(&[0x90 | channel, 0x66, 0x7F], &mut bus, &mut midi)
            .is_none());
        let mut expected = vec![Event::FaderStart(bus_api::FaderStart { deck })];
        assert_eq!(bus.cmds, expected);

        // A native note release must not act as the fader-stop control.
        s.handle_midi(&[0x80 | channel, 0x66, 0x00], &mut bus, &mut midi);
        assert_eq!(bus.cmds, expected);

        // The first complete CC14 sample publishes immediately, so this test
        // needs neither sleeps nor assumptions about the CC coalescing window.
        s.handle_midi(&[0xB0 | channel, 0x13, 0x7F], &mut bus, &mut midi);
        assert_eq!(bus.cmds, expected, "MSB alone is not a complete sample");
        assert!(s
            .handle_midi(&[0xB0 | channel, 0x33, 0x7F], &mut bus, &mut midi)
            .is_none());
        expected.push(Event::SetVolume(bus_api::SetVolume {
            deck,
            volume: 1.0,
            soft_takeover: true,
        }));
        assert_eq!(
            bus.cmds, expected,
            "volume must not emit another native start"
        );

        s.handle_midi(&[0x90 | channel, 0x52, 0x7F], &mut bus, &mut midi);
        expected.push(Event::FaderStop(bus_api::FaderStop { deck }));
        assert_eq!(bus.cmds, expected);

        // MIDI permits both explicit note-off and zero-velocity note-on
        // releases; neither is an additional native command.
        s.handle_midi(&[0x90 | channel, 0x52, 0x00], &mut bus, &mut midi);
        assert_eq!(bus.cmds, expected);
        assert!(bus.evts.is_empty(), "these controls publish commands only");
    }
}
