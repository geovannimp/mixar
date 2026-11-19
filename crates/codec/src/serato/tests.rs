//! Inline tests for the Serato/Mixed In Key reader.
//!
//! Every vector here is either a vendored upstream fixture (see
//! `fixtures/provenance.json`) or a hand-built ID3/container byte sequence for
//! a structurally interesting case. No test needs network access, a Serato
//! installation or the private acceptance sample.

use std::fs;
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use sha2::{Digest, Sha256};

use super::*;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/serato/fixtures");
const TEA_LEAVES_SHA256: &str = "c568221984ddb6b37c9fbe6711b97f89fc61778d0d26a8fbe9748b098da0d376";

fn fixture(rel: &str) -> Vec<u8> {
    fs::read(Path::new(FIXTURES).join(rel)).expect("fixture bytes")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

struct TempFile {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl TempFile {
    fn read_payload(&self) -> SeratoPayload {
        read_file_serato(&self.path).expect("read serato payload")
    }
}

fn temp_file(name: &str, bytes: &[u8]) -> TempFile {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("write temp file");
    TempFile { _dir: dir, path }
}

fn payload_of(name: &str, bytes: &[u8]) -> SeratoPayload {
    temp_file(name, bytes).read_payload()
}

// ---------------------------------------------------------------------------
// ID3 and container builders
// ---------------------------------------------------------------------------

fn syncsafe_bytes(len: usize) -> [u8; 4] {
    assert!(len < (1 << 28), "syncsafe value out of range");
    [
        ((len >> 21) & 0x7F) as u8,
        ((len >> 14) & 0x7F) as u8,
        ((len >> 7) & 0x7F) as u8,
        (len & 0x7F) as u8,
    ]
}

fn frame(version: u8, id: &[u8; 4], flags: u16, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(id);
    if version == 4 {
        out.extend_from_slice(&syncsafe_bytes(data.len()));
    } else {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    }
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(data);
    out
}

fn id3_tag(version: u8, flags: u8, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"ID3");
    out.push(version);
    out.push(0);
    out.push(flags);
    out.extend_from_slice(&syncsafe_bytes(body.len()));
    out.extend_from_slice(body);
    out
}

fn tag_with(version: u8, frames: &[Vec<u8>], padding: usize) -> Vec<u8> {
    let mut body: Vec<u8> = frames.concat();
    body.resize(body.len() + padding, 0);
    id3_tag(version, 0, &body)
}

/// A v2.4 tag holding exactly one GEOB frame — the shape most cases need.
fn single_geob(description: &str, object: &[u8]) -> Vec<u8> {
    tag_with(4, &[geob_frame(4, description, object)], 0)
}

/// Encode `text` for GEOB descriptions/filenames. Encoding 1 is BOM-qualified
/// UTF-16LE, encoding 2 is UTF-16BE, 0 is Latin-1 and 3 is UTF-8.
fn encode_text(encoding: u8, text: &str) -> Vec<u8> {
    match encoding {
        0 => text.bytes().collect(),
        3 => text.as_bytes().to_vec(),
        1 => {
            let mut out = vec![0xFF, 0xFE];
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_le_bytes());
            }
            out
        }
        _ => {
            let mut out = Vec::new();
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_be_bytes());
            }
            out
        }
    }
}

fn text_terminator(encoding: u8) -> &'static [u8] {
    if encoding == 1 || encoding == 2 {
        &[0, 0]
    } else {
        &[0]
    }
}

fn geob_body(encoding: u8, description: &str, object: &[u8]) -> Vec<u8> {
    let mut data = vec![encoding];
    data.extend_from_slice(b"application/octet-stream");
    data.push(0);
    data.extend_from_slice(text_terminator(encoding)); // empty filename
    data.extend_from_slice(&encode_text(encoding, description));
    data.extend_from_slice(text_terminator(encoding));
    data.extend_from_slice(object);
    data
}

fn geob_frame(version: u8, description: &str, object: &[u8]) -> Vec<u8> {
    frame(version, b"GEOB", 0, &geob_body(0, description, object))
}

fn uint32(value: u32) -> [u8; 4] {
    value.to_be_bytes()
}

/// `serato32` encoding, the inverse of the reader's decoder, used only for
/// building inputs; golden vectors below check the decoder against raw bytes.
fn serato32_encode(value: u32) -> [u8; 4] {
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

/// Build a logical `Serato Markers2` payload from raw records.
fn markers2_payload(records: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut binary = vec![1u8, 1];
    for (name, payload) in records {
        binary.extend_from_slice(name.as_bytes());
        binary.push(0);
        binary.extend_from_slice(&uint32(payload.len() as u32));
        binary.extend_from_slice(payload);
    }
    binary.push(0);

    let mut out = vec![1u8, 1];
    out.extend_from_slice(B64.encode(binary).as_bytes());
    out
}

fn cue_record(slot: u8, position_ms: u32, rgb: [u8; 3], name: &str) -> Vec<u8> {
    let mut out = vec![0, slot];
    out.extend_from_slice(&uint32(position_ms));
    out.push(0);
    out.extend_from_slice(&rgb);
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out
}

fn loop_record(
    slot: u8,
    start_ms: u32,
    end_ms: u32,
    rgb: [u8; 3],
    lock: u8,
    name: &str,
) -> Vec<u8> {
    let mut out = vec![0, slot];
    out.extend_from_slice(&uint32(start_ms));
    out.extend_from_slice(&uint32(end_ms));
    out.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
    out.extend_from_slice(&[0, rgb[0], rgb[1], rgb[2]]);
    out.extend_from_slice(&[0, lock]);
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out
}

fn color_record(rgb: [u8; 3]) -> Vec<u8> {
    vec![0, rgb[0], rgb[1], rgb[2]]
}

fn bpm_lock_record(locked: bool) -> Vec<u8> {
    vec![u8::from(locked)]
}

struct LegacyEntry {
    start: Option<u32>,
    end: Option<u32>,
    color: u32,
}

fn legacy_payload(entries: &[LegacyEntry; 14]) -> Vec<u8> {
    let mut out = vec![2u8, 5];
    out.extend_from_slice(&uint32(14));
    for entry in entries {
        match entry.start {
            Some(start) => {
                out.push(0);
                out.extend_from_slice(&serato32_encode(start));
            }
            None => {
                out.push(0x7F);
                out.extend_from_slice(&[0x7F; 4]);
            }
        }
        match entry.end {
            Some(end) => {
                out.push(0);
                out.extend_from_slice(&serato32_encode(end));
            }
            None => {
                out.push(0x7F);
                out.extend_from_slice(&[0x7F; 4]);
            }
        }
        out.extend_from_slice(&[0, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F]);
        out.extend_from_slice(&serato32_encode(entry.color));
        out.extend_from_slice(&[0, 0]);
    }
    out.extend_from_slice(&serato32_encode(0xFFFFFF));
    out
}

fn unset_legacy() -> LegacyEntry {
    LegacyEntry {
        start: None,
        end: None,
        color: 0,
    }
}

fn empty_legacy() -> [LegacyEntry; 14] {
    std::array::from_fn(|_| unset_legacy())
}

fn grid_payload(markers: &[(f32, u32)]) -> Vec<u8> {
    let mut out = vec![1u8, 0];
    out.extend_from_slice(&uint32(markers.len() as u32));
    for (index, (position, value)) in markers.iter().enumerate() {
        out.extend_from_slice(&position.to_be_bytes());
        if index + 1 == markers.len() {
            out.extend_from_slice(&value.to_be_bytes());
        } else {
            out.extend_from_slice(&uint32(*value));
        }
    }
    out.push(0x37);
    out
}

fn mik_json(value: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&value).expect("json")
}

