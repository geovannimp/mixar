//! Parser for the legacy `Serato Markers_` native tag payload.
//!
//! Layout: `02 05`, a big-endian `u32` marker count (always 14), fourteen
//! 22-byte entries and a four-byte footer (the track colour). Entries 0–4 are
//! cue slots 0–4, entries 5–13 are saved-loop slots 0–8. Times and colours use
//! Serato's `serato32` bit layout; the raw frame keeps every reserved, colour,
//! lock and footer byte untouched.

use super::serato32;

/// Number of legacy entries in the fixed layout.
const ENTRY_COUNT: usize = 14;
/// Bytes per legacy entry.
const ENTRY_LEN: usize = 22;
/// Header (2 + 4) plus entries plus footer.
const TOTAL_LEN: usize = 6 + ENTRY_COUNT * ENTRY_LEN + 4;

/// A legacy cue entry (position only; labels and colours come from Markers2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LegacyPosition {
    pub(crate) position_ms: u32,
}

/// A legacy saved-loop entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LegacyLoop {
    pub(crate) in_ms: u32,
    /// `None` when the entry's end-position flag is unset.
    pub(crate) out_ms: Option<u32>,
}

/// Decoded legacy markers. `None` entries are unset and never delete a Markers2
/// slot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LegacyMarkers {
    /// Cue slots 0–4.
    pub(crate) cues: [Option<LegacyPosition>; 5],
    /// Saved-loop slots 0–8.
    pub(crate) loops: [Option<LegacyLoop>; 9],
}

pub(crate) fn parse(data: &[u8]) -> Result<LegacyMarkers, ()> {
    if data.len() < TOTAL_LEN || data[0] != 2 || data[1] != 5 {
        return Err(());
    }
    let count = u32::from_be_bytes([data[2], data[3], data[4], data[5]]) as usize;
    if count != ENTRY_COUNT {
        return Err(());
    }
    // Trailing bytes beyond the documented layout are only tolerated as padding.
    if data[TOTAL_LEN..].iter().any(|b| *b != 0) {
        return Err(());
    }

    let mut out = LegacyMarkers::default();
    for index in 0..ENTRY_COUNT {
        let entry = &data[6 + index * ENTRY_LEN..6 + (index + 1) * ENTRY_LEN];
        let start_set = flag_set(entry[0])?;
        let start_ms = serato32(&entry[1..5]);
        let end_set = flag_set(entry[5])?;
        let end_ms = serato32(&entry[6..10]);

        if index < 5 {
            if start_set {
                out.cues[index] = Some(LegacyPosition {
                    position_ms: start_ms,
                });
            }
        } else if start_set {
            out.loops[index - 5] = Some(LegacyLoop {
                in_ms: start_ms,
                out_ms: end_set.then_some(end_ms),
            });
        }
    }
    Ok(out)
}

/// `00` means "set", `7f` means "unset"; anything else is corruption.
fn flag_set(byte: u8) -> Result<bool, ()> {
    match byte {
        0x00 => Ok(true),
        0x7f => Ok(false),
        _ => Err(()),
    }
}
