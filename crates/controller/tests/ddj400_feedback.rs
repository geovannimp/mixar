//! Fixed-byte LED / pad feedback checks for the shipped DDJ-400 mapping.
//!
//! Every assertion pins the exact three MIDI bytes so a wrong channel, note or
//! velocity on a real unit shows up here rather than on hardware.

use std::path::Path;
use std::thread;
use std::time::Duration;

use controller::{DeckFeedback, MappingSession, MidiOut, BLINK_HALF_PERIOD};
use engine_api::{CmdBody, Kind, Origin, PadMode};

/// Deck 1 button channel, pad channels, master channel, beat-FX channel.
const D1: u8 = 0x90;
const D2: u8 = 0x91;
const PAD1: u8 = 0x97;
const PAD1_SHIFT: u8 = 0x98;
const PAD2: u8 = 0x99;
const PAD2_SHIFT: u8 = 0x9A;
const MASTER: u8 = 0x96;

struct CaptureMidi {
    frames: Vec<Vec<u8>>,
}

impl MidiOut for CaptureMidi {
    fn send(&mut self, bytes: &[u8]) {
        self.frames.push(bytes.to_vec());
    }
}

struct NullBus;

impl controller::ActionPublish for NullBus {
    fn publish_engine(&mut self, _origin: Origin, _kind: Kind, _body: CmdBody) {}
    fn publish_library(
        &mut self,
        _origin: library_api::Origin,
        _kind: library_api::Kind,
        _body: library_api::EvtBody,
    ) {
    }
}

/// Session as the host leaves it right after attach: `on_init` has already run,
/// so the LED cache is warm and later assertions see only real transitions.
fn attached() -> (MappingSession, CaptureMidi) {
    let bundle = controller::load_bundle(Path::new("../../mappings/ddj-400")).unwrap();
    let mut s = MappingSession::from_bundle(bundle).unwrap();
    let mut midi = CaptureMidi { frames: vec![] };
    s.on_init(&mut NullBus, &mut midi).unwrap();
    midi.frames.clear();
    (s, midi)
}

fn deck(pad_mode: PadMode) -> DeckFeedback {
    DeckFeedback {
        pad_mode,
        ..Default::default()
    }
}

/// Frames sorted, so assertions do not depend on section iteration order.
fn sorted(midi: &CaptureMidi) -> Vec<[u8; 3]> {
    let mut out: Vec<[u8; 3]> = midi
        .frames
        .iter()
        .filter(|f| f.len() == 3)
        .map(|f| [f[0], f[1], f[2]])
        .collect();
    out.sort_unstable();
    out
}

fn sorted_of(frames: &[[u8; 3]]) -> Vec<[u8; 3]> {
    let mut out = frames.to_vec();
    out.sort_unstable();
    out
}

#[test]
fn attach_asserts_lamps_and_detach_clears_them() {
    let bundle = controller::load_bundle(Path::new("../../mappings/ddj-400")).unwrap();
    let mut s = MappingSession::from_bundle(bundle).unwrap();
    let mut bus = NullBus;
    let mut midi = CaptureMidi { frames: vec![] };

    s.on_init(&mut bus, &mut midi).unwrap();
    // Forced sweep: everything is asserted dark except the deck's default page.
    let lit: Vec<[u8; 3]> = sorted(&midi).into_iter().filter(|f| f[2] != 0x00).collect();
    assert_eq!(
        lit,
        vec![[D1, 0x1B, 0x7F], [D2, 0x1B, 0x7F]],
        "only the hot-cue page lamps may start lit"
    );
    assert!(
        sorted(&midi).contains(&[D1, 0x0B, 0x00]),
        "attach must assert the dark state too"
    );
    assert!(
        sorted(&midi).contains(&[PAD1_SHIFT, 0x00, 0x00]),
        "attach must assert the hot-cue pad bank"
    );

    midi.frames.clear();
    s.set_deck_feedback(
        0,
        &DeckFeedback {
            playing: true,
            ..deck(PadMode::HotCue)
        },
        &mut midi,
    );
    assert_eq!(midi.frames, vec![vec![D1, 0x0B, 0x7F]], "play lamp lights");

    midi.frames.clear();
    s.on_shutdown(&mut bus, &mut midi).unwrap();
    let cleared = sorted(&midi);
    assert!(
        cleared.contains(&[D1, 0x0B, 0x00]),
        "detach clears the play lamp"
    );
    assert!(
        cleared.contains(&[D1, 0x1B, 0x00]),
        "detach clears the page lamp"
    );
    assert!(
        cleared.contains(&[D1, 0x0C, 0x00]),
        "detach clears the cue lamp"
    );
    assert!(
        cleared.contains(&[MASTER, 0x63, 0x00]),
        "detach clears master cue"
    );
    assert!(
        cleared.iter().all(|f| f[2] == 0x00),
        "every lamp must be dark after detach, got {cleared:?}"
    );
}