fn mik_body(value: serde_json::Value) -> Vec<u8> {
    B64.encode(mik_json(value)).into_bytes()
}

fn cue_points_body(times: &[f64]) -> Vec<u8> {
    let cues: Vec<serde_json::Value> = times
        .iter()
        .map(|time| serde_json::json!({"time": time, "name": ""}))
        .collect();
    mik_body(serde_json::json!({
        "cues": cues,
        "source": "mixedinkey",
        "algorithm": 14
    }))
}

fn key_body(key: &str) -> Vec<u8> {
    mik_body(serde_json::json!({
        "key": key,
        "source": "mixedinkey",
        "algorithm": 94
    }))
}

fn wav_with(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut payload = Vec::new();
    for (id, data) in chunks {
        payload.extend_from_slice(*id);
        payload.extend_from_slice(&(data.len() as u32).to_le_bytes());
        payload.extend_from_slice(data);
        if data.len() % 2 == 1 {
            payload.push(0);
        }
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(payload.len() as u32 + 4).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&payload);
    out
}

fn aiff_with(form: &[u8; 4], chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut payload = Vec::new();
    for (id, data) in chunks {
        payload.extend_from_slice(*id);
        payload.extend_from_slice(&(data.len() as u32).to_be_bytes());
        payload.extend_from_slice(data);
        if data.len() % 2 == 1 {
            payload.push(0);
        }
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"FORM");
    out.extend_from_slice(&(payload.len() as u32 + 4).to_be_bytes());
    out.extend_from_slice(form);
    out.extend_from_slice(&payload);
    out
}

/// Apply ID3 unsynchronisation: insert `00` after every `FF` that is followed
/// by `00` or a byte >= `E0`, and at the end of the data.
fn unsynchronise(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for (index, byte) in data.iter().enumerate() {
        out.push(*byte);
        if *byte == 0xFF {
            let next = data.get(index + 1).copied();
            if next.is_none() || next == Some(0x00) || next.is_some_and(|b| b >= 0xE0) {
                out.push(0x00);
            }
        }
    }
    out
}

fn frames_by_kind(payload: &SeratoPayload, kind: SeratoFrameKind) -> Vec<&SeratoFrame> {
    payload.frames.iter().filter(|f| f.kind == kind).collect()
}

/// Status of the first frame of `kind`; every test that asserts a status goes
/// through this one helper.
fn status_of(payload: &SeratoPayload, kind: SeratoFrameKind) -> FrameStatus {
    frames_by_kind(payload, kind)
        .first()
        .map(|frame| frame.status)
        .expect("frame of that kind")
}

// ---------------------------------------------------------------------------
// Acceptance sample (fixture `tea-leaves.id3`)
// ---------------------------------------------------------------------------

const TEA_LEAVES_CUES: [(u8, u32, u32, &str); 8] = [
    (0, 510, 0xCC0000, "Energy 8"),
    (1, 12_661, 0xCC8800, "Energy 7"),
    (2, 30_888, 0x0000CC, "Energy 7"),
    (3, 55_190, 0xCCCC00, "Energy 7"),
    (4, 85_568, 0x00CC00, "Energy 7"),
    (5, 146_323, 0xCC00CC, "Energy 7"),
    (6, 158_475, 0x00CCCC, "Energy 7"),
    (7, 182_777, 0x8800CC, "Energy 4"),
];

fn tea_leaves() -> Vec<u8> {
    let bytes = fixture("tea-leaves.id3");
    assert_eq!(bytes.len(), 1464, "fixture size");
    assert_eq!(sha256_hex(&bytes), TEA_LEAVES_SHA256, "fixture hash");
    bytes
}

#[test]
fn tea_leaves_tag_decodes_the_full_oracle() {
    let payload = payload_of("tea-leaves.id3", &tea_leaves());

    assert!(payload.scan_complete);
    assert_eq!(payload.frames.len(), 5, "only the five vendor frames");
    assert_eq!(
        payload
            .frames
            .iter()
            .map(|f| (f.kind, f.status))
            .collect::<Vec<_>>(),
        vec![
            (SeratoFrameKind::MarkersLegacy, FrameStatus::Valid),
            (SeratoFrameKind::Markers2, FrameStatus::Valid),
            (SeratoFrameKind::BeatGrid, FrameStatus::Valid),
            (SeratoFrameKind::MikCuePoints, FrameStatus::Valid),
            (SeratoFrameKind::MikKey, FrameStatus::Valid),
        ]
    );

    let cues: Vec<(u8, u32, Option<u32>, Option<&str>)> = payload
        .cues
        .iter()
        .map(|cue| {
            (
                cue.slot,
                cue.position_ms,
                cue.color_rgb,
                cue.label.as_deref(),
            )
        })
        .collect();
    let expected: Vec<(u8, u32, Option<u32>, Option<&str>)> = TEA_LEAVES_CUES
        .iter()
        .map(|(slot, ms, rgb, label)| (*slot, *ms, Some(*rgb), Some(*label)))
        .collect();
    assert_eq!(cues, expected);

    assert!(payload.loops.is_empty());
    assert_eq!(payload.color_rgb, Some(0x00FF_FFFF), "track colour #FFFFFF");
    assert_eq!(payload.bpm_locked, Some(false));

    let grid = payload.beatgrid.as_ref().expect("beat grid");
    assert_eq!(grid.markers.len(), 1);
    match grid.markers[0] {
        SeratoGridMarker::Terminal {
            position_seconds,
            bpm,
        } => {
            assert_eq!(position_seconds, 3.191_995_382_308_96);
            assert_eq!(bpm, 79.0);
        }
        other => panic!("expected a terminal marker, got {other:?}"),
    }

    assert_eq!(payload.mik_key.as_deref(), Some("5A"));
    // The Mixed In Key frames stay byte-for-byte available.
    let key_frame = &frames_by_kind(&payload, SeratoFrameKind::MikKey)[0].payload;
    let key_json: serde_json::Value =
        serde_json::from_slice(&B64.decode(key_frame).expect("base64")).expect("json");
    assert_eq!(
        key_json,
        serde_json::json!({"key": "5A", "source": "mixedinkey", "algorithm": 94})
    );
    let cue_points = &frames_by_kind(&payload, SeratoFrameKind::MikCuePoints)[0].payload;
    let decoded: serde_json::Value =
        serde_json::from_slice(&B64.decode(cue_points).expect("base64")).expect("json");
    assert_eq!(decoded["source"], "mixedinkey");
    assert_eq!(decoded["algorithm"], 14);
    assert_eq!(decoded["cues"].as_array().expect("cues").len(), 8);
}

