//! Controller diagnostics: discarded MIDI/Rhai errors must reach tracing.

use std::path::Path;

use engine_api::{CmdBody, Kind, Origin};

struct CaptureBus;
impl controller::ActionPublish for CaptureBus {
    fn publish_engine(&mut self, _origin: Origin, _kind: Kind, _body: CmdBody) {}
    fn publish_library(
        &mut self,
        _origin: library_api::Origin,
        _kind: library_api::Kind,
        _body: library_api::EvtBody,
    ) {
    }
}

struct NullMidi;
impl controller::MidiOut for NullMidi {
    fn send(&mut self, _bytes: &[u8]) {}
}

#[test]
fn on_init_script_error_is_returned() {
    let b = controller::load_bundle(Path::new("tests/fixtures/script-fail")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus;
    let mut midi = NullMidi;
    let err = s.on_init(&mut bus, &mut midi).unwrap_err();
    assert!(
        err.to_string().contains("missing_pc_mode_sysex")
            || err.to_string().contains("Variable not found"),
        "{err}"
    );
}

#[test]
fn idle_heartbeat_script_error_is_returned() {
    let mut b = controller::load_bundle(Path::new("tests/fixtures/script-fail")).unwrap();
    b.map.lifecycle.on_init = None;
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus;
    let mut midi = NullMidi;
    let err = s.idle_heartbeat(&mut bus, &mut midi).unwrap_err();
    assert!(
        err.to_string().contains("missing_heartbeat_const")
            || err.to_string().contains("Variable not found"),
        "{err}"
    );
}

#[test]
fn named_script_binding_failure_is_reported() {
    let b = controller::load_bundle(Path::new("tests/fixtures/script-fail")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus;
    let mut midi = NullMidi;
    // Note on ch1 note 0x0B = play_pause → script boom
    let fail = s
        .handle_midi(&[0x90, 0x0B, 0x7F], &mut bus, &mut midi)
        .expect("script binding failure");
    assert_eq!(fail.section, "deck_1");
    assert_eq!(fail.alias, "play_pause");
    assert_eq!(fail.script_fn, "boom");
    assert!(
        fail.error.contains("missing_binding_const") || fail.error.contains("Variable not found"),
        "{}",
        fail.error
    );
}
