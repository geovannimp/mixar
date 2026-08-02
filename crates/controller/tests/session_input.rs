use engine_api::{CmdBody, Kind, Origin};
use std::path::Path;

struct CaptureBus {
    cmds: Vec<(Origin, Kind, CmdBody)>,
}

impl controller::BusPublish for CaptureBus {
    fn publish(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        self.cmds.push((origin, kind, body));
    }
}

fn session() -> controller::MappingSession {
    let b = controller::load_bundle(Path::new("tests/fixtures/valid-minimal")).unwrap();
    controller::MappingSession::from_bundle(b).unwrap()
}

#[test]
fn note_toggle_play_publishes_play_when_stopped() {
    let mut s = session();
    let mut bus = CaptureBus { cmds: vec![] };
    // note on play_pause ch1 note 0x0B
    s.handle_midi(&[0x90, 0x0B, 0x7F], &mut bus);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].0, Origin::Deck(0));
    assert_eq!(bus.cmds[0].1, Kind::Play);
}

#[test]
fn modifier_shift_selects_set_filter() {
    let mut s = session();
    let mut bus = CaptureBus { cmds: vec![] };
    // hold shift
    s.handle_midi(&[0x90, 0x3F, 0x7F], &mut bus);
    assert!(bus.cmds.is_empty());
    // CC volume with shift → set_filter
    s.handle_midi(&[0xB0, 0x13, 64], &mut bus);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::SetFilter);
}

#[test]
fn soft_takeover_blocks_then_latches() {
    let mut s = session();
    s.set_control_value(Origin::Deck(0), "volume", 0.9);
    let mut bus = CaptureBus { cmds: vec![] };
    // HW at ~0.1 — far from 0.9
    s.handle_midi(&[0xB0, 0x13, 13], &mut bus);
    assert!(bus.cmds.is_empty(), "should soft-takeover block");
    // HW near 0.9 (114/127 ≈ 0.897)
    s.handle_midi(&[0xB0, 0x13, 114], &mut bus);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::SetVolume);
}