#[test]
fn tea_leaves_only_five_cues_come_from_legacy() {
    let payload = payload_of("tea-leaves.id3", &tea_leaves());
    let legacy = frames_by_kind(&payload, SeratoFrameKind::MarkersLegacy)[0]
        .payload
        .clone();
    let markers2 = frames_by_kind(&payload, SeratoFrameKind::Markers2)[0]
        .payload
        .clone();
    let legacy_only = payload_of("legacy-only.mp3", &single_geob("Serato Markers_", &legacy));
    assert_eq!(
        legacy_only
            .cues
            .iter()
            .map(|cue| cue.slot)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    assert!(legacy_only.cues.iter().all(|cue| cue.label.is_none()));
    assert!(legacy_only.cues.iter().all(|cue| cue.color_rgb.is_none()));

    let markers2_only = payload_of(
        "markers2-only.mp3",
        &single_geob("Serato Markers2", &markers2),
    );
    assert_eq!(markers2_only.cues.len(), 8);
    assert_eq!(markers2_only.cues[7].label.as_deref(), Some("Energy 4"));
}

#[test]
fn serato32_golden_vectors_match_the_native_layout() {
    // Raw bytes from the native format documentation.
    assert_eq!(serato32(&[0x00, 0x0D, 0x2A, 0x58]), 218_456);
    assert_eq!(serato32(&[0x06, 0x30, 0x00, 0x00]), 0xCC_0000);
    assert_eq!(serato32(&[0x00, 0x00, 0x03, 0x54]), 468);
    assert_eq!(serato32(&[0x00, 0x00, 0x00, 0x6C]), 108);
}

// ---------------------------------------------------------------------------
// Native marker formats and merge
// ---------------------------------------------------------------------------

#[test]
fn legacy_positions_win_while_markers2_labels_and_colours_stay() {
    let markers2 = markers2_payload(&[
        ("COLOR", color_record([0x99, 0xFF, 0x99])),
        ("CUE", cue_record(0, 1_000, [0xAA, 0x00, 0x00], "Intro")),
        ("CUE", cue_record(1, 2_000, [0x00, 0xAA, 0x00], "Drop")),
        ("BPMLOCK", bpm_lock_record(false)),
    ]);
    let mut entries = empty_legacy();
    // Entry 1 is set and wins that slot's position; entry 0 is unset and must
    // not delete the Markers2 cue in slot 0.
    entries[1] = LegacyEntry {
        start: Some(5_000),
        end: None,
        color: 0,
    };
    let legacy = legacy_payload(&entries);

    let payload = payload_of(
        "merge.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers_", &legacy),
                geob_frame(4, "Serato Markers2", &markers2),
            ],
            0,
        ),
    );

    assert_eq!(payload.cues.len(), 2);
    assert_eq!(payload.cues[0].slot, 0);
    assert_eq!(
        payload.cues[0].position_ms, 1_000,
        "unset legacy is a no-op"
    );
    assert_eq!(payload.cues[0].label.as_deref(), Some("Intro"));
    assert_eq!(payload.cues[0].color_rgb, Some(0xAA_0000));
    assert_eq!(payload.cues[1].slot, 1);
    assert_eq!(payload.cues[1].position_ms, 5_000, "legacy start wins");
    assert_eq!(
        payload.cues[1].label.as_deref(),
        Some("Drop"),
        "the Markers2 name stays"
    );
    assert_eq!(payload.cues[1].color_rgb, Some(0x00AA00));
    assert_eq!(payload.color_rgb, Some(0x99FF99));
}

#[test]
fn markers2_slots_five_to_seven_survive_a_legacy_merge() {
    let mut records: Vec<(&str, Vec<u8>)> = vec![("COLOR", color_record([0xFF, 0xFF, 0xFF]))];
    for slot in 0..8u8 {
        records.push((
            "CUE",
            cue_record(slot, u32::from(slot) * 1_000, [0x00, 0x00, 0xCC], "cue"),
        ));
    }
    let markers2 = markers2_payload(&records);
    let mut entries = empty_legacy();
    entries[0] = LegacyEntry {
        start: Some(7_500),
        end: None,
        color: 0,
    };
    let legacy = legacy_payload(&entries);

    let payload = payload_of(
        "slots.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers_", &legacy),
                geob_frame(4, "Serato Markers2", &markers2),
            ],
            0,
        ),
    );
    assert_eq!(payload.cues.len(), 8);
    assert_eq!(payload.cues[0].position_ms, 7_500);
    assert_eq!(payload.cues[7].position_ms, 7_000);
    assert_eq!(payload.cues[7].label.as_deref(), Some("cue"));
}

#[test]
fn saved_loops_fixture_decodes() {
    let payload = payload_of(
        "saved-loops.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato Markers2",
                &fixture("markers2/saved-loops.octet-stream"),
            )],
            0,
        ),
    );
    assert_eq!(payload.loops.len(), 4);
    for (index, saved) in payload.loops.iter().enumerate() {
        assert_eq!(usize::from(saved.slot), index);
        assert_eq!(saved.in_ms, 0);
        assert_eq!(saved.out_ms, 2_086);
        assert_eq!(saved.color_rgb, Some(0x27_AAE1));
    }
    assert_eq!(payload.loops[0].label, None);
    assert_eq!(payload.loops[3].label.as_deref(), Some("Test"));
    assert_eq!(payload.color_rgb, Some(0xFF_FFFF));
    assert!(frames_by_kind(&payload, SeratoFrameKind::Markers2)[0]
        .payload
        .starts_with(b"\x01\x01"));
}

#[test]
fn legacy_saved_loops_merge_and_slot_eight_survives() {
    let markers2 = markers2_payload(&[
        ("COLOR", color_record([0xFF, 0xFF, 0xFF])),
        (
            "LOOP",
            loop_record(0, 100, 900, [0x11, 0x22, 0x33], 0, "keep"),
        ),
        ("BPMLOCK", bpm_lock_record(false)),
    ]);
    let legacy = fixture("markers_/saved-loops.octet-stream");

    let payload = payload_of(
        "legacy-loops.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers_", &legacy),
                geob_frame(4, "Serato Markers2", &markers2),
            ],
            0,
        ),
    );

    // Legacy entries 5–8 are loop slots 0–3 at 0→2086 ms.
    assert_eq!(payload.loops.len(), 4);
    for saved in &payload.loops {
        assert_eq!(saved.in_ms, 0);
        assert_eq!(saved.out_ms, 2_086);
    }
    // Slot 0 keeps Markers2's label/colour but takes the legacy bounds.
    assert_eq!(payload.loops[0].label.as_deref(), Some("keep"));
    assert_eq!(payload.loops[0].color_rgb, Some(0x11_2233));
    // Legacy-only slots carry no label/colour.
    assert_eq!(payload.loops[3].slot, 3);
    assert_eq!(payload.loops[3].label, None);
    assert_eq!(payload.loops[3].color_rgb, None);

    // A loop that is only reachable through legacy entry 13 (slot 8) survives.
    let mut entries = empty_legacy();
    entries[13] = LegacyEntry {
        start: Some(1_000),
        end: Some(4_000),
        color: 0,
    };
    let payload = payload_of(
        "legacy-slot-8.mp3",
        &tag_with(
            4,
            &[geob_frame(4, "Serato Markers_", &legacy_payload(&entries))],
            0,
        ),
    );
    assert_eq!(payload.loops.len(), 1);
    assert_eq!(payload.loops[0].slot, 8);
    assert_eq!(payload.loops[0].in_ms, 1_000);
    assert_eq!(payload.loops[0].out_ms, 4_000);
}

#[test]
fn merged_loop_with_unusable_bounds_is_dropped() {
    let markers2 = markers2_payload(&[
        ("COLOR", color_record([0xFF, 0xFF, 0xFF])),
        ("LOOP", loop_record(0, 1_000, 2_000, [0, 0, 0], 0, "")),
    ]);
    let mut entries = empty_legacy();
    // Legacy start after the Markers2 end makes the merged loop unusable.
    entries[5] = LegacyEntry {
        start: Some(5_000),
        end: None,
        color: 0,
    };
    let payload = payload_of(
        "bad-loop.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers_", &legacy_payload(&entries)),
                geob_frame(4, "Serato Markers2", &markers2),
            ],
            0,
        ),
    );
    assert!(payload.loops.is_empty());
}

