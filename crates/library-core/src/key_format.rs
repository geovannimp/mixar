//! Musical ↔ Camelot (Mixed In Key) key display helpers.
//!
//! Storage and APIs keep musical notation. UIs call [`format_key`] with the
//! user's [`KeyDisplayMode`] from app settings.

/// How keys are shown in the UI.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum KeyDisplayMode {
    #[default]
    Musical,
    Camelot,
}

/// Circle-of-fifths majors starting at C. Index `i` → Camelot `(i + 7) % 12 + 1` + `B`.
pub const MAJOR_KEYS: [&str; 12] = [
    "C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "D#", "A#", "F",
];

/// Relative minors starting at Am. Index `i` → Camelot `(i + 7) % 12 + 1` + `A`.
pub const MINOR_KEYS: [&str; 12] = [
    "Am", "Em", "Bm", "F#m", "C#m", "G#m", "D#m", "A#m", "Fm", "Cm", "Gm", "Dm",
];

const CAMELOT_OFFSET: usize = 7;

/// Format a stored key for display. Empty input returns empty; unknown tokens pass through.
pub fn format_key(key: &str, mode: KeyDisplayMode) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    match mode {
        KeyDisplayMode::Musical => {
            camelot_to_musical(trimmed).unwrap_or_else(|| trimmed.to_string())
        }
        KeyDisplayMode::Camelot => {
            musical_to_camelot(trimmed).unwrap_or_else(|| trimmed.to_string())
        }
    }
}

/// Musical → Camelot (`C` → `8B`, `Am` → `8A`).
pub fn musical_to_camelot(key: &str) -> Option<String> {
    let trimmed = key.trim();
    if let Some(i) = MAJOR_KEYS.iter().position(|k| *k == trimmed) {
        return Some(format!("{}B", (i + CAMELOT_OFFSET) % 12 + 1));
    }
    if let Some(i) = MINOR_KEYS.iter().position(|k| *k == trimmed) {
        return Some(format!("{}A", (i + CAMELOT_OFFSET) % 12 + 1));
    }
    None
}

/// Camelot → musical (`8B` → `C`, `8A` → `Am`). Accepts lower/upper letter suffix.
pub fn camelot_to_musical(code: &str) -> Option<String> {
    let trimmed = code.trim();
    if trimmed.len() < 2 {
        return None;
    }
    let upper = trimmed.to_uppercase();
    let (number_text, minor) = if let Some(rest) = upper.strip_suffix('A') {
        (rest, true)
    } else {
        let rest = upper.strip_suffix('B')?;
        (rest, false)
    };
    let number: usize = number_text.parse().ok()?;
    if !(1..=12).contains(&number) {
        return None;
    }
    let index = (number + 12 - 1 - CAMELOT_OFFSET) % 12;
    if minor {
        Some(MINOR_KEYS[index].to_string())
    } else {
        Some(MAJOR_KEYS[index].to_string())
    }
}

/// Camelot number + minor flag → musical (tag import).
pub fn camelot_code_to_musical(code: usize, minor: bool) -> Option<String> {
    if !(1..=12).contains(&code) {
        return None;
    }
    let index = (code + 12 - 1 - CAMELOT_OFFSET) % 12;
    if minor {
        Some(MINOR_KEYS[index].to_string())
    } else {
        Some(MAJOR_KEYS[index].to_string())
    }
}

/// Canonical musical spelling for a musical or Camelot token (`Bb` → `A#`,
/// `Bbm` → `A#m`, `8A` → `Am`).
///
/// Folds the five common flat spellings onto the sharp names used by
/// [`MAJOR_KEYS`]/[`MINOR_KEYS`]. `None` for empty/unknown tokens.
pub fn canonical_key(key: &str) -> Option<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(musical) = camelot_to_musical(trimmed) {
        return Some(musical);
    }
    // Tags are usually written `Am` / `Bb`, but a lowercase root is common in
    // free-form comments; normalize it before folding flats.
    let mut normalized = String::with_capacity(trimmed.len());
    for (index, ch) in trimmed.chars().enumerate() {
        normalized.push(if index == 0 {
            ch.to_ascii_uppercase()
        } else {
            ch
        });
    }
    let folded = fold_flats(&normalized);
    MAJOR_KEYS
        .iter()
        .chain(MINOR_KEYS.iter())
        .any(|known| *known == folded)
        .then_some(folded)
}

