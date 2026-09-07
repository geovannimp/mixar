//! Top-level Rhai `const` / `let` must be visible inside lifecycle hooks.

use std::collections::HashSet;

use controller::script::{ScriptHost, ScriptRuntime};
use controller::{ActionPublish, MidiOut};
use engine_api::{CmdBody, Kind, Origin};

struct CaptureBus;
impl ActionPublish for CaptureBus {
    fn publish_engine(&mut self, _origin: Origin, _kind: Kind, _body: CmdBody) {}
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
impl MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

#[test]
fn top_level_const_is_visible_in_hooks() {
    let source = r#"
        const X = [0xF0, 0x00, 0x7F];
        fn on_init() { midi_out(X); }
    "#;
    let mut runtime = ScriptRuntime::compile(source).unwrap();
    let mut bus = CaptureBus;
    let mut midi = CaptureMidi { frames: vec![] };
    let modifiers = HashSet::new();
    let mut host = ScriptHost {
        bus: &mut bus,
        midi: &mut midi,
        modifiers: &modifiers,
    };
    runtime.call_hook("on_init", &mut host).unwrap();
    assert_eq!(midi.frames, vec![vec![0xF0, 0x00, 0x7F]]);
}

#[test]
fn top_level_const_survives_second_hook_call() {
    let source = r#"
        const X = [0x01, 0x02];
        fn pulse() { midi_out(X); }
    "#;
    let mut runtime = ScriptRuntime::compile(source).unwrap();
    let mut bus = CaptureBus;
    let mut midi = CaptureMidi { frames: vec![] };
    let modifiers = HashSet::new();
    let mut host = ScriptHost {
        bus: &mut bus,
        midi: &mut midi,
        modifiers: &modifiers,
    };
    runtime.call_hook("pulse", &mut host).unwrap();
    runtime.call_hook("pulse", &mut host).unwrap();
    assert_eq!(midi.frames, vec![vec![0x01, 0x02], vec![0x01, 0x02]]);
}