#[test]
fn tracklist_colour_is_exact_not_snapped() {
    let payload = payload_of(
        "tracklist-colour.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato Markers2",
                &fixture("markers2/tracklist-color.octet-stream"),
            )],
            0,
        ),
    );
    assert_eq!(payload.color_rgb, Some(0x99_FF99));
}

#[test]
fn flip_records_are_preserved_byte_for_byte() {
    let payload = payload_of(
        "flips.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato Markers2",
                &fixture("markers2/flips.octet-stream"),
            )],
            0,
        ),
    );
    let flips: Vec<&OpaqueRecord> = payload
        .opaque_records
        .iter()
        .filter(|record| record.record_type == b"FLIP")
        .collect();
    assert_eq!(flips.len(), 3);
    assert_eq!(flips[0].payload.len(), 187);
    assert_eq!(
        sha256_hex(&flips[0].payload),
        "c8b0fd1a2842445368a14abfbb01a5c3fcd73ddb646d7361ca9cb123bf5eaac6"
    );
    assert_eq!(payload.cues.len(), 3);
    assert_eq!(payload.bpm_locked, Some(false));
}

#[test]
fn very_long_names_and_one_char_too_long_base64_decode() {
    let payload = payload_of(
        "very-long-names.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato Markers2",
                &fixture("markers2/very-long-names.octet-stream"),
            )],
            0,
        ),
    );
    assert!(payload.scan_complete);
    assert_eq!(payload.cues.len(), 8);
    assert_eq!(payload.loops.len(), 8);
    assert!(payload.cues[0]
        .label
        .as_deref()
        .is_some_and(|label| label.len() > 40));
}

#[test]
fn unpadded_and_wrapped_base64_decode() {
    let logical = fixture("markers2/hotcue-colors.octet-stream");
    // Strip padding and wrap with CRLF whitespace like real files do.
    let mut text: Vec<u8> = logical[2..].to_vec();
    while text.last() == Some(&b'=') {
        text.pop();
    }
    let mut wrapped = vec![1u8, 1];
    for (index, byte) in text.iter().enumerate() {
        if index > 0 && index % 72 == 0 {
            wrapped.extend_from_slice(b"\r\n");
        }
        wrapped.push(*byte);
    }
    wrapped.extend_from_slice(&[0u8; 16]);

    let payload = payload_of("unpadded.mp3", &single_geob("Serato Markers2", &wrapped));
    assert_eq!(payload.cues.len(), 8);
    assert_eq!(payload.cues[0].color_rgb, Some(0xCC_0000));
}

// ---------------------------------------------------------------------------
// Beat grid
// ---------------------------------------------------------------------------

#[test]
fn changing_tempo_grid_decodes_exactly() {
    let payload = payload_of(
        "changing-tempo.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato BeatGrid",
                &fixture("beatgrid/manual-beatgrid-changing-tempo.octet-stream"),
            )],
            0,
        ),
    );
    let grid = payload.beatgrid.expect("grid");
    assert_eq!(grid.markers.len(), 8);
    let mut starts = Vec::new();
    let mut counts = Vec::new();
    for marker in &grid.markers {
        match *marker {
            SeratoGridMarker::NonTerminal {
                position_seconds,
                beats_to_next,
            } => {
                starts.push(position_seconds);
                counts.push(beats_to_next);
            }
            SeratoGridMarker::Terminal {
                position_seconds,
                bpm,
            } => {
                starts.push(position_seconds);
                assert_eq!(bpm, 116.995_559_692_382_81);
            }
        }
    }
    assert_eq!(
        starts
            .iter()
            .map(|s| (s * 1000.0).round() as i64)
            .collect::<Vec<_>>(),
        vec![71, 16_470, 36_892, 239_980, 244_096, 301_660, 350_927, 424_703]
    );
    assert_eq!(counts, vec![32, 40, 396, 8, 112, 96, 144]);
}

#[test]
fn analyzed_grid_decodes() {
    let payload = payload_of(
        "analyzed-grid.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato BeatGrid",
                &fixture("beatgrid/analyzed.octet-stream"),
            )],
            0,
        ),
    );
    let grid = payload.beatgrid.expect("grid");
    assert_eq!(grid.markers.len(), 1);
    assert_eq!(
        grid.markers[0],
        SeratoGridMarker::Terminal {
            position_seconds: 0.304_994_344_711_303_7,
            bpm: 115.0,
        }
    );
}

#[test]
fn invalid_grids_are_rejected() {
    let cases: Vec<Vec<u8>> = vec![
        // Empty marker list.
        vec![1, 0, 0, 0, 0, 0, 0x37],
        // Zero BPM.
        grid_payload(&[(1.0, 0.0f32.to_bits())]),
        // Negative position.
        grid_payload(&[(-1.0, 79.0f32.to_bits())]),
        // Non-monotonic positions.
        grid_payload(&[(2.0, 4), (1.0, 79.0f32.to_bits())]),
        // Zero beats between markers.
        grid_payload(&[(1.0, 0), (2.0, 79.0f32.to_bits())]),
        // Truncated marker array.
        vec![1, 0, 0, 0, 0, 2, 0x40, 0x00, 0x00, 0x00],
    ];
    for (index, case) in cases.iter().enumerate() {
        let payload = payload_of(
            &format!("bad-grid-{index}.mp3"),
            &single_geob("Serato BeatGrid", case),
        );
        assert!(payload.beatgrid.is_none(), "case {index} must be rejected");
        let frames = frames_by_kind(&payload, SeratoFrameKind::BeatGrid);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].status, FrameStatus::Invalid);
    }
}

// ---------------------------------------------------------------------------
// Mixed In Key
// ---------------------------------------------------------------------------

#[test]
fn mik_cues_are_a_fallback_only() {
    let times = [
        510.0, 12_661.0, 30_888.0, 55_190.0, 85_568.0, 146_323.0, 158_475.0, 182_777.0,
    ];
    let cue_points = cue_points_body(&times);

    // A grid alone does not suppress the fallback.
    let payload = payload_of(
        "mik-grid.mp3",
        &tag_with(
            4,
            &[
                geob_frame(
                    4,
                    "Serato BeatGrid",
                    &fixture("beatgrid/analyzed.octet-stream"),
                ),
                geob_frame(4, "CuePoints", &cue_points),
            ],
            0,
        ),
    );
    assert_eq!(
        payload
            .cues
            .iter()
            .map(|c| (c.slot, c.position_ms, c.label.clone()))
            .collect::<Vec<_>>(),
        times
            .iter()
            .enumerate()
            .map(|(slot, ms)| (slot as u8, *ms as u32, None))
            .collect::<Vec<_>>()
    );

    // A valid but empty Markers2 frame suppresses the fallback.
    let empty_markers2 = markers2_payload(&[
        ("COLOR", color_record([0xFF, 0xFF, 0xFF])),
        ("BPMLOCK", bpm_lock_record(false)),
    ]);
    let payload = payload_of(
        "mik-empty-markers.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers2", &empty_markers2),
                geob_frame(4, "CuePoints", &cue_points),
            ],
            0,
        ),
    );
    assert!(payload.cues.is_empty());
    assert_eq!(
        status_of(&payload, SeratoFrameKind::MikCuePoints),
        FrameStatus::Valid
    );

    // A malformed markers frame also suppresses it.
    let payload = payload_of(
        "mik-invalid-markers.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers2", b"\x01\x01@@@"),
                geob_frame(4, "CuePoints", &cue_points),
            ],
            0,
        ),
    );
    assert!(payload.cues.is_empty());
    assert_eq!(
        status_of(&payload, SeratoFrameKind::Markers2),
        FrameStatus::Invalid
    );

    // A legacy markers frame alone also suppresses it.
    let payload = payload_of(
        "mik-legacy-markers.mp3",
        &tag_with(
            4,
            &[
                geob_frame(
                    4,
                    "Serato Markers_",
                    &fixture("markers_/hotcue-positions-00m00s0-03m38s4-01m00s0-00m00s1-00m01s0.octet-stream"),
                ),
                geob_frame(4, "CuePoints", &cue_points),
            ],
            0,
        ),
    );
    assert_eq!(payload.cues.len(), 5);
    assert!(payload
        .cues
        .iter()
        .all(|cue| cue.label.is_none() && cue.color_rgb.is_none()));
}

