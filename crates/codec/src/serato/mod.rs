//! Read-only Serato and Mixed In Key metadata ingestion from ID3v2.3/v2.4 tags.
//!
//! This module reads the vendor metadata that Serato writes into GEOB frames
//! (`Serato Markers2`, `Serato Markers_`, `Serato BeatGrid`) and the JSON frames
//! Mixed In Key adds (`CuePoints`, `Key`) from MP3, WAV and AIFF/AIFC files.
//! It never writes to the file and never mutates the tags it reads.
//!
//! The result preserves the difference between "no such frame" (no entry in
//! [`SeratoPayload::frames`]), "present but empty" ([`FrameStatus::Valid`] with
//! no decoded rows) and "present but corrupt" ([`FrameStatus::Invalid`]), and
//! keeps every recognized frame's logical payload bytes so later write-back work
//! can round-trip them.

mod beatgrid;
mod id3;
mod markers2;
mod markers_legacy;
mod mixed_in_key;

#[cfg(test)]
mod tests;

use std::path::Path;

/// Read the Serato and Mixed In Key metadata of `path`.
///
/// Returns empty data for unsupported containers, files without an ID3 tag, and
/// unsupported ID3 versions. Structural problems inside a tag set
/// [`SeratoPayload::scan_complete`] to `false` instead of failing the read; only
/// file I/O errors are returned as `Err`.
pub fn read_file_serato(path: &Path) -> anyhow::Result<SeratoPayload> {
    let scan = id3::scan_path(path)?;
    Ok(build_payload(scan))
}

/// Read the vendor payload from an already-open handle.
///
/// The handle is read from its current position; the caller rewinds it before
/// handing it to another reader.
pub fn read_from_file(file: &mut std::fs::File) -> anyhow::Result<SeratoPayload> {
    let scan = id3::scan_reader(file)?;
    Ok(build_payload(scan))
}

/// Everything the reader could recover from one file.
#[derive(Debug, Clone, PartialEq)]
pub struct SeratoPayload {
    /// `false` when a bound was exceeded, a value was structurally invalid or a
    /// compressed/encrypted GEOB frame had to be skipped.
    pub scan_complete: bool,
    /// Recognized frames in file order, including duplicates.
    pub frames: Vec<SeratoFrame>,
    /// Cues merged from Markers2 + legacy Markers_ (or Mixed In Key fallback).
    pub cues: Vec<SeratoCue>,
    /// Saved loops merged from Markers2 + legacy Markers_.
    pub loops: Vec<SeratoLoop>,
    /// Beat grid from the first `Serato BeatGrid` frame.
    pub beatgrid: Option<SeratoBeatGrid>,
    /// Exact 24-bit `0xRRGGBB` track colour from Markers2 `COLOR`.
    pub color_rgb: Option<u32>,
    /// Markers2 `BPMLOCK` value. Reading it never locks anything.
    pub bpm_locked: Option<bool>,
    /// Mixed In Key `Key` fallback, used only when the file has no ordinary key.
    pub mik_key: Option<String>,
    /// Unknown-but-recognized Markers2 records (e.g. `FLIP`), in original order.
    pub opaque_records: Vec<OpaqueRecord>,
}

impl Default for SeratoPayload {
    fn default() -> Self {
        Self {
            scan_complete: true,
            frames: Vec::new(),
            cues: Vec::new(),
            loops: Vec::new(),
            beatgrid: None,
            color_rgb: None,
            bpm_locked: None,
            mik_key: None,
            opaque_records: Vec::new(),
        }
    }
}

/// Which vendor frame a [`SeratoFrame`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeratoFrameKind {
    Markers2,
    MarkersLegacy,
    BeatGrid,
    MikCuePoints,
    MikKey,
}

/// Whether a recognized frame could be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameStatus {
    Valid,
    Invalid,
}

/// One recognized GEOB frame, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeratoFrame {
    pub kind: SeratoFrameKind,
    pub status: FrameStatus,
    /// Unmodified logical GEOB object data (after ID3 decoding).
    pub payload: Vec<u8>,
}

/// One cue, in its original zero-based slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeratoCue {
    pub slot: u8,
    pub position_ms: u32,
    pub label: Option<String>,
    pub color_rgb: Option<u32>,
}

/// One saved loop, in its original zero-based slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeratoLoop {
    pub slot: u8,
    pub in_ms: u32,
    pub out_ms: u32,
    pub label: Option<String>,
    pub color_rgb: Option<u32>,
}

