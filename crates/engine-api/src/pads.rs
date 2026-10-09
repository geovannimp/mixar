//! Keyboard / Key Shift pad slot tables.
//!
//! Single source of truth shared by the engine press/release handlers, the
//! controller LED mirror, and tests. Keyboard is pitch-only with 4 pages; Key
//! Shift shares pages 1–4 and adds the Reset/Up/Down/Sync utility page as page 5.

/// Action a Keyboard or Key Shift pad performs within a semitone page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PitchPadAction {
    /// Absolute semitone offset from the root.
    Semitone(i8),
    /// Reset the deck key shift to 0.
    KeyReset,
    /// Nudge the deck key shift up one semitone.
    SemitoneUp,
    /// Nudge the deck key shift down one semitone.
    SemitoneDown,
    /// Key sync (not implemented → no-op).
    KeySync,
    /// No action.
    None,
}

/// Number of Keyboard semitone pages (pitch-only; no utility page).
pub const KEYBOARD_PAGE_COUNT: u8 = 4;

/// Number of Key Shift semitone pages (includes the Reset/Up/Down/Sync utility page).
pub const KEY_SHIFT_PAGE_COUNT: u8 = 5;

/// Default Keyboard / Key Shift semitone page (`0…+7`), index 2 in both tables.
pub const DEFAULT_PITCH_PAGE: u8 = 2;

/// Rekordbox DDJ-400 footnote *6 Keyboard page table; slot 0 = pad 1 = MIDI offset 0.
///
/// Keyboard is pitch-only: four pages, no Reset/Up/Down/Sync utility page.
#[rustfmt::skip]
const KEYBOARD_PAGES: [[PitchPadAction; 8]; 4] = [
    // PAGE 1 — label "+8…+12"
    [
        PitchPadAction::Semitone(8), PitchPadAction::Semitone(9),
        PitchPadAction::Semitone(10), PitchPadAction::Semitone(11),
        PitchPadAction::Semitone(12), PitchPadAction::None,
        PitchPadAction::None, PitchPadAction::None,
    ],
    // PAGE 2 (default) — label "0…+7"
    [
        PitchPadAction::Semitone(0), PitchPadAction::Semitone(1),
        PitchPadAction::Semitone(2), PitchPadAction::Semitone(3),
        PitchPadAction::Semitone(4), PitchPadAction::Semitone(5),
        PitchPadAction::Semitone(6), PitchPadAction::Semitone(7),
    ],
    // PAGE 3 — label "-1…-8"
    [
        PitchPadAction::Semitone(-8), PitchPadAction::Semitone(-7),
        PitchPadAction::Semitone(-6), PitchPadAction::Semitone(-5),
        PitchPadAction::Semitone(-4), PitchPadAction::Semitone(-3),
        PitchPadAction::Semitone(-2), PitchPadAction::Semitone(-1),
    ],
    // PAGE 4 — label "-9…-12"
    [
        PitchPadAction::None, PitchPadAction::None,
        PitchPadAction::None, PitchPadAction::None,
        PitchPadAction::Semitone(-12), PitchPadAction::Semitone(-11),
        PitchPadAction::Semitone(-10), PitchPadAction::Semitone(-9),
    ],
];

/// Rekordbox DDJ-400 footnote *6 Key Shift page table; slot 0 = pad 1 = MIDI offset 0.
///
/// Key Shift shares pages 1–4 with Keyboard and adds the utility page as page 5.
#[rustfmt::skip]
const KEY_SHIFT_PAGES: [[PitchPadAction; 8]; 5] = [
    // PAGE 1 — label "+8…+12"
    [
        PitchPadAction::Semitone(8), PitchPadAction::Semitone(9),
        PitchPadAction::Semitone(10), PitchPadAction::Semitone(11),
        PitchPadAction::Semitone(12), PitchPadAction::None,
        PitchPadAction::None, PitchPadAction::None,
    ],
    // PAGE 2 (default) — label "0…+7"
    [
        PitchPadAction::Semitone(0), PitchPadAction::Semitone(1),
        PitchPadAction::Semitone(2), PitchPadAction::Semitone(3),
        PitchPadAction::Semitone(4), PitchPadAction::Semitone(5),
        PitchPadAction::Semitone(6), PitchPadAction::Semitone(7),
    ],
    // PAGE 3 — label "-1…-8"
    [
        PitchPadAction::Semitone(-8), PitchPadAction::Semitone(-7),
        PitchPadAction::Semitone(-6), PitchPadAction::Semitone(-5),
        PitchPadAction::Semitone(-4), PitchPadAction::Semitone(-3),
        PitchPadAction::Semitone(-2), PitchPadAction::Semitone(-1),
    ],
    // PAGE 4 — label "-9…-12"
    [
        PitchPadAction::None, PitchPadAction::None,
        PitchPadAction::None, PitchPadAction::None,
        PitchPadAction::Semitone(-12), PitchPadAction::Semitone(-11),
        PitchPadAction::Semitone(-10), PitchPadAction::Semitone(-9),
    ],
    // PAGE 5 — label "UTIL"
    [
        PitchPadAction::KeyReset, PitchPadAction::SemitoneDown,
        PitchPadAction::Semitone(-5), PitchPadAction::Semitone(-12),
        PitchPadAction::KeySync, PitchPadAction::SemitoneUp,
        PitchPadAction::Semitone(7), PitchPadAction::Semitone(12),
    ],
];

