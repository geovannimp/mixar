use engine_api::{CmdBody, Kind, Origin};
use std::path::Path;

struct CaptureBus {
    cmds: Vec<(Origin, Kind, CmdBody)>,
    library: Vec<(library_api::Origin, library_api::Kind, library_api::EvtBody)>,
}

impl controller::ActionPublish for CaptureBus {
    fn publish_engine(&mut self, origin: Origin, kind: Kind, body: CmdBody) {
        self.cmds.push((origin, kind, body));
    }
    fn publish_library(
        &mut self,
        origin: library_api::Origin,
        kind: library_api::Kind,
        body: library_api::EvtBody,
    ) {
        self.library.push((origin, kind, body));
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

struct NullMidi;
impl controller::MidiOut for NullMidi {
    fn send(&mut self, _bytes: &[u8]) {}
}

fn session() -> controller::MappingSession {
    let b = controller::load_bundle(Path::new("tests/fixtures/valid-minimal")).unwrap();
    controller::MappingSession::from_bundle(b).unwrap()
}

#[test]
fn note_toggle_play_publishes_toggle_play() {
    let mut s = session();
    let mut bus = CaptureBus {
        cmds: vec![],
        library: vec![],
    };
    let mut midi = NullMidi;
    // note on play_pause ch1 note 0x0B
    s.handle_midi(&[0x90, 0x0B, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].0, Origin::Deck(0));
    assert_eq!(bus.cmds[0].1, Kind::TogglePlay);
}

#[test]
fn modifier_shift_selects_set_filter() {
    let mut s = session();
    let mut bus = CaptureBus {
        cmds: vec![],
        library: vec![],
    };
    let mut midi = NullMidi;
    // hold shift
    s.handle_midi(&[0x90, 0x3F, 0x7F], &mut bus, &mut midi);
    assert!(bus.cmds.is_empty());
    // CC volume with shift → set_filter
    s.handle_midi(&[0xB0, 0x13, 64], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::SetFilter);
}

#[test]
fn soft_takeover_flag_passed_on_absolute_volume() {
    let mut s = session();
    let mut bus = CaptureBus {
        cmds: vec![],
        library: vec![],
    };
    let mut midi = NullMidi;
    s.handle_midi(&[0xB0, 0x13, 13], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::SetVolume);
    match &bus.cmds[0].2 {
        CmdBody::SetVolume {
            soft_takeover: true,
            ..
        } => {}
        other => panic!("expected soft_takeover volume, got {other:?}"),
    }
}

#[test]
fn headphone_cue_toggles_and_lights_led() {
    let mut s = session();
    let mut bus = CaptureBus {
        cmds: vec![],
        library: vec![],
    };
    let mut midi = CaptureMidi { frames: vec![] };

    s.handle_midi(&[0x90, 0x54, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::ToggleHeadphoneCue);
    assert_eq!(midi.frames.len(), 1, "PFL LED should turn on");
    assert_eq!(midi.frames[0], vec![0x90, 0x54, 0x7F]);

    s.handle_midi(&[0x80, 0x54, 0x00], &mut bus, &mut midi);
    s.handle_midi(&[0x90, 0x54, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 2);
    assert_eq!(bus.cmds[1].1, Kind::ToggleHeadphoneCue);
    assert_eq!(midi.frames.len(), 2, "PFL LED should turn off");
    assert_eq!(midi.frames[1], vec![0x90, 0x54, 0x00]);
}

#[test]
fn hot_cue_trigger_marks_playing_so_toggle_pauses() {
    let mut s = session();
    let mut cues = [None; 8];
    cues[0] = Some(1_000);
    let mut bus = CaptureBus {
        cmds: vec![],
        library: vec![],
    };
    let mut midi = CaptureMidi { frames: vec![] };
    s.set_deck_hot_cues(0, cues, &mut midi);
    assert!(s.snapshot().playing[0] == false);
    assert_eq!(midi.frames.len(), 1, "filled hot cue should light pad LED");
    assert_eq!(midi.frames[0], vec![0x90, 0x2E, 0x7F]);

    // pad_1 → TriggerHotCue (engine would start playback)
    s.handle_midi(&[0x90, 0x2E, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::TriggerHotCue);
    assert!(s.snapshot().playing[0], "hot cue must mark deck playing");

    bus.cmds.clear();
    s.handle_midi(&[0x80, 0x2E, 0x00], &mut bus, &mut midi);
    // play_pause always publishes TogglePlay; engine owns play/pause decision
    s.handle_midi(&[0x90, 0x0B, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.cmds.len(), 1);
    assert_eq!(bus.cmds[0].1, Kind::TogglePlay);
}

#[test]
fn set_deck_vu_sends_cc_with_mixxx_scale() {
    let mut s = session();
    let mut midi = CaptureMidi { frames: vec![] };
    s.set_deck_vu(0, 0.5, &mut midi);
    assert_eq!(midi.frames.len(), 1);
    // 0.5 * 150 = 75
    assert_eq!(midi.frames[0], vec![0xB0, 0x02, 75]);
    s.set_deck_vu(0, 0.5, &mut midi);
    assert_eq!(midi.frames.len(), 1, "duplicate VU must not resend");
    s.set_deck_vu(0, 1.0, &mut midi);
    assert_eq!(midi.frames[1], vec![0xB0, 0x02, 127]); // 150 clamped
}

#[test]
fn browse_relative_cc_publishes_library_navigation() {
    let mut s = session();
    let mut bus = CaptureBus {
        cmds: vec![],
        library: vec![],
    };
    let mut midi = NullMidi;
    // DDJ browse: ch7 CC 0x40, +1
    s.handle_midi(&[0xB6, 0x40, 0x01], &mut bus, &mut midi);
    assert_eq!(bus.library.len(), 1);
    assert_eq!(bus.library[0].0, library_api::Origin::LibraryNavigation);
    assert_eq!(bus.library[0].1, library_api::Kind::Navigate);
    assert!(matches!(
        bus.library[0].2,
        library_api::EvtBody::Navigate { delta: 1 }
    ));
    // −1
    s.handle_midi(&[0xB6, 0x40, 0x7F], &mut bus, &mut midi);
    assert_eq!(bus.library.len(), 2);
    assert_eq!(bus.library[1].1, library_api::Kind::Navigate);
    assert!(matches!(
        bus.library[1].2,
        library_api::EvtBody::Navigate { delta: -1 }
    ));
}