#[test]
fn mik_key_fallback_is_independent_of_markers() {
    let markers2 = markers2_payload(&[("COLOR", color_record([0xFF, 0xFF, 0xFF]))]);
    let payload = payload_of(
        "mik-key.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers2", &markers2),
                geob_frame(4, "Key", &key_body("5A")),
            ],
            0,
        ),
    );
    assert_eq!(payload.mik_key.as_deref(), Some("5A"));
    assert!(payload.cues.is_empty());
}

#[test]
fn malformed_mik_payloads_are_rejected() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "wrong-source",
            mik_body(serde_json::json!({"key": "5A", "source": "other"})),
        ),
        ("bad-base64", b"@@@@".to_vec()),
        ("bad-json", b"eyJub3QiOg==".to_vec()),
        (
            "empty-key",
            mik_body(serde_json::json!({"key": "  ", "source": "mixedinkey"})),
        ),
    ];
    for (name, body) in cases {
        let payload = payload_of(&format!("{name}.mp3"), &single_geob("Key", &body));
        assert_eq!(payload.mik_key, None, "{name}");
        assert_eq!(
            status_of(&payload, SeratoFrameKind::MikKey),
            FrameStatus::Invalid,
            "{name}"
        );
    }

    let cue_cases: Vec<(&str, Vec<u8>)> = vec![
        ("negative", cue_points_body(&[-5.0])),
        ("too-large", cue_points_body(&[f64::from(u32::MAX) + 10.0])),
        (
            "not-a-number",
            mik_body(serde_json::json!({"cues": [{"time": "510"}], "source": "mixedinkey"})),
        ),
        (
            "missing-time",
            mik_body(serde_json::json!({"cues": [{"name": "x"}], "source": "mixedinkey"})),
        ),
        (
            // Not representable as a finite f64: rejected, never rounded to a slot.
            "non-finite",
            B64.encode(br#"{"cues":[{"time":1e999}],"source":"mixedinkey"}"#)
                .into_bytes(),
        ),
    ];
    for (name, body) in cue_cases {
        let payload = payload_of(&format!("cue-{name}.mp3"), &single_geob("CuePoints", &body));
        assert!(payload.cues.is_empty(), "{name}");
        assert_eq!(
            status_of(&payload, SeratoFrameKind::MikCuePoints),
            FrameStatus::Invalid,
            "{name}"
        );
    }

    // Entries beyond the eighth are ignored; the first eight still import.
    let times: Vec<f64> = (0..10).map(|index| f64::from(index) * 1_000.0).collect();
    let payload = payload_of(
        "nine-cues.mp3",
        &tag_with(
            4,
            &[geob_frame(4, "CuePoints", &cue_points_body(&times))],
            0,
        ),
    );
    assert_eq!(payload.cues.len(), 8);
    assert_eq!(payload.cues[7].position_ms, 7_000);
}

