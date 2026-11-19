//! Write-side patching of the raw Serato GEOB payloads.
//!
//! The reader keeps each recognized frame's logical payload
//! ([`SeratoPayload::frames`]); this module patches those payloads in place
//! instead of re-serializing the aggregated, provenance-free cues/loops the
//! reader also exposes. Everything the writer does not own — `BPMLOCK`, `FLIP`,
//! unknown Markers2 records, reserved/lock bytes, the legacy footer and any
//! padding — is copied through byte-for-byte.
//!
//! The rules the reader established are kept here:
//!
//! - a descriptor that is absent is never authored, so a written kind must be
//!   adopted and its frame must still exist;
//! - Markers2 and the legacy descriptor are patched together from one snapshot,
//!   because the reader lets a set legacy entry override the Markers2 position
//!   of the same slot — leaving the legacy copy stale would restore a deleted
//!   cue on the next read;
//! - cue slots 8–15 are DB-only, legacy loop slot 8 is legacy-only, and a
//!   Markers2 `COLOR` record is the only track-colour authority written here.
//!
//! Nothing in this module touches the file system or Lofty; it turns one
//! [`SeratoPayload`] plus one [`SeratoWriteSnapshot`] into the exact logical
//! payloads that must replace the owned frames.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use super::markers2::decode_base64;
use super::{
    ConstantTempoGrid, SeratoCue, SeratoFrameKind, SeratoLoop, SeratoPayload, SeratoWriteKind,
    SeratoWriteSnapshot,
};
use crate::tag_write::FieldWrite;

/// Markers2 cue/loop slots.
const MARKERS2_SLOTS: u8 = 8;
/// Legacy cue entries (slots 0–4).
const LEGACY_CUE_SLOTS: u8 = 5;
/// Legacy loop entries (slots 0–8).
const LEGACY_LOOP_SLOTS: u8 = 9;
/// Bytes per legacy entry.
const LEGACY_ENTRY_LEN: usize = 22;
/// Legacy header (magic + count) plus entries plus footer.
const LEGACY_TOTAL_LEN: usize = 6 + 14 * LEGACY_ENTRY_LEN + 4;
/// Legacy "value is set" flag byte.
const LEGACY_SET: u8 = 0x00;
/// Legacy "value is unset" flag byte and position sentinel.
const LEGACY_UNSET: u8 = 0x7F;
/// ARGB the community reference uses for a loop without its own colour.
const DEFAULT_LOOP_ARGB: u32 = 0x0027_AAE1;
/// Largest value a `serato32` three-byte position can carry.
const SERATO32_MAX: u32 = 0x00FF_FFFF;

/// One frame whose logical payload must be replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FramePatch {
    /// Which recognized frame this replaces (unique after the duplicate check).
    pub(crate) kind: SeratoFrameKind,
    /// Replacement logical payload, exactly as `SeratoFrame::payload`.
    pub(crate) payload: Vec<u8>,
}

/// What one write request would do to the file's Serato frames.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SeratoWritePlan {
    /// Replacement payloads, in [`SeratoFrameKind`] order.
    pub(crate) patches: Vec<FramePatch>,
    /// Outcome per requested kind, in [`SeratoWriteKind::ALL`] order.
    pub(crate) outcomes: Vec<(SeratoWriteKind, FieldWrite)>,
}

impl SeratoWritePlan {
    /// Whether any owned frame changes.
    pub(crate) fn wrote(&self) -> bool {
        !self.patches.is_empty()
    }

    /// Outcome for one requested kind.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn outcome(&self, kind: SeratoWriteKind) -> Option<FieldWrite> {
        self.outcomes
            .iter()
            .find(|(candidate, _)| *candidate == kind)
            .map(|(_, outcome)| *outcome)
    }
}

