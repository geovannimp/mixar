use std::path::Path;

use engine_api::{CmdBody, Kind, Origin};

struct CaptureBus {
    cmds: Vec<(Origin, Kind, CmdBody)>,
}
impl controller::ActionPublish for CaptureBus {
    fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        self.cmds.push((origin, kind, body));
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
impl controller::MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

#[test]
fn on_init_calls_midi_out() {
    let b = controller::load_bundle(Path::new("tests/fixtures/with-script")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus { cmds: vec![] };
    let mut midi = CaptureMidi { frames: vec![] };
    s.on_init(&mut bus, &mut midi).unwrap();
    assert_eq!(midi.frames, vec![vec![0x90, 0x01, 0x7F]]);
}

#[test]
fn idle_heartbeat_rate_limits_and_skips_when_playing() {
    let b = controller::load_bundle(Path::new("tests/fixtures/with-script")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut bus = CaptureBus { cmds: vec![] };
    let mut midi = CaptureMidi { frames: vec![] };

    s.idle_heartbeat(&mut bus, &mut midi).unwrap();
    assert_eq!(midi.frames.len(), 1, "first idle heartbeat should fire");
    assert_eq!(midi.frames[0], vec![0xB0, 0x02, 0x00]);

    s.idle_heartbeat(&mut bus, &mut midi).unwrap();
    assert_eq!(midi.frames.len(), 1, "second call within window must no-op");

    s.set_control_value(engine_api::Origin::Deck(0), "playing", 1.0);
    midi.frames.clear();
    s.idle_heartbeat(&mut bus, &mut midi).unwrap();
    assert!(
        midi.frames.is_empty(),
        "must not heartbeat while a deck is playing"
    );
}
