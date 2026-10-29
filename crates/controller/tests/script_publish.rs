#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::collections::HashSet;

use bus_api::{Event, SetVolume};
use controller::script::{ScriptHost, ScriptRuntime};
use controller::{ActionPublish, MidiOut};

struct CaptureBus {
    cmds: Vec<Event>,
}

impl ActionPublish for CaptureBus {
    fn publish_cmd(&mut self, event: Event) {
        self.cmds.push(event);
    }

    fn publish_evt(&mut self, _event: Event) {}
}

struct NoMidi;

impl MidiOut for NoMidi {
    fn send(&mut self, _bytes: &[u8]) {}
}

#[test]
fn publish_forwards_json_payload() {
    let source = r#"
        fn on_init() {
            publish("deck1", "set_volume", "{\"type\":\"set_volume\",\"volume\":0.5}");
        }
    "#;
    let mut runtime = ScriptRuntime::compile(source).unwrap();
    let mut bus = CaptureBus { cmds: vec![] };
    let mut midi = NoMidi;
    let mut host = ScriptHost {
        bus: &mut bus,
        midi: &mut midi,
        modifiers: &HashSet::new(),
    };
    runtime.call_hook("on_init", &mut host).unwrap();

    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(
        bus.cmds[0],
        Event::SetVolume(SetVolume {
            deck: 0,
            volume: 0.5,
            soft_takeover: false,
        })
    );
}