/// Plan the frame patches for `snapshot`.
///
/// `Err` means the whole combined write must be refused: the scan could not be
/// trusted, an owned descriptor is invalid or duplicated, or one owned record
/// is structurally unusable. A missing (or non-adopted) descriptor is not an
/// error; it reports [`FieldWrite::Refused`] for that kind instead, which is how
/// "never author a frame" survives.
pub(crate) fn plan(
    payload: &SeratoPayload,
    snapshot: &SeratoWriteSnapshot,
) -> anyhow::Result<SeratoWritePlan> {
    let mut plan = SeratoWritePlan::default();
    let requested = requested_kinds(snapshot);
    if requested.is_empty() {
        return Ok(plan);
    }

    if payload.container.is_none() {
        // The reader does not understand this container, so it cannot rewrite
        // its vendor frames; ordinary tag writes are unaffected.
        plan.outcomes = requested
            .into_iter()
            .map(|kind| (kind, FieldWrite::Unsupported))
            .collect();
        return Ok(plan);
    }
    if !payload.scan_complete {
        anyhow::bail!("Serato tag scan is incomplete; refusing to rewrite vendor frames");
    }

    let need_cues = snapshot.cues.is_some();
    let need_loops = snapshot.loops.is_some();
    let need_colour = snapshot.colour.is_some();
    let need_grid = snapshot.grid.is_some();
    let need_markers = need_cues || need_loops || need_colour;

    let markers2 = owned_frame(payload, SeratoFrameKind::Markers2, need_markers)?;
    let legacy = owned_frame(
        payload,
        SeratoFrameKind::MarkersLegacy,
        need_cues || need_loops,
    )?;
    let grid = owned_frame(payload, SeratoFrameKind::BeatGrid, need_grid)?;

    let mut cues_changed = false;
    let mut loops_changed = false;
    let mut colour_changed = false;
    let mut colour_refused = false;

    if need_markers {
        if let Some(frame) = markers2 {
            let edits = patch_markers2(&frame.payload, snapshot)?;
            if let Some(bytes) = edits.payload {
                plan.patches.push(FramePatch {
                    kind: SeratoFrameKind::Markers2,
                    payload: bytes,
                });
            }
            cues_changed |= edits.cues;
            loops_changed |= edits.loops;
            colour_changed |= edits.colour;
            colour_refused |= edits.colour_refused;
        }
    }
    if need_cues || need_loops {
        if let Some(frame) = legacy {
            let edits = patch_legacy(&frame.payload, snapshot)?;
            if let Some(bytes) = edits.payload {
                plan.patches.push(FramePatch {
                    kind: SeratoFrameKind::MarkersLegacy,
                    payload: bytes,
                });
            }
            cues_changed |= edits.cues;
            loops_changed |= edits.loops;
        }
    }

    let mut grid_changed = false;
    if need_grid {
        if let Some(frame) = grid {
            let wanted = snapshot.grid.expect("requested");
            if let Some(bytes) = patch_grid(&frame.payload, wanted)? {
                plan.patches.push(FramePatch {
                    kind: SeratoFrameKind::BeatGrid,
                    payload: bytes,
                });
                grid_changed = true;
            }
        }
    }

    for kind in requested {
        let outcome = match kind {
            SeratoWriteKind::Cues => kind_outcome(
                need_cues,
                markers2.is_some() || legacy.is_some(),
                cues_changed,
            ),
            SeratoWriteKind::Loops => kind_outcome(
                need_loops,
                markers2.is_some() || legacy.is_some(),
                loops_changed,
            ),
            SeratoWriteKind::Grid => kind_outcome(need_grid, grid.is_some(), grid_changed),
            SeratoWriteKind::Colour => {
                if need_colour && markers2.is_some() && colour_refused {
                    FieldWrite::Refused
                } else {
                    kind_outcome(need_colour, markers2.is_some(), colour_changed)
                }
            }
        };
        plan.outcomes.push((kind, outcome));
    }
    Ok(plan)
}

/// Report for one kind: no descriptor → `Refused` (never authored), changed →
/// `Written`, otherwise `Unchanged`.
fn kind_outcome(requested: bool, descriptor_present: bool, changed: bool) -> FieldWrite {
    if !requested || !descriptor_present {
        FieldWrite::Refused
    } else if changed {
        FieldWrite::Written
    } else {
        FieldWrite::Unchanged
    }
}