/// A native Serato beat grid: ordered markers, the last one terminal.
#[derive(Debug, Clone, PartialEq)]
pub struct SeratoBeatGrid {
    pub markers: Vec<SeratoGridMarker>,
}

/// One beat-grid marker. Positions are seconds, exactly as stored on the wire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeratoGridMarker {
    NonTerminal {
        position_seconds: f64,
        beats_to_next: u32,
    },
    Terminal {
        position_seconds: f64,
        bpm: f64,
    },
}

/// A Markers2 record this reader does not interpret (`FLIP` and friends).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaqueRecord {
    pub record_type: Vec<u8>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Default)]
struct ParsedFrame {
    markers2: Option<markers2::Markers2Data>,
    legacy: Option<markers_legacy::LegacyMarkers>,
    grid: Option<SeratoBeatGrid>,
    mik_cues: Option<Vec<SeratoCue>>,
    mik_key: Option<String>,
}

fn build_payload(scan: id3::TagScan) -> SeratoPayload {
    let mut payload = SeratoPayload {
        scan_complete: scan.scan_complete,
        ..SeratoPayload::default()
    };

    let mut record_limit_hit = false;
    let parsed: Vec<ParsedFrame> = scan
        .frames
        .iter()
        .map(|frame| {
            let mut parsed_frame = ParsedFrame::default();
            match frame.kind {
                SeratoFrameKind::Markers2 => match markers2::parse(&frame.payload) {
                    Ok(data) => parsed_frame.markers2 = Some(data),
                    Err(markers2::Markers2Error::RecordLimit) => record_limit_hit = true,
                    Err(markers2::Markers2Error::Invalid) => {}
                },
                SeratoFrameKind::MarkersLegacy => {
                    parsed_frame.legacy = markers_legacy::parse(&frame.payload).ok()
                }
                SeratoFrameKind::BeatGrid => {
                    parsed_frame.grid = beatgrid::parse(&frame.payload).ok()
                }
                SeratoFrameKind::MikCuePoints => {
                    parsed_frame.mik_cues = mixed_in_key::parse_cue_points(&frame.payload).ok()
                }
                SeratoFrameKind::MikKey => {
                    parsed_frame.mik_key = mixed_in_key::parse_key(&frame.payload).ok()
                }
            }
            parsed_frame
        })
        .collect();

    // Exceeding a bound means the tag cannot be trusted to be complete.
    if record_limit_hit {
        payload.scan_complete = false;
    }

    for (frame, parsed_frame) in scan.frames.iter().zip(parsed.iter()) {
        payload.frames.push(SeratoFrame {
            kind: frame.kind,
            status: frame_status(frame.kind, parsed_frame),
            payload: frame.payload.clone(),
        });
    }

    // The first occurrence of each descriptor controls parsing, even when it is
    // invalid: later duplicates are retained but never override it.
    let first = |kind| first_parsed(&scan.frames, &parsed, kind);
    let selected_markers2 =
        first(SeratoFrameKind::Markers2).and_then(|parsed_frame| parsed_frame.markers2.as_ref());
    let selected_legacy =
        first(SeratoFrameKind::MarkersLegacy).and_then(|parsed_frame| parsed_frame.legacy.as_ref());
    let selected_grid = first(SeratoFrameKind::BeatGrid).and_then(|parsed| parsed.grid.clone());
    let selected_mik_cues =
        first(SeratoFrameKind::MikCuePoints).and_then(|parsed_frame| parsed_frame.mik_cues.clone());
    let selected_mik_key =
        first(SeratoFrameKind::MikKey).and_then(|parsed_frame| parsed_frame.mik_key.clone());

    let (cues, loops) = merge_markers(selected_markers2, selected_legacy);
    payload.cues = cues;
    payload.loops = loops;
    payload.beatgrid = selected_grid;
    payload.mik_key = selected_mik_key;

    if let Some(markers2) = selected_markers2 {
        payload.color_rgb = markers2.color_rgb;
        payload.bpm_locked = markers2.bpm_locked;
        payload.opaque_records = markers2.opaque.clone();
    }

    // Mixed In Key cues are a fallback only. They are used when the scan is
    // complete and the tag does not carry a Serato markers descriptor at all
    // (valid, invalid or empty — presence alone suppresses the fallback).
    let has_serato_markers = scan.frames.iter().any(|frame| {
        matches!(
            frame.kind,
            SeratoFrameKind::Markers2 | SeratoFrameKind::MarkersLegacy
        )
    });
    if payload.scan_complete && !has_serato_markers {
        if let Some(cues) = selected_mik_cues {
            payload.cues = cues;
        }
    }

    payload
}

