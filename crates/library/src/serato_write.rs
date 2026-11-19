//! Adopted Serato state as the writer's patching contract (#548).
//!
//! The writer itself lives in `codec` ([`codec::write_track_payload`]); this
//! module is the library boundary that decides *what* should be mirrored into
//! the file. It is deliberately conservative:
//!
//! - only a file track with a successful adoption marker and the matching
//!   adopted-kind bit is projected, so an unadopted (or zero-mask) file never
//!   gains a marker frame and never looks like a deliberate clear;
//! - cue slots 8–15 are DB-only, automatic loop memory (`auto_saved`) is not an
//!   explicit saved loop, and a multi-segment grid has no native single-marker
//!   projection, so those stay in the database;
//! - every projected value is validated instead of truncated: an out-of-range
//!   position, inverted loop, 24-bit colour or NUL label refuses the write and
//!   leaves both the file and the row alone.
//!
//! Reads only — no file I/O and no lock is held by callers of [`snapshot`]
//! beyond the manager reference they already own.

use codec::serato::{ConstantTempoGrid, SeratoCue, SeratoLoop, SeratoWriteSnapshot};
use library_core::{Library, LibraryError, Result, TrackId};

use crate::beat_grid;
use crate::serato_adoption::{
    SERATO_KIND_COLOUR, SERATO_KIND_CUES, SERATO_KIND_GRID, SERATO_KIND_LOOPS,
};
use crate::LibraryManager;

/// Slot accents the pad grid paints for a hot cue without its own colour
/// (`apps/gui-flutter/lib/widgets/mixer/pads/pad_button.dart::hotCueAccent`),
/// so a mirrored cue keeps the colour the user sees.
const HOT_CUE_ACCENTS: [u32; 8] = [
    0xEF_4444, // red-500
    0xF9_7316, // orange-500
    0xEA_B308, // yellow-500
    0x22_C55E, // green-500
    0x06_B6D4, // cyan-500
    0x3B_82F6, // blue-500
    0x8B_5CF6, // violet-500
    0xEC_4899, // pink-500
];

/// Markers2 cue/loop slots; slots 8–15 of the DB are not representable.
const MARKERS2_SLOTS: u8 = 8;
/// Legacy loop slot 8 exists only in the legacy descriptor.
const LEGACY_LAST_LOOP_SLOT: u8 = 8;