fn requested_kinds(snapshot: &SeratoWriteSnapshot) -> Vec<SeratoWriteKind> {
    SeratoWriteKind::ALL
        .into_iter()
        .filter(|kind| match kind {
            SeratoWriteKind::Cues => snapshot.cues.is_some(),
            SeratoWriteKind::Loops => snapshot.loops.is_some(),
            SeratoWriteKind::Grid => snapshot.grid.is_some(),
            SeratoWriteKind::Colour => snapshot.colour.is_some(),
        })
        .collect()
}

/// The one owned frame of `kind`, or a refusal error when the file carries
/// duplicates or an unreadable copy of it.
fn owned_frame(
    payload: &SeratoPayload,
    kind: SeratoFrameKind,
    needed: bool,
) -> anyhow::Result<Option<&super::SeratoFrame>> {
    if !needed {
        return Ok(None);
    }
    let mut frames = payload.frames.iter().filter(|frame| frame.kind == kind);
    let Some(frame) = frames.next() else {
        return Ok(None);
    };
    if frames.next().is_some() {
        anyhow::bail!(
            "duplicated {} frame; refusing to rewrite vendor frames",
            frame_name(kind)
        );
    }
    if frame.status != super::FrameStatus::Valid {
        anyhow::bail!(
            "{} frame is unreadable; refusing to rewrite vendor frames",
            frame_name(kind)
        );
    }
    Ok(Some(frame))
}

fn frame_name(kind: SeratoFrameKind) -> &'static str {
    match kind {
        SeratoFrameKind::Markers2 => "Serato Markers2",
        SeratoFrameKind::MarkersLegacy => "Serato Markers_",
        SeratoFrameKind::BeatGrid => "Serato BeatGrid",
        SeratoFrameKind::MikCuePoints => "CuePoints",
        SeratoFrameKind::MikKey => "Key",
    }
}

// ---------------------------------------------------------------------------
// Markers2
// ---------------------------------------------------------------------------

/// One Markers2 record, kept raw and in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawRecord {
    name: Vec<u8>,
    payload: Vec<u8>,
}

/// A decoded Markers2 payload that still knows every raw record.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawMarkers2 {
    records: Vec<RawRecord>,
    /// Length of the payload being replaced, so padding can be preserved.
    outer_len: usize,
}

/// What a Markers2 patch changed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Markers2Edits {
    cues: bool,
    loops: bool,
    colour: bool,
    /// A colour was requested but the frame has no `COLOR` record to update.
    colour_refused: bool,
    payload: Option<Vec<u8>>,
}

fn parse_markers2(payload: &[u8]) -> anyhow::Result<RawMarkers2> {
    if payload.len() < 2 || payload[0] != 1 || payload[1] != 1 {
        anyhow::bail!("Serato Markers2 payload has no 01 01 header");
    }
    let mut end = payload.len();
    while end > 2 && (payload[end - 1] == 0 || payload[end - 1].is_ascii_whitespace()) {
        end -= 1;
    }
    let decoded = decode_base64(&payload[2..end])
        .ok_or_else(|| anyhow::anyhow!("Serato Markers2 base64 could not be decoded"))?;
    if decoded.len() < 2 || decoded[0] != 1 || decoded[1] != 1 {
        anyhow::bail!("Serato Markers2 body has no 01 01 header");
    }

    let mut records = Vec::new();
    let mut pos = 2usize;
    while let Some(&first) = decoded.get(pos) {
        if first == 0 {
            // Everything after the terminator must be padding we can drop;
            // anything else is a shape this writer cannot preserve safely.
            if decoded[pos + 1..].iter().any(|byte| *byte != 0) {
                anyhow::bail!("Serato Markers2 payload has an unrepresentable tail");
            }
            break;
        }
        let Some(name_len) = decoded[pos..].iter().position(|byte| *byte == 0) else {
            anyhow::bail!("Serato Markers2 record name is unterminated");
        };
        let name = decoded[pos..pos + name_len].to_vec();
        pos += name_len + 1;
        let Some(len_bytes) = decoded.get(pos..pos + 4) else {
            anyhow::bail!("Serato Markers2 record length is truncated");
        };
        let len = u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]);
        pos += 4;
        // Distinct from the payload `end` above: this is the end of one record.
        let record_end = pos
            .checked_add(len as usize)
            .ok_or_else(|| anyhow::anyhow!("Serato Markers2 record length overflows"))?;
        let Some(record_payload) = decoded.get(pos..record_end) else {
            anyhow::bail!("Serato Markers2 record payload is truncated");
        };
        records.push(RawRecord {
            name,
            payload: record_payload.to_vec(),
        });
        pos = record_end;
    }
    Ok(RawMarkers2 {
        records,
        outer_len: payload.len(),
    })
}

