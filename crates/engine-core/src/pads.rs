//! Pad slot tables and helpers.
//!
//! The Keyboard / Key Shift semitone tables live in [`engine_api::pads`] so the
//! controller LED mirror and the engine share one source of truth; they are
//! re-exported here to keep `engine_core::pads::*` paths working.

pub(crate) const HOT_CUE_SLOT_COUNT: usize = 16;

pub use engine_api::pads::{
    key_shift_page_action, key_shift_page_next, key_shift_page_prev, keyboard_page_action,
    keyboard_page_next, keyboard_page_prev, PitchPadAction, DEFAULT_PITCH_PAGE,
    KEYBOARD_PAGE_COUNT, KEY_SHIFT_PAGE_COUNT,
};

/// Loop-roll beat lengths for slots 0..=7 (Flutter / Tauri grids).
pub const LOOP_ROLL_PAD_BEATS: [f32; 8] = [
    1.0 / 32.0,
    1.0 / 16.0,
    1.0 / 8.0,
    1.0 / 4.0,
    1.0 / 2.0,
    1.0,
    2.0,
    4.0,
];

/// Beat-jump sizes for slots 0..=7: forward then backward (Flutter / Tauri grids).
pub const BEAT_JUMP_PAD_BEATS: [f32; 8] = [1.0, 2.0, 4.0, 8.0, -1.0, -2.0, -4.0, -8.0];

pub fn loop_roll_beats(slot: u8) -> Option<f32> {
    LOOP_ROLL_PAD_BEATS.get(slot as usize).copied()
}

pub fn beat_jump_beats(slot: u8) -> Option<f32> {
    BEAT_JUMP_PAD_BEATS.get(slot as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_roll_slots_are_fractions_then_bars() {
        assert_eq!(loop_roll_beats(0), Some(1.0 / 32.0));
        assert_eq!(loop_roll_beats(4), Some(0.5));
        assert_eq!(loop_roll_beats(7), Some(4.0));
        assert_eq!(loop_roll_beats(8), None);
    }

    #[test]
    fn beat_jump_slots_match_ui_grid() {
        assert_eq!(beat_jump_beats(0), Some(1.0));
        assert_eq!(beat_jump_beats(3), Some(8.0));
        assert_eq!(beat_jump_beats(4), Some(-1.0));
        assert_eq!(beat_jump_beats(7), Some(-8.0));
        assert_eq!(beat_jump_beats(8), None);
    }
}
