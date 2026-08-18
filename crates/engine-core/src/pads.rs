//! Pad slot tables shared by press/release cmd handlers and tests.

/// Loop-roll beat lengths for slots 0..=7 (matches Flutter / Tauri grids).
pub const LOOP_ROLL_PAD_BEATS: [f32; 8] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];

/// Beat-jump sizes for the first four pads (forward). Back pads negate these.
const BEAT_JUMP_PAD_FORWARD: [f32; 4] = [1.0, 2.0, 4.0, 8.0];

pub fn loop_roll_beats(slot: u8) -> Option<f32> {
    LOOP_ROLL_PAD_BEATS.get(slot as usize).copied()
}

pub fn beat_jump_beats(slot: u8) -> Option<f32> {
    match slot {
        0..=3 => Some(BEAT_JUMP_PAD_FORWARD[slot as usize]),
        4..=7 => Some(-BEAT_JUMP_PAD_FORWARD[(slot - 4) as usize]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beat_jump_slots_match_ui_grid() {
        assert_eq!(beat_jump_beats(0), Some(1.0));
        assert_eq!(beat_jump_beats(3), Some(8.0));
        assert_eq!(beat_jump_beats(4), Some(-1.0));
        assert_eq!(beat_jump_beats(7), Some(-8.0));
        assert_eq!(beat_jump_beats(8), None);
    }
}