/// Re-encode a Markers2 payload: `01 01`, unpadded standard base64 of the
/// `01 01`-prefixed record stream, then NUL padding up to the old allocation
/// when it fits.
fn encode_markers2(raw: &RawMarkers2, records: &[RawRecord]) -> Vec<u8> {
    let mut binary = vec![1u8, 1];
    for record in records {
        binary.extend_from_slice(&record.name);
        binary.push(0);
        binary.extend_from_slice(&(record.payload.len() as u32).to_be_bytes());
        binary.extend_from_slice(&record.payload);
    }
    binary.push(0);

    let mut text = STANDARD.encode(&binary);
    while text.ends_with('=') {
        text.pop();
    }
    let mut out = vec![1u8, 1];
    out.extend_from_slice(text.as_bytes());
    while out.len() < raw.outer_len {
        out.push(0);
    }
    out
}

fn patch_markers2(payload: &[u8], snapshot: &SeratoWriteSnapshot) -> anyhow::Result<Markers2Edits> {
    let raw = parse_markers2(payload)?;
    let mut edits = Markers2Edits::default();

    let cues = match snapshot.cues.as_ref() {
        Some(cues) => Some(cue_map(cues)?),
        None => None,
    };
    let loops = match snapshot.loops.as_ref() {
        Some(loops) => Some(loop_map(loops)?),
        None => None,
    };
    let colour = snapshot.colour;

    let mut kept: Vec<RawRecord> = Vec::with_capacity(raw.records.len());
    let mut seen_cues = std::collections::BTreeSet::new();
    let mut seen_loops = std::collections::BTreeSet::new();
    let mut seen_colour = false;

    for record in &raw.records {
        match record.name.as_slice() {
            b"CUE" if cues.is_some() => {
                let slot = record_slot(&record.payload, MARKERS2_SLOTS, "CUE")?;
                if !seen_cues.insert(slot) {
                    anyhow::bail!("duplicated Serato Markers2 CUE slot {slot}");
                }
                match cues.as_ref().and_then(|map| map.get(&slot)) {
                    Some(cue) => {
                        let updated = patch_cue_record(&record.payload, cue)?;
                        if updated != record.payload {
                            edits.cues = true;
                        }
                        kept.push(RawRecord {
                            name: record.name.clone(),
                            payload: updated,
                        });
                    }
                    None => edits.cues = true,
                }
            }
            b"LOOP" if loops.is_some() => {
                let slot = record_slot(&record.payload, MARKERS2_SLOTS, "LOOP")?;
                if !seen_loops.insert(slot) {
                    anyhow::bail!("duplicated Serato Markers2 LOOP slot {slot}");
                }
                match loops.as_ref().and_then(|map| map.get(&slot)) {
                    Some(saved) => {
                        let updated = patch_loop_record(&record.payload, saved)?;
                        if updated != record.payload {
                            edits.loops = true;
                        }
                        kept.push(RawRecord {
                            name: record.name.clone(),
                            payload: updated,
                        });
                    }
                    None => edits.loops = true,
                }
            }
            b"COLOR" if colour.is_some() => {
                if seen_colour {
                    anyhow::bail!("duplicated Serato Markers2 COLOR record");
                }
                seen_colour = true;
                match colour.expect("requested") {
                    Some(rgb) => {
                        let updated = patch_color_record(&record.payload, rgb)?;
                        if updated != record.payload {
                            edits.colour = true;
                        }
                        kept.push(RawRecord {
                            name: record.name.clone(),
                            payload: updated,
                        });
                    }
                    None => edits.colour = true,
                }
            }
            _ => kept.push(record.clone()),
        }
    }

    if let Some(cues) = cues.as_ref() {
        for (slot, cue) in cues {
            if seen_cues.contains(slot) {
                continue;
            }
            kept.push(RawRecord {
                name: b"CUE".to_vec(),
                payload: new_cue_record(cue),
            });
            edits.cues = true;
        }
    }
    if let Some(loops) = loops.as_ref() {
        for (slot, saved) in loops {
            if seen_loops.contains(slot) {
                continue;
            }
            kept.push(RawRecord {
                name: b"LOOP".to_vec(),
                payload: new_loop_record(saved),
            });
            edits.loops = true;
        }
    }
    if matches!(colour, Some(Some(_))) && !seen_colour {
        // Only an existing `COLOR` record is ours to update; a file that lost
        // it must not silently acquire one (the adoption mask may be stale).
        edits.colour_refused = true;
    }

    if edits.cues || edits.loops || edits.colour {
        edits.payload = Some(encode_markers2(&raw, &kept));
    }
    Ok(edits)
}

