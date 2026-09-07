//! Controller diagnostics: discarded MIDI/Rhai errors must reach tracing.

use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use engine_api::{CmdBody, Kind, Origin};
use tracing_subscriber::fmt::MakeWriter;

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

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Capture {
    type Writer = Capture;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn with_capture(f: impl FnOnce()) -> String {
    let buf = Capture::default();
    let store = Arc::clone(&buf.0);
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(buf)
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    let bytes = store.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
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
fn named_script_binding_failure_is_logged() {
    let log = with_capture(|| {
        let b = controller::load_bundle(Path::new("tests/fixtures/script-fail")).unwrap();
        let mut s = controller::MappingSession::from_bundle(b).unwrap();
        let mut bus = CaptureBus;
        let mut midi = NullMidi;
        // Note on ch1 note 0x0B = play_pause → script boom
        s.handle_midi(&[0x90, 0x0B, 0x7F], &mut bus, &mut midi);
    });
    assert!(log.contains("script binding failed"), "{log}");
    assert!(log.contains("boom"), "{log}");
    assert!(
        log.contains("missing_binding_const") || log.contains("Variable not found"),
        "{log}"
    );
}