/// Camelot-compatible family of `key`: the key itself, one number down with the
/// same letter, one number up with the same letter, then the same number with
/// the opposite letter (relative major/minor) — in that order, canonical
/// musical spellings, de-duplicated.
///
/// `None` when `key` is not a known key. `harmonic_family("8A")` is
/// `["Am", "Dm", "Em", "C"]`.
pub fn harmonic_family(key: &str) -> Option<Vec<String>> {
    let musical = canonical_key(key)?;
    let camelot = musical_to_camelot(&musical)?;
    let minor = camelot.ends_with('A');
    let number: usize = camelot.trim_end_matches(['A', 'B']).parse().ok()?;
    let same_letter = if minor { 'A' } else { 'B' };
    let candidates = [
        camelot,
        format!("{}{same_letter}", if number == 1 { 12 } else { number - 1 }),
        format!("{}{same_letter}", if number == 12 { 1 } else { number + 1 }),
        format!("{number}{}", if minor { 'B' } else { 'A' }),
    ];

    let mut family: Vec<String> = Vec::with_capacity(candidates.len());
    for code in candidates {
        let Some(member) = camelot_to_musical(&code) else {
            continue;
        };
        if !family.contains(&member) {
            family.push(member);
        }
    }
    Some(family)
}

/// Fold `Bb`/`Eb`/`Ab`/`Db`/`Gb` (optionally minor) onto sharp spellings.
fn fold_flats(key: &str) -> String {
    let (letter, suffix) = match key.strip_suffix('m') {
        Some(rest) => (rest, "m"),
        None => (key, ""),
    };
    let mut chars = letter.chars();
    let Some(root) = chars.next() else {
        return key.to_string();
    };
    if chars.as_str() != "b" {
        return key.to_string();
    }
    let sharp = match root {
        'B' => "A#",
        'E' => "D#",
        'A' => "G#",
        'D' => "C#",
        'G' => "F#",
        _ => return key.to_string(),
    };
    format!("{sharp}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_in_key_anchors() {
        assert_eq!(musical_to_camelot("C").as_deref(), Some("8B"));
        assert_eq!(musical_to_camelot("Am").as_deref(), Some("8A"));
        assert_eq!(camelot_to_musical("8B").as_deref(), Some("C"));
        assert_eq!(camelot_to_musical("8A").as_deref(), Some("Am"));
        assert_eq!(camelot_to_musical("1A").as_deref(), Some("G#m"));
        assert_eq!(musical_to_camelot("G#m").as_deref(), Some("1A"));
    }

    #[test]
    fn round_trip_all_wheel() {
        for key in MAJOR_KEYS.iter().chain(MINOR_KEYS.iter()) {
            let camelot = musical_to_camelot(key).unwrap();
            assert_eq!(camelot_to_musical(&camelot).as_deref(), Some(*key));
        }
    }

    #[test]
    fn format_key_modes() {
        assert_eq!(format_key("C", KeyDisplayMode::Camelot), "8B");
        assert_eq!(format_key("8A", KeyDisplayMode::Musical), "Am");
        assert_eq!(format_key("C", KeyDisplayMode::Musical), "C");
        assert_eq!(format_key("", KeyDisplayMode::Camelot), "");
    }

    #[test]
    fn camelot_code_matches_string_parser() {
        assert_eq!(camelot_code_to_musical(8, false).as_deref(), Some("C"));
        assert_eq!(camelot_code_to_musical(8, true).as_deref(), Some("Am"));
    }

    #[test]
    fn canonical_key_folds_flats_and_camelot() {
        assert_eq!(canonical_key("Bb").as_deref(), Some("A#"));
        assert_eq!(canonical_key("Bbm").as_deref(), Some("A#m"));
        assert_eq!(canonical_key("Eb").as_deref(), Some("D#"));
        assert_eq!(canonical_key(" gb ").as_deref(), Some("F#"));
        assert_eq!(canonical_key("bm").as_deref(), Some("Bm"));
        assert_eq!(canonical_key("8A").as_deref(), Some("Am"));
        assert_eq!(canonical_key("12b").as_deref(), Some("E"));
        assert_eq!(canonical_key("Am").as_deref(), Some("Am"));
        assert_eq!(canonical_key(""), None);
        assert_eq!(canonical_key("H"), None);
    }

    #[test]
    fn harmonic_family_of_eight_a() {
        assert_eq!(
            harmonic_family("8A"),
            Some(vec![
                "Am".to_string(),
                "Dm".to_string(),
                "Em".to_string(),
                "C".to_string(),
            ])
        );
        assert_eq!(harmonic_family("nonsense"), None);
        // Wraps around the wheel and stays four long.
        assert_eq!(harmonic_family("1A").unwrap(), ["G#m", "C#m", "D#m", "B"]);
    }
}