/// Keyboard page action for `slot` (`0..=7`) on `page` (clamped to `1..=4`).
pub fn keyboard_page_action(page: u8, slot: u8) -> PitchPadAction {
    let page = page.clamp(1, KEYBOARD_PAGE_COUNT);
    let slot = slot.min(7) as usize;
    KEYBOARD_PAGES[(page - 1) as usize][slot]
}

/// Key Shift page action for `slot` (`0..=7`) on `page` (clamped to `1..=5`).
pub fn key_shift_page_action(page: u8, slot: u8) -> PitchPadAction {
    let page = page.clamp(1, KEY_SHIFT_PAGE_COUNT);
    let slot = slot.min(7) as usize;
    KEY_SHIFT_PAGES[(page - 1) as usize][slot]
}

/// Next Keyboard page, wrapping `4 → 1`.
pub fn keyboard_page_next(page: u8) -> u8 {
    let page = page.clamp(1, KEYBOARD_PAGE_COUNT);
    if page >= KEYBOARD_PAGE_COUNT {
        1
    } else {
        page + 1
    }
}

/// Previous Keyboard page, wrapping `1 → 4`.
pub fn keyboard_page_prev(page: u8) -> u8 {
    let page = page.clamp(1, KEYBOARD_PAGE_COUNT);
    if page <= 1 {
        KEYBOARD_PAGE_COUNT
    } else {
        page - 1
    }
}

/// Next Key Shift page, wrapping `5 → 1`.
pub fn key_shift_page_next(page: u8) -> u8 {
    let page = page.clamp(1, KEY_SHIFT_PAGE_COUNT);
    if page >= KEY_SHIFT_PAGE_COUNT {
        1
    } else {
        page + 1
    }
}

/// Previous Key Shift page, wrapping `1 → 5`.
pub fn key_shift_page_prev(page: u8) -> u8 {
    let page = page.clamp(1, KEY_SHIFT_PAGE_COUNT);
    if page <= 1 {
        KEY_SHIFT_PAGE_COUNT
    } else {
        page - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_pages_are_pitch_only() {
        assert_eq!(KEYBOARD_PAGE_COUNT, 4);
        assert_eq!(KEYBOARD_PAGES.len(), 4);
        assert!(
            KEYBOARD_PAGES
                .iter()
                .flatten()
                .all(|action| matches!(action, PitchPadAction::Semitone(_) | PitchPadAction::None)),
            "Keyboard pages must never contain utility actions"
        );
        assert_eq!(keyboard_page_action(2, 0), PitchPadAction::Semitone(0));
        assert_eq!(keyboard_page_action(2, 7), PitchPadAction::Semitone(7));
        assert_eq!(keyboard_page_action(4, 7), PitchPadAction::Semitone(-9));
        // Out-of-range pages clamp into `1..=4` (page 4).
        assert_eq!(keyboard_page_action(9, 0), PitchPadAction::None);
    }

    #[test]
    fn key_shift_pages_include_utility_page() {
        assert_eq!(KEY_SHIFT_PAGE_COUNT, 5);
        assert_eq!(KEY_SHIFT_PAGES.len(), 5);
        assert_eq!(key_shift_page_action(5, 0), PitchPadAction::KeyReset);
        assert_eq!(key_shift_page_action(5, 1), PitchPadAction::SemitoneDown);
        assert_eq!(key_shift_page_action(5, 2), PitchPadAction::Semitone(-5));
        assert_eq!(key_shift_page_action(5, 4), PitchPadAction::KeySync);
        assert_eq!(key_shift_page_action(5, 5), PitchPadAction::SemitoneUp);
        assert_eq!(key_shift_page_action(5, 6), PitchPadAction::Semitone(7));
        assert_eq!(key_shift_page_action(5, 7), PitchPadAction::Semitone(12));
    }

    #[test]
    fn pitch_page_wrapping_is_per_mode() {
        assert_eq!(DEFAULT_PITCH_PAGE, 2);
        assert_eq!(keyboard_page_next(4), 1);
        assert_eq!(keyboard_page_next(2), 3);
        assert_eq!(keyboard_page_prev(1), KEYBOARD_PAGE_COUNT);
        assert_eq!(keyboard_page_prev(3), 2);
        assert_eq!(key_shift_page_next(5), 1);
        assert_eq!(key_shift_page_next(2), 3);
        assert_eq!(key_shift_page_prev(1), KEY_SHIFT_PAGE_COUNT);
        assert_eq!(key_shift_page_prev(3), 2);
    }
}
