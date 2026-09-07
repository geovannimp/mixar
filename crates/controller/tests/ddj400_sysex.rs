//! DDJ-400 shipped mapping: PC-mode SysEx must emit exact Mixxx bytes.

use std::path::PathBuf;

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

struct CaptureMidi {
    frames: Vec<Vec<u8>>,
}
impl controller::MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

fn ddj400_bundle_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mappings/ddj-400")
}

const PC_MODE_SYSEX: &[u8] = &[
    0xF0, 0x00, 0x40, 0x05, 0x00, 0x00, 0x02, 0x06, 0x00, 0x03, 0x01, 0xF7,
];

#[test]
fn ddj400_on_init_sends_pc_mode_sysex_and_pad_lights() {
    let b = controller::load_bundle(&ddj400_bundle_dir()).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus;
    let mut midi = CaptureMidi { frames: vec![] };
    s.on_init(&mut bus, &mut midi).unwrap();
    assert_eq!(
        midi.frames,
        vec![
            PC_MODE_SYSEX.to_vec(),
            vec![0x9F, 0x00, 0x7F],
            vec![0x9F, 0x01, 0x7F],
        ]
    );
}

#[test]
fn ddj400_idle_heartbeat_reasserts_pc_mode_sysex() {
    let b = controller::load_bundle(&ddj400_bundle_dir()).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus;
    let mut midi = CaptureMidi { frames: vec![] };
    s.idle_heartbeat(&mut bus, &mut midi).unwrap();
    assert_eq!(midi.frames, vec![PC_MODE_SYSEX.to_vec()]);
}