#[test]
fn positional_json_is_not_accepted_as_an_object() {
    // `cues` written as positional arrays instead of objects.
    let raw = B64
        .encode(br#"{"source":"mixedinkey","cues":[[1234,"X"]]}"#)
        .into_bytes();
    let payload = payload_of("positional-cues.mp3", &single_geob("CuePoints", &raw));
    assert!(payload.cues.is_empty());
    assert_eq!(
        status_of(&payload, SeratoFrameKind::MikCuePoints),
        FrameStatus::Invalid
    );

    // A `Key` frame replaced wholesale by a positional array.
    let raw = B64.encode(br#"["mixedinkey","5A",null]"#).into_bytes();
    let payload = payload_of("positional-key.mp3", &single_geob("Key", &raw));
    assert_eq!(payload.mik_key, None);
    assert_eq!(
        status_of(&payload, SeratoFrameKind::MikKey),
        FrameStatus::Invalid
    );
}

#[test]
fn mik_time_rounds_to_the_nearest_millisecond() {
    let mut body = mik_json(serde_json::json!({
        "cues": [{"time": 510.4}, {"time": 12661.6, "name": "Drop"}],
        "source": "mixedinkey",
        "algorithm": 14
    }));
    body.push(0);
    let payload = payload_of("rounding.mp3", &single_geob("CuePoints", &body));
    assert_eq!(payload.cues[0].position_ms, 510);
    assert_eq!(payload.cues[0].label, None);
    assert_eq!(payload.cues[1].position_ms, 12_662);
    assert_eq!(payload.cues[1].label.as_deref(), Some("Drop"));
    assert_eq!(payload.cues[1].color_rgb, None);
}

// ---------------------------------------------------------------------------
// Containers and ID3 structure
// ---------------------------------------------------------------------------

fn carrier_mp3(tag: &[u8]) -> Vec<u8> {
    let carrier = fixture("carriers/empty.mp3");
    assert_eq!(carrier.len(), 2506, "pinned silent carrier");
    assert!(!carrier.starts_with(b"ID3"), "carrier is MPEG frames only");
    let mut out = tag.to_vec();
    out.extend_from_slice(&carrier);
    out
}

#[test]
fn mp3_carrier_with_prepended_tag_decodes() {
    let tag = tag_with(
        4,
        &[geob_frame(
            4,
            "Serato Markers2",
            &fixture("markers2/hotcue-colors.octet-stream"),
        )],
        0,
    );
    let payload = payload_of("carrier.mp3", &carrier_mp3(&tag));
    assert!(payload.scan_complete);
    assert_eq!(payload.cues.len(), 8);
    assert_eq!(payload.cues[3].color_rgb, Some(0xCC_CC00));
}

#[test]
fn wav_container_reads_the_id3_chunk_across_odd_chunks() {
    let wav = wav_with(&[
        (
            b"fmt ",
            b"\x01\x00\x02\x00\x44\xac\x00\x00\x10\xb1\x02\x00\x04\x00\x10\x00",
        ),
        (b"junk", b"\x01\x02\x03"),
        (b"id3 ", &tea_leaves()),
        (b"data", b"\x00\x00\x00\x00"),
    ]);
    let payload = payload_of("tagged.wav", &wav);
    assert!(payload.scan_complete);
    assert_eq!(payload.cues.len(), 8);
    assert_eq!(payload.mik_key.as_deref(), Some("5A"));
}

#[test]
fn aiff_and_aifc_containers_read_the_id3_chunk() {
    for form in [b"AIFF", b"AIFC"] {
        let aiff = aiff_with(
            form,
            &[
                (b"COMM", b"\x00\x01\x00\x00\x00\x00\x00\x0a\x40\x0e\xac\x44"),
                (b"SSND", b"\x00\x00\x00\x00\x00\x00\x00\x00"),
                (b"ID3 ", &tea_leaves()),
            ],
        );
        let payload = payload_of("tagged.aiff", &aiff);
        assert!(payload.scan_complete, "{}", String::from_utf8_lossy(form));
        assert_eq!(payload.cues.len(), 8);
        assert_eq!(payload.beatgrid.expect("grid").markers.len(), 1);
    }
}

#[test]
fn containers_without_a_metadata_chunk_have_no_auxiliary_data() {
    let wav = wav_with(&[(b"data", b"\x00\x00\x00\x00")]);
    let payload = payload_of("plain.wav", &wav);
    assert!(payload.scan_complete);
    assert!(payload.frames.is_empty());
    assert_eq!(payload, SeratoPayload::default());

    let aiff = aiff_with(b"AIFF", &[(b"SSND", b"\x00\x00\x00\x00\x00\x00\x00\x00")]);
    assert_eq!(payload_of("plain.aiff", &aiff), SeratoPayload::default());
}

#[test]
fn unsupported_containers_have_no_auxiliary_data() {
    let mut flac = b"fLaC".to_vec();
    flac.resize(64, 0);
    assert_eq!(payload_of("track.flac", &flac), SeratoPayload::default());

    let mut m4a = vec![0, 0, 0, 0x20];
    m4a.extend_from_slice(b"ftypM4A ");
    m4a.resize(64, 0);
    assert_eq!(payload_of("track.m4a", &m4a), SeratoPayload::default());

    assert_eq!(payload_of("empty.bin", b""), SeratoPayload::default());
}

#[test]
fn container_length_and_padding_errors_mark_the_scan_incomplete() {
    let tag = tag_with(
        4,
        &[geob_frame(
            4,
            "Serato Markers2",
            &fixture("markers2/hotcue-colors.octet-stream"),
        )],
        0,
    );

    // RIFF size claims more than the file holds.
    let mut wav = wav_with(&[(b"id3 ", &tag)]);
    let declared = wav.len() as u32 + 10;
    wav[4..8].copy_from_slice(&declared.to_le_bytes());
    let payload = payload_of("long-riff.wav", &wav);
    assert!(!payload.scan_complete);
    assert!(payload.cues.is_empty());

    // A chunk declares a size that runs past the declared form end.
    let mut wav = wav_with(&[(b"id3 ", &tag)]);
    let id3_offset = 12;
    wav[id3_offset + 4..id3_offset + 8].copy_from_slice(&(tag.len() as u32 + 4).to_le_bytes());
    let payload = payload_of("overrun.wav", &wav);
    assert!(!payload.scan_complete);
    assert!(payload.cues.is_empty());

    // FORM size beyond the file length.
    let mut aiff = aiff_with(b"AIFF", &[(b"ID3 ", &tag)]);
    let declared = aiff.len() as u32 + 10;
    aiff[4..8].copy_from_slice(&declared.to_be_bytes());
    assert!(!payload_of("long-form.aiff", &aiff).scan_complete);

    // A partial chunk header before the metadata chunk is malformed.
    let mut payload_bytes = vec![0u8; 5];
    payload_bytes.extend_from_slice(b"id3 ");
    payload_bytes.extend_from_slice(&(tag.len() as u32).to_le_bytes());
    payload_bytes.extend_from_slice(&tag);
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    let declared = (payload_bytes.len() + 4) as u32;
    wav.extend_from_slice(&declared.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(&payload_bytes);
    assert!(!payload_of("short-tail.wav", &wav).scan_complete);
}

#[test]
fn both_id3_versions_decode_frame_sizes() {
    let markers2 = fixture("markers2/hotcue-colors.octet-stream");
    for version in [3u8, 4u8] {
        let payload = payload_of(
            &format!("v{version}.mp3"),
            &tag_with(
                version,
                &[geob_frame(version, "Serato Markers2", &markers2)],
                16,
            ),
        );
        assert!(payload.scan_complete, "v{version}");
        assert_eq!(payload.cues.len(), 8, "v{version}");
    }
}

#[test]
fn extended_headers_are_skipped_in_both_versions() {
    let markers2 = fixture("markers2/hotcue-colors.octet-stream");

    // v2.3: size word excludes itself.
    let mut body = vec![0u8, 0, 0, 6, 0, 0, 0, 0, 0, 0];
    body.extend_from_slice(&geob_frame(3, "Serato Markers2", &markers2));
    let payload = payload_of("ext23.mp3", &id3_tag(3, 0x40, &body));
    assert!(payload.scan_complete);
    assert_eq!(payload.cues.len(), 8);

    // v2.4: syncsafe size includes itself.
    let mut body = vec![0u8, 0, 0, 6, 1, 0];
    body.extend_from_slice(&geob_frame(4, "Serato Markers2", &markers2));
    let payload = payload_of("ext24.mp3", &id3_tag(4, 0x40, &body));
    assert!(payload.scan_complete);
    assert_eq!(payload.cues.len(), 8);
}

#[test]
fn corrupt_extended_headers_are_incomplete_not_empty() {
    // v2.4 header claiming far more bytes than the tag holds.
    let mut body = Vec::new();
    body.extend_from_slice(&syncsafe_bytes(9_999));
    body.extend_from_slice(&[1, 0]);
    let payload = payload_of("ext-overrun.mp3", &id3_tag(4, 0x40, &body));
    assert!(!payload.scan_complete);
    assert!(payload.frames.is_empty());

    // v2.3 header shorter than the mandatory size word plus flags.
    let mut body = Vec::new();
    body.extend_from_slice(&2u32.to_be_bytes());
    body.extend_from_slice(&[0, 0]);
    let payload = payload_of("ext-too-small.mp3", &id3_tag(3, 0x40, &body));
    assert!(!payload.scan_complete);
    assert!(payload.frames.is_empty());
}

#[test]
fn excessive_container_chunk_counts_are_bounded() {
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    let chunk_count = 70_000usize;
    let payload_len = chunk_count * 8;
    wav.extend_from_slice(&((payload_len + 4) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    for _ in 0..chunk_count {
        wav.extend_from_slice(b"junk");
        wav.extend_from_slice(&0u32.to_le_bytes());
    }
    assert_eq!(wav.len(), payload_len + 12);
    let payload = payload_of("chunk-flood.wav", &wav);
    assert!(
        !payload.scan_complete,
        "a chunk flood must not look like a tagless file"
    );
    assert!(payload.frames.is_empty());
}

/// A grid whose terminal BPM is `0x4CFF0000`, i.e. contains an `FF 00` pair.
fn grid_with_ff_pair() -> Vec<u8> {
    grid_payload(&[(1.0, f32::from_bits(0x4CFF_0000).to_bits())])
}

#[test]
fn id3v23_tag_unsynchronisation_is_undone() {
    let logical_body = geob_frame(3, "Serato BeatGrid", &grid_with_ff_pair());
    let physical_body = unsynchronise(&logical_body);
    assert_ne!(physical_body, logical_body, "vector must exercise `FF 00`");

    let payload = payload_of("unsync23.mp3", &id3_tag(3, 0x80, &physical_body));
    assert!(payload.scan_complete);
    let grid = payload.beatgrid.expect("grid");
    assert_eq!(
        grid.markers[0],
        SeratoGridMarker::Terminal {
            position_seconds: 1.0,
            bpm: f64::from(f32::from_bits(0x4CFF_0000)),
        }
    );
}

#[test]
fn id3v24_frame_unsynchronisation_is_undone_exactly_once() {
    let logical_data = geob_body(0, "Serato BeatGrid", &grid_with_ff_pair());
    let physical_data = unsynchronise(&logical_data);
    assert!(physical_data.len() > logical_data.len());

    let payload = payload_of(
        "unsync24.mp3",
        &tag_with(4, &[frame(4, b"GEOB", 0x0002, &physical_data)], 0),
    );
    assert!(payload.scan_complete);
    let grid = payload.beatgrid.expect("grid");
    assert_eq!(
        grid.markers[0],
        SeratoGridMarker::Terminal {
            position_seconds: 1.0,
            bpm: f64::from(f32::from_bits(0x4CFF_0000)),
        }
    );

    // A v2.4 tag-level unsynchronisation flag applies to every frame.
    let mut body = frame(4, b"GEOB", 0, &physical_data);
    body.resize(body.len() + 32, 0);
    let payload = payload_of("unsync24-tag.mp3", &id3_tag(4, 0x80, &body));
    assert!(payload.scan_complete);
    assert_eq!(
        payload.beatgrid.expect("grid").markers[0],
        SeratoGridMarker::Terminal {
            position_seconds: 1.0,
            bpm: f64::from(f32::from_bits(0x4CFF_0000)),
        }
    );
}

#[test]
fn grouping_and_data_length_indicator_prefixes_are_skipped() {
    let markers2 = fixture("markers2/hotcue-colors.octet-stream");

    // v2.4: group id byte, then data length indicator.
    let mut data = vec![0x07u8];
    data.extend_from_slice(&syncsafe_bytes(
        geob_body(0, "Serato Markers2", &markers2).len(),
    ));
    data.extend_from_slice(&geob_body(0, "Serato Markers2", &markers2));
    let payload = payload_of(
        "dli.mp3",
        &tag_with(4, &[frame(4, b"GEOB", 0x0041, &data)], 0),
    );
    assert!(payload.scan_complete);
    assert_eq!(payload.cues.len(), 8);

    // v2.3: group id byte.
    let mut data = vec![0x07u8];
    data.extend_from_slice(&geob_body(0, "Serato Markers2", &markers2));
    let payload = payload_of(
        "group23.mp3",
        &tag_with(3, &[frame(3, b"GEOB", 0x0020, &data)], 0),
    );
    assert!(payload.scan_complete);
    assert_eq!(payload.cues.len(), 8);
}

#[test]
fn compressed_geob_frames_mark_the_scan_incomplete() {
    let markers2 = fixture("markers2/hotcue-colors.octet-stream");
    let mut data = vec![0u8, 0, 0, 0, 0]; // decompressed size prefix
    data.extend_from_slice(&geob_body(0, "Serato Markers2", &markers2));
    let payload = payload_of(
        "compressed.mp3",
        &tag_with(3, &[frame(3, b"GEOB", 0x0080, &data)], 0),
    );
    assert!(!payload.scan_complete);
    assert_eq!(
        payload,
        SeratoPayload {
            scan_complete: false,
            ..SeratoPayload::default()
        }
    );
}

#[test]
fn utf16_geob_descriptions_match() {
    let markers2 = fixture("markers2/tracklist-color.octet-stream");

    // Encoding 1 (BOM-qualified UTF-16): little-endian BOM.
    let payload = payload_of(
        "utf16le.mp3",
        &tag_with(
            4,
            &[frame(
                4,
                b"GEOB",
                0,
                &geob_body(1, "Serato Markers2", &markers2),
            )],
            0,
        ),
    );
    assert_eq!(payload.color_rgb, Some(0x99_FF99));

    // Encoding 1: big-endian BOM.
    let mut data = vec![1u8];
    data.extend_from_slice(b"application/octet-stream\0");
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(&[0xFE, 0xFF]);
    for unit in "Serato Markers2".encode_utf16() {
        data.extend_from_slice(&unit.to_be_bytes());
    }
    data.extend_from_slice(&[0, 0]);
    data.extend_from_slice(&markers2);
    let payload = payload_of(
        "utf16be-bom.mp3",
        &tag_with(4, &[frame(4, b"GEOB", 0, &data)], 0),
    );
    assert_eq!(payload.color_rgb, Some(0x99_FF99));

    // Encoding 2: UTF-16BE without a BOM.
    let payload = payload_of(
        "utf16be.mp3",
        &tag_with(
            4,
            &[frame(
                4,
                b"GEOB",
                0,
                &geob_body(2, "Serato Markers2", &markers2),
            )],
            0,
        ),
    );
    assert_eq!(payload.color_rgb, Some(0x99_FF99));
}

#[test]
fn unknown_geob_descriptions_are_ignored() {
    let payload = payload_of(
        "unknown.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Overview", &[0u8; 32]),
                geob_frame(4, "Energy", &mik_body(serde_json::json!({"energy": 8}))),
                geob_frame(4, "Key", &key_body("5A")),
            ],
            0,
        ),
    );
    assert_eq!(payload.frames.len(), 1);
    assert_eq!(payload.frames[0].kind, SeratoFrameKind::MikKey);
}

// ---------------------------------------------------------------------------
// Corruption, truncation and bounds
// ---------------------------------------------------------------------------

#[test]
fn truncated_files_never_panic_or_import_partial_data() {
    let tag = tea_leaves();
    for length in [
        0usize,
        1,
        3,
        9,
        10,
        11,
        20,
        64,
        200,
        700,
        1000,
        tag.len() - 1,
    ] {
        let payload = payload_of(&format!("truncated-{length}.mp3"), &tag[..length]);
        assert!(
            payload.cues.is_empty(),
            "length {length} must not decode cues"
        );
        assert!(payload.beatgrid.is_none(), "length {length}");
        assert_eq!(payload.mik_key, None, "length {length}");
        if length >= 3 {
            assert!(!payload.scan_complete, "length {length}");
        }
    }
}

#[test]
fn truncated_frames_mark_the_scan_incomplete() {
    let markers2 = fixture("markers2/hotcue-colors.octet-stream");
    let mut frames = geob_frame(4, "Serato Markers2", &markers2);
    // Frame size claims 20 bytes more than stored.
    let declared = u32::from_be_bytes([0, 0, 0, 0]);
    let _ = declared;
    let body_size_bytes = syncsafe_bytes(frames.len() - 10 + 20);
    frames[4..8].copy_from_slice(&body_size_bytes);
    let tag = tag_with(4, &[frames], 0);
    let payload = payload_of("truncated-frame.mp3", &tag);
    assert!(!payload.scan_complete);
    assert!(payload.cues.is_empty());
}

#[test]
fn corrupted_geob_content_marks_the_scan_incomplete() {
    // A GEOB frame whose description has no terminator cannot be classified.
    let data = vec![0u8, b'a', b'p', b'p'];
    let payload = payload_of(
        "bad-geob.mp3",
        &tag_with(4, &[frame(4, b"GEOB", 0, &data)], 0),
    );
    assert!(!payload.scan_complete);
    assert!(payload.frames.is_empty());
}

#[test]
fn corrupt_markers_and_grid_payloads_are_invalid_not_partial() {
    // Markers2: record length beyond the decoded data.
    let mut binary = vec![1u8, 1];
    binary.extend_from_slice(b"COLOR\0");
    binary.extend_from_slice(&uint32(40));
    binary.extend_from_slice(&[0, 1, 2, 3]);
    let markers2 = {
        let mut out = vec![1u8, 1];
        out.extend_from_slice(B64.encode(&binary).as_bytes());
        out
    };
    let payload = payload_of(
        "bad-markers2.mp3",
        &single_geob("Serato Markers2", &markers2),
    );
    assert_eq!(
        status_of(&payload, SeratoFrameKind::Markers2),
        FrameStatus::Invalid
    );
    assert!(payload.cues.is_empty());
    assert_eq!(payload.color_rgb, None);

    // Markers2: slot outside 0–7.
    let mut records = vec![("COLOR", color_record([0xFF, 0xFF, 0xFF]))];
    records.push(("CUE", cue_record(9, 1_000, [0, 0, 0], "bad")));
    let payload = payload_of(
        "bad-slot.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato Markers2",
                &markers2_payload(&records),
            )],
            0,
        ),
    );
    assert_eq!(
        status_of(&payload, SeratoFrameKind::Markers2),
        FrameStatus::Invalid
    );

    // Loop end before start.
    let mut records = vec![("COLOR", color_record([0xFF, 0xFF, 0xFF]))];
    records.push(("LOOP", loop_record(0, 2_000, 1_000, [0, 0, 0], 0, "")));
    let payload = payload_of(
        "bad-loop-bounds.mp3",
        &tag_with(
            4,
            &[geob_frame(
                4,
                "Serato Markers2",
                &markers2_payload(&records),
            )],
            0,
        ),
    );
    assert_eq!(
        status_of(&payload, SeratoFrameKind::Markers2),
        FrameStatus::Invalid
    );

    // Legacy markers: wrong entry count.
    let mut legacy = fixture("markers_/saved-loops.octet-stream");
    legacy[2..6].copy_from_slice(&uint32(13));
    let payload = payload_of(
        "bad-legacy-count.mp3",
        &single_geob("Serato Markers_", &legacy),
    );
    assert_eq!(
        status_of(&payload, SeratoFrameKind::MarkersLegacy),
        FrameStatus::Invalid
    );

    // Legacy markers: corrupt set/unset flag.
    let mut legacy = fixture("markers_/saved-loops.octet-stream");
    legacy[6] = 0x12;
    let payload = payload_of(
        "bad-legacy-flag.mp3",
        &single_geob("Serato Markers_", &legacy),
    );
    assert_eq!(
        status_of(&payload, SeratoFrameKind::MarkersLegacy),
        FrameStatus::Invalid
    );

    // Grid: excessive marker count.
    let mut grid = vec![1u8, 0];
    grid.extend_from_slice(&uint32(0x00FF_FFFF));
    let payload = payload_of("huge-grid.mp3", &single_geob("Serato BeatGrid", &grid));
    assert!(payload.beatgrid.is_none());
    assert_eq!(
        status_of(&payload, SeratoFrameKind::BeatGrid),
        FrameStatus::Invalid
    );
}