/// Build the Serato write snapshot for `id`, or `None` when there is nothing to
/// mirror.
///
/// `None` means "leave the vendor frames alone": write-back is off, the track
/// is a stream/unknown id, it never adopted vendor metadata, the mask has no
/// requested kind, or every adopted kind already projects to nothing. A missing
/// adoption marker or a zero mask is **not** an error and never disables the
/// ordinary tag write.
pub(crate) fn snapshot(lib: &LibraryManager, id: &TrackId) -> Result<Option<SeratoWriteSnapshot>> {
    if !lib.config().tag_write_back.is_on() {
        return Ok(None);
    }
    let Some(source) = lib.get_track(id)? else {
        return Ok(None);
    };
    if source.file().is_none() {
        return Ok(None);
    }
    let state = match lib.serato_adoption_state(id) {
        Ok(state) => state,
        Err(err) => {
            // An unknown/deleted row or a corrupt mask is a skipped write, never
            // a reason to touch the file or change adoption.
            tracing::warn!(
                track_id = %id,
                error = %err,
                "Serato write-back skipped"
            );
            return Ok(None);
        }
    };
    if state.imported_at.is_none() {
        return Ok(None);
    }
    let mask = state.adopted_kinds;
    let metadata = source.metadata();
    let mut snapshot = SeratoWriteSnapshot::default();

    if mask & SERATO_KIND_CUES != 0 {
        let mut cues = Vec::new();
        for row in lib.list_track_hot_cues(id)? {
            if row.slot_index >= MARKERS2_SLOTS {
                // DB-only slots (8–15); the native CUE record cannot carry them.
                continue;
            }
            let position_ms = nonnegative_ms(row.position_ms, "hot-cue position", row.slot_index)?;
            let color_rgb =
                parse_hex_color(row.color.as_deref()).unwrap_or_else(|| palette(row.slot_index));
            cues.push(SeratoCue {
                slot: row.slot_index,
                position_ms,
                label: label(row.label)?,
                color_rgb: Some(color_rgb),
            });
        }
        snapshot.cues = Some(cues);
    }

    if mask & SERATO_KIND_LOOPS != 0 {
        let mut loops = Vec::new();
        for row in lib.list_track_loops(id)? {
            // Automatic recall is not an explicit saved loop; exporting it would
            // create slots the user never saved.
            if row.auto_saved || row.slot_index > LEGACY_LAST_LOOP_SLOT {
                continue;
            }
            let in_ms = nonnegative_ms(row.in_ms, "loop start", row.slot_index)?;
            let out_ms = nonnegative_ms(row.out_ms, "loop end", row.slot_index)?;
            if out_ms <= in_ms {
                return Err(write_back_err(format!(
                    "loop slot {} ends before it starts ({in_ms} → {out_ms})",
                    row.slot_index
                )));
            }
            loops.push(SeratoLoop {
                slot: row.slot_index,
                in_ms,
                out_ms,
                label: label(row.label)?,
                // A loop without its own colour keeps whatever ARGB the file
                // already carries; the cue palette is not a loop authority.
                color_rgb: parse_hex_color(row.color.as_deref()),
            });
        }
        snapshot.loops = Some(loops);
    }

    if mask & SERATO_KIND_GRID != 0 {
        // Only one canonical segment has a native single-marker projection. A
        // still-multi-segment grid (an imported variable-tempo file) is kept.
        if let Some(grid) = beat_grid::get_track_beat_grid(&lib.db, id)? {
            if let [segment] = grid.segments.as_slice() {
                snapshot.grid = Some(ConstantTempoGrid {
                    first_beat_ms: segment.start_time_ms,
                    bpm: segment.bpm,
                });
            }
        }
    }

    if mask & SERATO_KIND_COLOUR != 0 {
        // The row's exact RGB is authoritative; a palette-only legacy row uses
        // its own swatch, exactly like the store's colour coherence rule.
        //
        // `None` means the user cleared the colour: the store treats a cleared
        // editor value as a decision (never as "missing data"), and adoption
        // never clears an adopted colour, so no unrelated path can reach here
        // with an accidentally empty value. The codec turns it into "remove the
        // Markers2 COLOR record", and does nothing when there is no record to
        // remove, so repeated triggers stay no-ops.
        let rgb = metadata
            .color_rgb
            .filter(|value| *value <= 0x00FF_FFFF)
            .or_else(|| metadata.color_label.map(library_core::ColorLabel::rgb));
        snapshot.colour = Some(rgb);
    }

    if snapshot.is_empty() {
        return Ok(None);
    }
    Ok(Some(snapshot))
}

/// A stored millisecond value that a native frame can carry.
fn nonnegative_ms(value: i32, what: &str, slot: u8) -> Result<u32> {
    u32::try_from(value)
        .map_err(|_| write_back_err(format!("{what} of slot {slot} is negative ({value} ms)")))
}

/// A label with no embedded NUL (which the NUL-terminated wire form cannot
/// carry). An empty label is "no label", matching the reader.
fn label(value: Option<String>) -> Result<Option<String>> {
    match value {
        None => Ok(None),
        Some(text) if text.is_empty() => Ok(None),
        Some(text) => {
            if text.contains('\0') {
                return Err(write_back_err("label contains a NUL byte".to_string()));
            }
            Ok(Some(text))
        }
    }
}

/// Parse a stored `#RRGGBB` value; anything else is "no stored colour".
fn parse_hex_color(raw: Option<&str>) -> Option<u32> {
    let digits = raw?.trim();
    let digits = digits.strip_prefix('#').unwrap_or(digits);
    if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16)
        .ok()
        .filter(|value| *value <= 0x00FF_FFFF)
}

/// The pad accent a cue slot falls back to when it stores no colour.
fn palette(slot: u8) -> u32 {
    HOT_CUE_ACCENTS[usize::from(slot % MARKERS2_SLOTS)]
}

fn write_back_err(message: String) -> LibraryError {
    LibraryError::Backend {
        backend: "library",
        message: format!("Serato write-back refused: {message}"),
    }
}

#[cfg(test)]
mod tests;
