//! Musical key → Pioneer key-table id.
//!
//! Rekordbox's `keys` table numbers the 24 keys chromatically from A: minor
//! keys take ids 1–12 (Am, Bbm, Bm, …), major keys 13–24 (A, Bb, B, …). The
//! device database always carries the same 24 rows; a track row references its
//! key by id.
//!
//! Mixar stores keys in musical notation with sharps ([`library_core::MAJOR_KEYS`] /
//! [`library_core::MINOR_KEYS`]); Pioneer spells the flat enharmonics
//! (`F#m` is `Gbm` on the device), so ids are derived from the pitch class
//! rather than from the spelling.

use library_core::camelot_to_musical;

/// The 24 `(id, name)` rows of the device `keys` table, in id order.
///
/// Names follow Rekordbox's spelling (`Am`, `Db`, …).
pub const KEY_TABLE: [(u32, &str); 24] = [
    (1, "Am"),
    (2, "Bbm"),
    (3, "Bm"),
    (4, "Cm"),
    (5, "Dbm"),
    (6, "Dm"),
    (7, "Ebm"),
    (8, "Em"),
    (9, "Fm"),
    (10, "Gbm"),
    (11, "Gm"),
    (12, "Abm"),
    (13, "A"),
    (14, "Bb"),
    (15, "B"),
    (16, "C"),
    (17, "Db"),
    (18, "D"),
    (19, "Eb"),
    (20, "E"),
    (21, "F"),
    (22, "Gb"),
    (23, "G"),
    (24, "Ab"),
];

/// Pioneer key-table id (1–24) for a stored musical key, or `None` when the
/// key is absent or not parseable.
///
/// Accepts musical notation (`"F#m"`, `"Bb"`, `"amin"`) and Camelot codes
/// (`"8A"`), which file tags occasionally carry.
pub fn id_for_musical(key: &str) -> Option<u32> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return None;
    }

    let musical = match camelot_to_musical(trimmed) {
        Some(musical) => musical,
        None if looks_like_camelot(trimmed) => return None,
        None => trimmed.to_owned(),
    };

    let (root, minor) = split_mode(&musical)?;
    let pitch = pitch_class(root)?;
    // Id 1 is A minor; the table walks the chromatic scale from A.
    let offset = (i32::from(pitch) - 9).rem_euclid(12) as u32;
    Some(if minor { 1 + offset } else { 13 + offset })
}

fn looks_like_camelot(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_digit() => {}
        _ => return false,
    }
    let mut rest: Vec<char> = chars.collect();
    match rest.pop() {
        Some(last) => {
            matches!(last, 'A' | 'B' | 'a' | 'b') && rest.iter().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// Split `"F#m"` into its root (`"F#"`) and mode.
fn split_mode(value: &str) -> Option<(&str, bool)> {
    for (suffix, minor) in [
        ("minor", true),
        ("min", true),
        ("major", false),
        ("maj", false),
    ] {
        if value.len() < suffix.len() {
            continue;
        }
        let (root, tail) = value.split_at(value.len() - suffix.len());
        if tail.eq_ignore_ascii_case(suffix) && !root.trim().is_empty() {
            return Some((root.trim(), minor));
        }
    }
    if let Some(root) = value
        .strip_suffix('m')
        .or_else(|| value.strip_suffix('M'))
        .filter(|root| !root.trim().is_empty())
    {
        return Some((root.trim(), true));
    }
    (!value.trim().is_empty()).then(|| (value.trim(), false))
}

/// Pitch class (`C` = 0 … `B` = 11) of a note name such as `"F#"`, `"Bb"`,
/// `"C"`.
fn pitch_class(root: &str) -> Option<u8> {
    let mut chars = root.chars();
    let base = match chars.next()?.to_ascii_uppercase() {
        'C' => 0i32,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let mut pitch = base;
    for accidental in chars {
        match accidental {
            '#' | '♯' => pitch += 1,
            'b' | '♭' => pitch -= 1,
            _ => return None,
        }
    }
    Some(pitch.rem_euclid(12) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use library_core::{MAJOR_KEYS, MINOR_KEYS};

    #[test]
    fn ids_are_unique_and_cover_the_table() {
        for (index, (id, _)) in KEY_TABLE.iter().enumerate() {
            assert_eq!(*id as usize, index + 1);
        }
    }

    #[test]
    fn sharps_and_flats_share_an_id() {
        assert_eq!(id_for_musical("F#m"), id_for_musical("Gbm"));
        assert_eq!(id_for_musical("C#m"), id_for_musical("Dbm"));
        assert_eq!(id_for_musical("A#"), id_for_musical("Bb"));
    }

    #[test]
    fn known_keys_map_to_the_documented_ids() {
        assert_eq!(id_for_musical("Am"), Some(1));
        assert_eq!(id_for_musical("A"), Some(13));
        assert_eq!(id_for_musical("C"), Some(16));
        assert_eq!(id_for_musical("Ab"), Some(24));
        assert_eq!(id_for_musical("amin"), Some(1));
        assert_eq!(id_for_musical("F# major"), Some(22));
    }

    #[test]
    fn camelot_codes_are_accepted() {
        assert_eq!(id_for_musical("8A"), Some(1));
        assert_eq!(id_for_musical("8B"), Some(16));
    }

    #[test]
    fn stored_spellings_cover_both_halves_of_the_table() {
        let mut minor_ids: Vec<u32> = MINOR_KEYS
            .iter()
            .filter_map(|key| id_for_musical(key))
            .collect();
        minor_ids.sort_unstable();
        minor_ids.dedup();
        assert_eq!(minor_ids, (1..=12).collect::<Vec<u32>>());

        let mut major_ids: Vec<u32> = MAJOR_KEYS
            .iter()
            .filter_map(|key| id_for_musical(key))
            .collect();
        major_ids.sort_unstable();
        major_ids.dedup();
        assert_eq!(major_ids, (13..=24).collect::<Vec<u32>>());
    }

    #[test]
    fn unknown_input_is_none() {
        assert_eq!(id_for_musical(""), None);
        assert_eq!(id_for_musical("  "), None);
        assert_eq!(id_for_musical("H"), None);
        assert_eq!(id_for_musical("8C"), None);
        assert_eq!(id_for_musical("m"), None);
    }
}