#[test]
fn excessive_claimed_sizes_are_bounded() {
    // Tag header claims 16 MiB + 1; the file is tiny.
    let mut tag = Vec::new();
    tag.extend_from_slice(b"ID3");
    tag.extend_from_slice(&[4, 0, 0]);
    tag.extend_from_slice(&[0x10, 0x00, 0x00, 0x01]);
    let payload = payload_of("huge-tag.mp3", &tag);
    assert!(!payload.scan_complete);
    assert!(payload.frames.is_empty());

    // A frame claims more bytes than its tag contains.
    let mut body = Vec::new();
    body.extend_from_slice(b"GEOB");
    body.extend_from_slice(&syncsafe_bytes(0x0FFF_FFFF));
    body.extend_from_slice(&[0, 0]);
    let payload = payload_of("huge-frame.mp3", &id3_tag(4, 0, &body));
    assert!(!payload.scan_complete);
    assert!(payload.frames.is_empty());

    // 1 mod 4 base64 that is not valid data at all.
    let payload = payload_of("bad-b64.mp3", &single_geob("Serato Markers2", b"\x01\x01A"));
    assert_eq!(
        status_of(&payload, SeratoFrameKind::Markers2),
        FrameStatus::Invalid
    );
}

#[test]
fn one_malformed_payload_does_not_discard_independent_kinds() {
    let payload = payload_of(
        "mixed-validity.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers2", b"\x01\x01@@@"),
                geob_frame(
                    4,
                    "Serato BeatGrid",
                    &fixture("beatgrid/manual-beatgrid-changing-tempo.octet-stream"),
                ),
                geob_frame(4, "Key", &key_body("5A")),
            ],
            0,
        ),
    );
    assert!(payload.scan_complete);
    assert_eq!(
        status_of(&payload, SeratoFrameKind::Markers2),
        FrameStatus::Invalid
    );
    assert_eq!(payload.beatgrid.expect("grid").markers.len(), 8);
    assert_eq!(payload.mik_key.as_deref(), Some("5A"));
}