fn record_slot(payload: &[u8], slots: u8, name: &str) -> anyhow::Result<u8> {
    let Some(&slot) = payload.get(1) else {
        anyhow::bail!("Serato Markers2 {name} record is shorter than its slot");
    };
    if slot >= slots {
        anyhow::bail!("Serato Markers2 {name} slot {slot} is out of range");
    }
    Ok(slot)
}

fn cue_map(cues: &[SeratoCue]) -> anyhow::Result<std::collections::BTreeMap<u8, &SeratoCue>> {
    let mut map = std::collections::BTreeMap::new();
    for cue in cues.iter().filter(|cue| cue.slot < MARKERS2_SLOTS) {
        if map.insert(cue.slot, cue).is_some() {
            anyhow::bail!("duplicated requested hot-cue slot {}", cue.slot);
        }
        validate_label(cue.label.as_deref())?;
        validate_rgb(cue.color_rgb)?;
    }
    Ok(map)
}

fn loop_map(loops: &[SeratoLoop]) -> anyhow::Result<std::collections::BTreeMap<u8, &SeratoLoop>> {
    let mut map = std::collections::BTreeMap::new();
    // Slot 8 exists only in the legacy descriptor, so dropping it here cannot
    // clear a Markers2 slot: every Markers2 `LOOP` record is slot 0–7, and a DB
    // row for such a slot is always in this map (adoption imported it) unless
    // the user deleted it, which is exactly the clear this map expresses.
    for saved in loops.iter().filter(|saved| saved.slot < MARKERS2_SLOTS) {
        if map.insert(saved.slot, saved).is_some() {
            anyhow::bail!("duplicated requested loop slot {}", saved.slot);
        }
        validate_label(saved.label.as_deref())?;
        validate_rgb(saved.color_rgb)?;
        if saved.out_ms <= saved.in_ms {
            anyhow::bail!(
                "loop slot {} ends before it starts ({} → {})",
                saved.slot,
                saved.in_ms,
                saved.out_ms
            );
        }
    }
    Ok(map)
}

fn validate_label(label: Option<&str>) -> anyhow::Result<()> {
    if let Some(label) = label {
        if label.contains('\0') {
            anyhow::bail!("label contains a NUL byte");
        }
    }
    Ok(())
}

fn validate_rgb(rgb: Option<u32>) -> anyhow::Result<()> {
    if let Some(rgb) = rgb {
        if rgb > 0x00FF_FFFF {
            anyhow::bail!("colour {rgb:#08x} is outside the 24-bit range");
        }
    }
    Ok(())
}

