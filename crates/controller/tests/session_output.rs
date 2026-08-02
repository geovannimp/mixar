use std::path::Path;

struct CaptureMidi {
    frames: Vec<Vec<u8>>,
}

impl controller::MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

#[test]
fn play_pause_output_fires_pause_led_when_playing() {
    let b = controller::load_bundle(Path::new("tests/fixtures/valid-minimal")).unwrap();
    let mut s = controller::MappingSession::from_bundle(b).unwrap();
    let mut midi = CaptureMidi { frames: vec![] };
    s.on_deck_playing(0, true, &mut midi);
    assert_eq!(midi.frames.len(), 1);
    // pause_led: note ch1 note 0x0C vel 0x7F
    assert_eq!(midi.frames[0], vec![0x90, 0x0C, 0x7F]);
}