#[test]
fn duplicate_frames_are_retained_but_the_first_one_wins() {
    let first = markers2_payload(&[
        ("COLOR", color_record([0x11, 0x22, 0x33])),
        ("CUE", cue_record(0, 1_000, [0xAA, 0, 0], "first")),
    ]);
    let second = markers2_payload(&[
        ("COLOR", color_record([0x44, 0x55, 0x66])),
        ("CUE", cue_record(0, 2_000, [0xBB, 0, 0], "second")),
    ]);
    let payload = payload_of(
        "duplicates.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers2", &first),
                geob_frame(4, "Serato Markers2", &second),
            ],
            0,
        ),
    );
    assert_eq!(payload.frames.len(), 2);
    assert_eq!(payload.color_rgb, Some(0x11_2233));
    assert_eq!(payload.cues[0].label.as_deref(), Some("first"));

    // The first occurrence controls parsing even when it is invalid.
    let payload = payload_of(
        "duplicates-invalid-first.mp3",
        &tag_with(
            4,
            &[
                geob_frame(4, "Serato Markers2", b"\x01\x01@@@"),
                geob_frame(4, "Serato Markers2", &second),
            ],
            0,
        ),
    );
    assert_eq!(payload.frames.len(), 2);
    assert_eq!(payload.frames[0].status, FrameStatus::Invalid);
    assert_eq!(payload.frames[1].status, FrameStatus::Valid);
    assert!(payload.cues.is_empty());
    assert_eq!(payload.color_rgb, None);
}

#[test]
fn excessive_markers2_record_counts_are_bounded() {
    let mut binary = vec![1u8, 1];
    for _ in 0..16_385 {
        binary.extend_from_slice(b"X\0");
        binary.extend_from_slice(&0u32.to_be_bytes());
    }
    binary.push(0);
    let mut markers2 = vec![1u8, 1];
    markers2.extend_from_slice(B64.encode(&binary).as_bytes());

    let payload = payload_of(
        "too-many-records.mp3",
        &single_geob("Serato Markers2", &markers2),
    );
    assert!(
        !payload.scan_complete,
        "a record bound must not masquerade as a complete scan"
    );
    assert_eq!(
        status_of(&payload, SeratoFrameKind::Markers2),
        FrameStatus::Invalid
    );
}

#[test]
fn reading_never_modifies_the_file() {
    let bytes = tea_leaves();
    let file = temp_file("untouched.mp3", &carrier_mp3(&bytes));
    let before = fs::read(&file.path).expect("read");
    let _ = file.read_payload();
    let _ = read_file_serato(Path::new(FIXTURES).join("tea-leaves.id3").as_path()).expect("read");
    let after = fs::read(&file.path).expect("read");
    assert_eq!(before, after);
}

#[test]
fn missing_files_are_reported_as_errors() {
    let missing = Path::new(FIXTURES).join("does-not-exist.mp3");
    assert!(read_file_serato(&missing).is_err());
}