/// Patch a `CUE` record: position, colour and name change; the reserved bytes
/// at 0, 6, 10 and 11 and anything after the name terminator are preserved.
fn patch_cue_record(existing: &[u8], cue: &SeratoCue) -> anyhow::Result<Vec<u8>> {
    if existing.len() < 13 {
        anyhow::bail!("Serato Markers2 CUE record is too short");
    }
    let mut out = existing.to_vec();
    out[2..6].copy_from_slice(&cue.position_ms.to_be_bytes());
    let rgb = cue.color_rgb.unwrap_or(0);
    out[7] = (rgb >> 16) as u8;
    out[8] = (rgb >> 8) as u8;
    out[9] = rgb as u8;
    out.truncate(12);
    out.extend_from_slice(cue.label.as_deref().unwrap_or("").as_bytes());
    out.push(0);
    append_name_suffix(&mut out, existing, 12);
    Ok(out)
}

/// Canonical 12-byte prefix for a new `CUE` record.
fn new_cue_record(cue: &SeratoCue) -> Vec<u8> {
    let rgb = cue.color_rgb.unwrap_or(0);
    let mut out = Vec::with_capacity(13 + cue.label.as_deref().map_or(0, str::len));
    out.push(0);
    out.push(cue.slot);
    out.extend_from_slice(&cue.position_ms.to_be_bytes());
    out.push(0);
    out.push((rgb >> 16) as u8);
    out.push((rgb >> 8) as u8);
    out.push(rgb as u8);
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(cue.label.as_deref().unwrap_or("").as_bytes());
    out.push(0);
    out
}

/// Patch a `LOOP` record: bounds, colour and name change; the reserved bytes at
/// 0 and 10–13 and the lock byte at 19 are preserved, as is anything after the
/// name terminator.
fn patch_loop_record(existing: &[u8], saved: &SeratoLoop) -> anyhow::Result<Vec<u8>> {
    if existing.len() < 20 {
        anyhow::bail!("Serato Markers2 LOOP record is too short");
    }
    let mut out = existing.to_vec();
    out[2..6].copy_from_slice(&saved.in_ms.to_be_bytes());
    out[6..10].copy_from_slice(&saved.out_ms.to_be_bytes());
    if let Some(rgb) = saved.color_rgb {
        // The RGB is ours; the alpha byte in front of it is not. Serato's own
        // default (`00 27 AA E1`) is transparent, so rewriting it would change
        // a byte nobody asked about.
        out[15] = (rgb >> 16) as u8;
        out[16] = (rgb >> 8) as u8;
        out[17] = rgb as u8;
    }
    out.truncate(20);
    out.extend_from_slice(saved.label.as_deref().unwrap_or("").as_bytes());
    out.push(0);
    append_name_suffix(&mut out, existing, 20);
    Ok(out)
}

/// Canonical 20-byte prefix for a new `LOOP` record.
fn new_loop_record(saved: &SeratoLoop) -> Vec<u8> {
    let argb = match saved.color_rgb {
        Some(rgb) => 0xFF00_0000 | (rgb & 0x00FF_FFFF),
        None => DEFAULT_LOOP_ARGB,
    };
    let mut out = Vec::with_capacity(21 + saved.label.as_deref().map_or(0, str::len));
    out.push(0);
    out.push(saved.slot);
    out.extend_from_slice(&saved.in_ms.to_be_bytes());
    out.extend_from_slice(&saved.out_ms.to_be_bytes());
    out.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    out.extend_from_slice(&argb.to_be_bytes());
    out.push(0);
    out.push(0);
    out.extend_from_slice(saved.label.as_deref().unwrap_or("").as_bytes());
    out.push(0);
    out
}

/// Copy whatever followed the original record's name terminator.
fn append_name_suffix(out: &mut Vec<u8>, existing: &[u8], name_start: usize) {
    let Some(rest) = existing.get(name_start..) else {
        return;
    };
    if let Some(end) = rest.iter().position(|byte| *byte == 0) {
        out.extend_from_slice(&rest[end + 1..]);
    }
}