#[test]
fn hot_cue_pad_lights_its_own_bank_only() {
    let (mut s, mut midi) = attached();

    let mut fb = deck(PadMode::HotCue);
    fb.hot_cues[0] = Some(1_000);
    s.set_deck_feedback(0, &fb, &mut midi);
    assert_eq!(midi.frames, vec![vec![PAD1_SHIFT, 0x00, 0x7F]]);

    // Unloading the track clears the pad.
    midi.frames.clear();
    s.set_deck_feedback(0, &deck(PadMode::HotCue), &mut midi);
    assert_eq!(midi.frames, vec![vec![PAD1_SHIFT, 0x00, 0x00]]);
}

#[test]
fn pad_mode_switch_moves_the_lamp_between_pages() {
    let (mut s, mut midi) = attached();

    let mut fb = deck(PadMode::HotCue);
    fb.hot_cues[2] = Some(2_000);
    s.set_deck_feedback(0, &fb, &mut midi);
    midi.frames.clear();

    // Sampler page: the hot-cue pad bank and the page lamp both go dark.
    s.set_deck_feedback(0, &deck(PadMode::Sampler), &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [PAD1_SHIFT, 0x02, 0x00], // pad 3 hot-cue lamp off
            [D1, 0x1B, 0x00],         // HOT CUE button off
            [D1, 0x22, 0x7F],         // SAMPLER button on
        ])
    );

    // Back to the hot-cue page: the pad lights again without a cue change.
    midi.frames.clear();
    let mut back = deck(PadMode::HotCue);
    back.hot_cues[2] = Some(2_000);
    s.set_deck_feedback(0, &back, &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[[PAD1_SHIFT, 0x02, 0x7F], [D1, 0x1B, 0x7F], [D1, 0x22, 0x00],])
    );
}

#[test]
fn loop_roll_page_lights_saved_loop_slots() {
    let (mut s, mut midi) = attached();

    let mut fb = deck(PadMode::LoopRoll);
    fb.loop_slots[3] = true;
    s.set_deck_feedback(0, &fb, &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [PAD1, 0x63, 0x7F], // saved loop on pad 4
            [D1, 0x1B, 0x00],   // HOT CUE button off
            [D1, 0x6D, 0x7F],   // BEAT LOOP button on
        ])
    );
}

#[test]
fn loop_group_lights_while_a_loop_is_active() {
    let (mut s, mut midi) = attached();

    s.set_deck_feedback(
        0,
        &DeckFeedback {
            loop_active: true,
            ..deck(PadMode::HotCue)
        },
        &mut midi,
    );
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [D1, 0x10, 0x7F], // LOOP IN
            [D1, 0x11, 0x7F], // LOOP OUT
            [D1, 0x4D, 0x7F], // RELOOP / EXIT
            [D1, 0x4E, 0x7F], // +SHIFT LOOP OUT
            [D1, 0x50, 0x7F], // +SHIFT RELOOP / EXIT
        ])
    );

    midi.frames.clear();
    s.set_deck_feedback(0, &deck(PadMode::HotCue), &mut midi);
    assert!(
        sorted(&midi).iter().all(|f| f[1] != 0x4D || f[2] == 0x00),
        "exiting the loop clears the exit-loop lamp"
    );
}

