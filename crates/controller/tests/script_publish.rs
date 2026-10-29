#![allow(clippy::unwrap_used)] // test crate: unwrap on fixtures and helpers
use std::collections::HashSet;

use controller::script::{ScriptHost, ScriptRuntime};
use controller::{ActionPublish, MidiOut};
use engine_api::{Event, SetVolume};

struct CaptureBus {
    cmds: Vec<Event>,
}

impl ActionPublish for CaptureBus {
    fn publish_engine(&mut self, event: Event) {
        self.cmds.push(event);
    }

    fn publish_library(
        &mut self,
        _origin: library_api::Origin,
        _kind: library_api::Kind,
        _body: library_api::EvtBody,
    ) {
    }
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