fn patch_color_record(existing: &[u8], rgb: u32) -> anyhow::Result<Vec<u8>> {
    if existing.len() < 4 {
        anyhow::bail!("Serato Markers2 COLOR record is too short");
    }
    let mut out = existing.to_vec();
    out[1] = (rgb >> 16) as u8;
    out[2] = (rgb >> 8) as u8;
    out[3] = rgb as u8;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Legacy markers
// ---------------------------------------------------------------------------

/// What a legacy patch changed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct LegacyEdits {
    cues: bool,
    loops: bool,
    payload: Option<Vec<u8>>,
}

fn patch_legacy(payload: &[u8], snapshot: &SeratoWriteSnapshot) -> anyhow::Result<LegacyEdits> {
    if payload.len() < LEGACY_TOTAL_LEN || payload[0] != 2 || payload[1] != 5 {
        anyhow::bail!("legacy markers payload has no 02 05 header");
    }
    let count = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]);
    if count != 14 {
        anyhow::bail!("legacy markers payload declares {count} entries, expected 14");
    }
    if payload[LEGACY_TOTAL_LEN..].iter().any(|byte| *byte != 0) {
        anyhow::bail!("legacy markers payload has an unrepresentable tail");
    }

    let mut out = payload.to_vec();
    let mut edits = LegacyEdits::default();

    if let Some(cues) = snapshot.cues.as_ref() {
        let mut map = std::collections::BTreeMap::new();
        for cue in cues.iter().filter(|cue| cue.slot < LEGACY_CUE_SLOTS) {
            if map.insert(cue.slot, cue).is_some() {
                anyhow::bail!("duplicated requested hot-cue slot {}", cue.slot);
            }
        }
        for slot in 0..LEGACY_CUE_SLOTS {
            let entry = 6 + usize::from(slot) * LEGACY_ENTRY_LEN;
            let desired = map.get(&slot).map(|cue| cue.position_ms);
            if desired.is_some_and(|ms| ms > SERATO32_MAX) {
                anyhow::bail!("cue slot {slot} position does not fit the legacy markers layout");
            }
            let wrote = write_legacy_value(&mut out, entry, desired);
            edits.cues |= wrote;
        }
    }

    if let Some(loops) = snapshot.loops.as_ref() {
        let mut map = std::collections::BTreeMap::new();
        for saved in loops.iter().filter(|saved| saved.slot < LEGACY_LOOP_SLOTS) {
            if map.insert(saved.slot, saved).is_some() {
                anyhow::bail!("duplicated requested loop slot {}", saved.slot);
            }
            if saved.out_ms <= saved.in_ms {
                anyhow::bail!(
                    "loop slot {} ends before it starts ({} → {})",
                    saved.slot,
                    saved.in_ms,
                    saved.out_ms
                );
            }
        }
        for slot in 0..LEGACY_LOOP_SLOTS {
            let entry = 6 + (usize::from(slot) + usize::from(LEGACY_CUE_SLOTS)) * LEGACY_ENTRY_LEN;
            let desired = map.get(&slot);
            if desired
                .is_some_and(|saved| saved.in_ms > SERATO32_MAX || saved.out_ms > SERATO32_MAX)
            {
                anyhow::bail!("loop slot {slot} bounds do not fit the legacy markers layout");
            }
            let (start, end) = match desired {
                Some(saved) => (Some(saved.in_ms), Some(saved.out_ms)),
                None => (None, None),
            };
            edits.loops |= write_legacy_value(&mut out, entry, start);
            edits.loops |= write_legacy_value(&mut out, entry + 5, end);
        }
    }

    if edits.cues || edits.loops {
        edits.payload = Some(out);
    }
    Ok(edits)
}