#[test]
fn master_cue_lamp_follows_the_mixer() {
    let (mut s, mut midi) = attached();

    s.set_master_cue(true, &mut midi);
    assert_eq!(midi.frames, vec![vec![MASTER, 0x63, 0x7F]]);
    s.set_master_cue(false, &mut midi);
    assert_eq!(midi.frames[1], vec![MASTER, 0x63, 0x00]);
}

#[test]
fn holding_shift_blinks_the_mode_indicators() {
    let (mut s, mut midi) = attached();
    let mut bus = NullBus;

    // SHIFT down (deck 1, note 0x3F).
    s.handle_midi(&[D1, 0x3F, 0x7F], &mut bus, &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [D1, 0x1E, 0x7F], // +SHIFT BEAT LOOP
            [D1, 0x69, 0x7F], // +SHIFT HOT CUE
            [D1, 0x6B, 0x7F], // +SHIFT BEAT JUMP
            [D1, 0x6F, 0x7F], // +SHIFT SAMPLER
        ]),
        "shift must light all four indicators"
    );

    // Blink phase 1: lit.
    midi.frames.clear();
    s.tick_leds(&mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [D1, 0x1E, 0x7F],
            [D1, 0x69, 0x7F],
            [D1, 0x6B, 0x7F],
            [D1, 0x6F, 0x7F],
        ])
    );

    // Blink phase 2: dark.
    midi.frames.clear();
    thread::sleep(BLINK_HALF_PERIOD + Duration::from_millis(20));
    s.tick_leds(&mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [D1, 0x1E, 0x00],
            [D1, 0x69, 0x00],
            [D1, 0x6B, 0x00],
            [D1, 0x6F, 0x00],
        ])
    );

    // Release SHIFT: indicators go dark and stop blinking.
    midi.frames.clear();
    s.handle_midi(&[D1, 0x3F, 0x00], &mut bus, &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [D1, 0x1E, 0x00],
            [D1, 0x69, 0x00],
            [D1, 0x6B, 0x00],
            [D1, 0x6F, 0x00],
        ])
    );
    midi.frames.clear();
    thread::sleep(BLINK_HALF_PERIOD + Duration::from_millis(20));
    s.tick_leds(&mut midi);
    assert!(
        midi.frames.is_empty(),
        "released shift must not keep blinking"
    );
}

#[test]
fn deck_two_uses_its_own_channels() {
    let (mut s, mut midi) = attached();

    let mut fb = deck(PadMode::HotCue);
    fb.playing = true;
    fb.hot_cues[7] = Some(9_000);
    s.set_deck_feedback(1, &fb, &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[[D2, 0x0B, 0x7F], [PAD2_SHIFT, 0x07, 0x7F]])
    );

    midi.frames.clear();
    let mut loop_roll = deck(PadMode::LoopRoll);
    loop_roll.loop_slots[7] = true;
    s.set_deck_feedback(1, &loop_roll, &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [PAD2, 0x67, 0x7F],
            [D2, 0x0B, 0x00], // deck stopped playing
            [D2, 0x1B, 0x00], // HOT CUE page off
            [D2, 0x6D, 0x7F], // BEAT LOOP page on
            [PAD2_SHIFT, 0x07, 0x00],
        ])
    );
}

#[test]
fn controller_input_lights_leds_without_an_engine_echo() {
    let (mut s, mut midi) = attached();
    let mut bus = NullBus;

    // Headphone cue (note 0x54) toggles the PFL lamp.
    s.handle_midi(&[D1, 0x54, 0x7F], &mut bus, &mut midi);
    assert_eq!(midi.frames, vec![vec![D1, 0x54, 0x7F]]);
    midi.frames.clear();

    // Loop In alone must not light the loop group; only Loop Out completes it.
    s.handle_midi(&[D1, 0x10, 0x7F], &mut bus, &mut midi);
    assert!(
        midi.frames.is_empty(),
        "loop in alone leaves the loop lamps dark"
    );

    s.handle_midi(&[D1, 0x11, 0x7F], &mut bus, &mut midi);
    assert_eq!(
        sorted(&midi),
        sorted_of(&[
            [D1, 0x10, 0x7F],
            [D1, 0x11, 0x7F],
            [D1, 0x4D, 0x7F],
            [D1, 0x4E, 0x7F],
            [D1, 0x50, 0x7F],
        ])
    );
}