fn frame_status(kind: SeratoFrameKind, parsed: &ParsedFrame) -> FrameStatus {
    let valid = match kind {
        SeratoFrameKind::Markers2 => parsed.markers2.is_some(),
        SeratoFrameKind::MarkersLegacy => parsed.legacy.is_some(),
        SeratoFrameKind::BeatGrid => parsed.grid.is_some(),
        SeratoFrameKind::MikCuePoints => parsed.mik_cues.is_some(),
        SeratoFrameKind::MikKey => parsed.mik_key.is_some(),
    };
    if valid {
        FrameStatus::Valid
    } else {
        FrameStatus::Invalid
    }
}

/// The first frame of `kind`, even when its payload failed to decode.
fn first_parsed<'a>(
    frames: &[id3::RawGeoBFrame],
    parsed: &'a [ParsedFrame],
    kind: SeratoFrameKind,
) -> Option<&'a ParsedFrame> {
    let index = frames.iter().position(|frame| frame.kind == kind)?;
    parsed.get(index)
}

/// Merge Markers2 and legacy markers by slot, then field.
///
/// Markers2 supplies every label and colour. A legacy entry whose start position
/// is set overrides that slot's cue position (or loop start), and a set end
/// overrides the loop end. An unset legacy entry never deletes a Markers2 slot.
fn merge_markers(
    markers2: Option<&markers2::Markers2Data>,
    legacy: Option<&markers_legacy::LegacyMarkers>,
) -> (Vec<SeratoCue>, Vec<SeratoLoop>) {
    let mut cues: std::collections::BTreeMap<u8, SeratoCue> = std::collections::BTreeMap::new();
    let mut loops: std::collections::BTreeMap<u8, SeratoLoop> = std::collections::BTreeMap::new();

    if let Some(markers2) = markers2 {
        for cue in &markers2.cues {
            cues.insert(cue.slot, cue.clone());
        }
        for saved in &markers2.loops {
            loops.insert(saved.slot, saved.clone());
        }
    }

    if let Some(legacy) = legacy {
        for (index, entry) in legacy.cues.iter().enumerate() {
            let Some(entry) = entry else { continue };
            let slot = index as u8;
            match cues.get_mut(&slot) {
                Some(cue) => cue.position_ms = entry.position_ms,
                None => {
                    cues.insert(
                        slot,
                        SeratoCue {
                            slot,
                            position_ms: entry.position_ms,
                            label: None,
                            color_rgb: None,
                        },
                    );
                }
            }
        }
        for (index, entry) in legacy.loops.iter().enumerate() {
            let Some(entry) = entry else { continue };
            let slot = index as u8;
            match loops.get_mut(&slot) {
                Some(saved) => {
                    saved.in_ms = entry.in_ms;
                    if let Some(out_ms) = entry.out_ms {
                        saved.out_ms = out_ms;
                    }
                }
                None => {
                    // A legacy-only loop needs both bounds to be usable.
                    if let Some(out_ms) = entry.out_ms {
                        loops.insert(
                            slot,
                            SeratoLoop {
                                slot,
                                in_ms: entry.in_ms,
                                out_ms,
                                label: None,
                                color_rgb: None,
                            },
                        );
                    }
                }
            }
        }
    }

    loops.retain(|_, saved| saved.out_ms > saved.in_ms);
    (cues.into_values().collect(), loops.into_values().collect())
}

/// Decode a `serato32`-encoded three-byte value (bits are spread over four
/// bytes with unused bits in between).
pub(crate) fn serato32(bytes: &[u8]) -> u32 {
    let (w, x, y, z) = (bytes[0], bytes[1], bytes[2], bytes[3]);
    let c = (z & 0x7F) | ((y & 0x01) << 7);
    let b = ((y & 0x7F) >> 1) | ((x & 0x03) << 6);
    let a = ((x & 0x7F) >> 2) | ((w & 0x07) << 5);
    (u32::from(a) << 16) | (u32::from(b) << 8) | u32::from(c)
}