/// Write one legacy start/end value slot (flag byte plus four `serato32` bytes).
/// Returns whether anything changed.
fn write_legacy_value(entry: &mut [u8], offset: usize, value: Option<u32>) -> bool {
    let before = [
        entry[offset],
        entry[offset + 1],
        entry[offset + 2],
        entry[offset + 3],
        entry[offset + 4],
    ];
    match value {
        Some(ms) => {
            entry[offset] = LEGACY_SET;
            entry[offset + 1..offset + 5].copy_from_slice(&serato32_encode(ms));
        }
        None => {
            entry[offset] = LEGACY_UNSET;
            entry[offset + 1..offset + 5].fill(LEGACY_UNSET);
        }
    }
    before
        != [
            entry[offset],
            entry[offset + 1],
            entry[offset + 2],
            entry[offset + 3],
            entry[offset + 4],
        ]
}

/// Inverse of [`super::serato32`]: spread a 24-bit value over four bytes with
/// the top bits of each byte unused.
fn serato32_encode(value: u32) -> [u8; 4] {
    let value = value & SERATO32_MAX;
    let a = ((value >> 16) & 0xFF) as u8;
    let b = ((value >> 8) & 0xFF) as u8;
    let c = (value & 0xFF) as u8;
    [
        (a >> 5) & 0x07,
        (((a & 0x1F) << 2) | (b >> 6)) & 0x7F,
        (((b & 0x3F) << 1) | (c >> 7)) & 0x7F,
        c & 0x7F,
    ]
}

// ---------------------------------------------------------------------------
// BeatGrid
// ---------------------------------------------------------------------------

/// Rewrite the grid as one terminal marker, keeping the original footer byte.
///
/// Returns `Ok(None)` when the file already carries exactly that marker, which
/// includes an imported anchor whose milliseconds the DB rounded.
fn patch_grid(payload: &[u8], grid: ConstantTempoGrid) -> anyhow::Result<Option<Vec<u8>>> {
    if payload.len() < 7 || payload[0] != 1 || payload[1] != 0 {
        anyhow::bail!("Serato BeatGrid payload has no 01 00 header");
    }
    let count = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]) as usize;
    let body_len = count
        .checked_mul(8)
        .and_then(|n| n.checked_add(6))
        .ok_or_else(|| anyhow::anyhow!("Serato BeatGrid marker count overflows"))?;
    let total_len = body_len
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("Serato BeatGrid marker count overflows"))?;
    if count == 0 || payload.len() < total_len {
        anyhow::bail!("Serato BeatGrid payload is truncated");
    }
    if payload[total_len..].iter().any(|byte| *byte != 0) {
        anyhow::bail!("Serato BeatGrid payload has an unrepresentable tail");
    }
    let footer = payload[total_len - 1];

    if grid.first_beat_ms < 0 {
        anyhow::bail!("grid first beat {} ms is negative", grid.first_beat_ms);
    }
    let position_seconds = grid.first_beat_ms as f64 / 1000.0;
    let position = position_seconds as f32;
    if !position.is_finite() || position < 0.0 {
        anyhow::bail!("grid position {position_seconds} s is not representable");
    }
    let bpm = grid.bpm as f32;
    if !bpm.is_finite() || bpm <= 0.0 {
        anyhow::bail!("grid BPM {} is not representable", grid.bpm);
    }

    // An imported anchor keeps the wire values; comparing the projected
    // milliseconds (not the raw float) is what makes a rounded DB copy a no-op.
    if count == 1 {
        let existing_position =
            f32::from_be_bytes([payload[6], payload[7], payload[8], payload[9]]);
        let existing_bpm = f32::from_be_bytes([payload[10], payload[11], payload[12], payload[13]]);
        let same_position =
            (f64::from(existing_position) * 1000.0).round() == grid.first_beat_ms as f64;
        if same_position && existing_bpm == bpm {
            return Ok(None);
        }
    }

    let mut out = Vec::with_capacity(total_len);
    out.extend_from_slice(&[1, 0]);
    out.extend_from_slice(&1u32.to_be_bytes());
    out.extend_from_slice(&position.to_be_bytes());
    out.extend_from_slice(&bpm.to_be_bytes());
    out.push(footer);
    out.extend_from_slice(&payload[total_len..]);
    if out == payload {
        return Ok(None);
    }
    Ok(Some(out))
}
