use std::collections::HashSet;

use controller::script::{ScriptHost, ScriptRuntime};
use controller::{ActionPublish, MidiOut};

struct CaptureBus;

impl ActionPublish for CaptureBus {
    fn publish_engine(
        &mut self,
        _origin: engine_api::Origin,
        _kind: engine_api::Kind,
        _body: engine_api::CmdBody,
    ) {
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

impl MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

#[test]
fn infinite_loop_hits_operation_limit() {
    let source = r#"
        fn on_init() {
            loop {}
        }
    "#;
    let mut runtime = ScriptRuntime::compile(source).unwrap();
    let mut bus = CaptureBus;
    let mut midi = CaptureMidi { frames: vec![] };
    let mut host = ScriptHost {
        bus: &mut bus,
        midi: &mut midi,
        modifiers: &HashSet::new(),
    };
    let err = runtime.call_hook("on_init", &mut host).unwrap_err();
    assert!(
        err.to_string().to_ascii_lowercase().contains("operations"),
        "expected operation limit error, got {err}"
    );
}

#[test]
fn midi_out_rejects_oversize_payload() {
    let bytes: String = (0..257)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("fn on_init() {{ midi_out([{bytes}]); }}");
    let mut runtime = ScriptRuntime::compile(&source).unwrap();
    let mut bus = CaptureBus;
    let mut midi = CaptureMidi { frames: vec![] };
    let mut host = ScriptHost {
        bus: &mut bus,
        midi: &mut midi,
        modifiers: &HashSet::new(),
    };
    let err = runtime.call_hook("on_init", &mut host).unwrap_err();
    assert!(
        err.to_string().contains("midi_out payload too large"),
        "expected midi cap error, got {err}"
    );
}
