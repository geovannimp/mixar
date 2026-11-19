//! Test-only independent decoder for the Serato wire layouts.
//!
//! Deliberately shares nothing with the production parsers, the encoder helpers
//! or `serato32`: it decodes the published community layouts
//! ([Markers2](https://github.com/Holzhaus/serato-tags/blob/master/docs/serato_markers2.md),
//! [legacy markers](https://github.com/Holzhaus/serato-tags/blob/master/docs/serato_markers_.md),
//! [BeatGrid](https://github.com/Holzhaus/serato-tags/blob/master/docs/serato_beatgrid.md))
//! straight from the file bytes. Only standard base64 and byte-order primitives
//! are shared with the implementation under test, as the review allowed.
//!
//! Frame extraction walks the ID3v2 tag (or the `id3 `/`ID3 ` chunk of a RIFF or
//! AIFF container) by itself so a production-reader bug cannot hide a writer
//! bug.

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

/// One `CUE` record as the community layout defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefCue {
    pub(crate) slot: u8,
    pub(crate) position_ms: u32,
    /// `(r, g, b)` from bytes 7–9.
    pub(crate) rgb: (u8, u8, u8),
    /// Bytes 0, 6, 10 and 11 — the reserved prefix bytes.
    pub(crate) reserved: [u8; 4],
    pub(crate) label: String,
    /// Anything after the label terminator.
    pub(crate) suffix: Vec<u8>,
}

/// One `LOOP` record as the community layout defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefLoop {
    pub(crate) slot: u8,
    pub(crate) in_ms: u32,
    pub(crate) out_ms: u32,
    /// Bytes 10–13 (the `FF FF FF FF` prefix).
    pub(crate) prefix: [u8; 4],
    pub(crate) argb: u32,
    /// Byte 18, the reserved byte after the colour.
    pub(crate) reserved: u8,
    /// Byte 19: the loop lock.
    pub(crate) locked: bool,
    pub(crate) label: String,
    pub(crate) suffix: Vec<u8>,
}

/// A decoded Markers2 payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RefMarkers2 {
    pub(crate) cues: Vec<RefCue>,
    pub(crate) loops: Vec<RefLoop>,
    pub(crate) color: Option<u32>,
    pub(crate) bpm_lock: Option<bool>,
    /// Record names in wire order, so a caller can assert nothing was reordered.
    pub(crate) order: Vec<String>,
    /// Unknown records (`FLIP`, …) with their payloads, in wire order.
    pub(crate) opaque: Vec<(String, Vec<u8>)>,
}

/// A decoded legacy `Serato Markers_` payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RefLegacy {
    /// Slots 0–4; `None` is the unset sentinel.
    pub(crate) cues: [Option<u32>; 5],
    /// Slots 0–8; `None` is the unset sentinel.
    pub(crate) loops: [Option<(u32, u32)>; 9],
    /// The four footer bytes, decoded as the track colour.
    pub(crate) footer_color: u32,
    /// Every entry's non-position bytes, so reserved/lock bytes can be compared.
    pub(crate) shadows: [Vec<u8>; 14],
}

/// A decoded `Serato BeatGrid` payload.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RefGrid {
    pub(crate) markers: Vec<(f32, f32)>,
    pub(crate) footer: u8,
}

/// All GEOB objects of a file, by description, in file order.
pub(crate) fn geob_frames(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let Some(tag) = id3_tag(bytes) else {
        return Vec::new();
    };
    let major = tag[3];
    let Some(size) = syncsafe(&tag[6..10]) else {
        return Vec::new();
    };
    let body = &tag[10..(10 + size as usize).min(tag.len())];
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 10 <= body.len() {
        let id = &body[pos..pos + 4];
        if id[0] == 0 {
            break;
        }
        let frame_size = if major == 4 {
            match syncsafe(&body[pos + 4..pos + 8]) {
                Some(size) => size as usize,
                None => break,
            }
        } else {
            u32::from_be_bytes([body[pos + 4], body[pos + 5], body[pos + 6], body[pos + 7]])
                as usize
        };
        let start = pos + 10;
        let Some(data) = body.get(start..start + frame_size) else {
            break;
        };
        pos = start + frame_size;
        if id != b"GEOB" {
            continue;
        }
        if let Some((description, object)) = parse_geob(data) {
            out.push((description, object));
        }
    }
    out
}

/// The first GEOB object with `description`.
pub(crate) fn geob(bytes: &[u8], description: &str) -> Option<Vec<u8>> {
    geob_frames(bytes)
        .into_iter()
        .find(|(found, _)| found == description)
        .map(|(_, object)| object)
}

/// The ID3v2 tag bytes of an MP3, WAV or AIFF file.
fn id3_tag(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() >= 10 && &bytes[..3] == b"ID3" {
        let size = syncsafe(&bytes[6..10])? as usize;
        return bytes.get(..10 + size);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return riff_chunk(bytes, 12, true);
    }
    if bytes.len() >= 12 && &bytes[..4] == b"FORM" {
        return riff_chunk(bytes, 12, false);
    }
    None
}

fn riff_chunk(bytes: &[u8], mut pos: usize, little_endian: bool) -> Option<&[u8]> {
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = if little_endian {
            u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ])
        } else {
            u32::from_be_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ])
        } as usize;
        let start = pos + 8;
        let end = (start + size).min(bytes.len());
        if id == b"id3 " || id == b"ID3 " {
            return bytes.get(start..end);
        }
        pos = start + size + (size & 1);
    }
    None
}

fn parse_geob(data: &[u8]) -> Option<(String, Vec<u8>)> {
    let (&encoding, rest) = data.split_first()?;
    let (_, rest) = c_string(rest, 0)?;
    let (_, rest) = c_string(rest, encoding)?;
    let (description, object) = c_string(rest, encoding)?;
    Some((description, object.to_vec()))
}

/// Read a NUL-terminated string for ID3 encoding `encoding`.
fn c_string(data: &[u8], encoding: u8) -> Option<(String, &[u8])> {
    match encoding {
        0 | 3 => {
            let end = data.iter().position(|byte| *byte == 0)?;
            let text = if encoding == 0 {
                data[..end].iter().map(|byte| char::from(*byte)).collect()
            } else {
                String::from_utf8(data[..end].to_vec()).ok()?
            };
            Some((text, &data[end + 1..]))
        }
        _ => {
            let mut end = 0usize;
            loop {
                if data.get(end..end + 2)? == [0, 0] {
                    break;
                }
                end += 2;
            }
            let little_endian = encoding == 1 && data.get(..2) == Some(&[0xFF, 0xFE]);
            let skip = usize::from(
                encoding == 1
                    && data
                        .get(..2)
                        .is_some_and(|b| b == [0xFF, 0xFE] || b == [0xFE, 0xFF]),
            ) * 2;
            let units: Vec<u16> = data[skip..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| {
                    if little_endian {
                        u16::from_le_bytes([pair[0], pair[1]])
                    } else {
                        u16::from_be_bytes([pair[0], pair[1]])
                    }
                })
                .collect();
            Some((String::from_utf16(&units).ok()?, &data[end + 2..]))
        }
    }
}

fn syncsafe(bytes: &[u8]) -> Option<u32> {
    if bytes.len() != 4 || bytes.iter().any(|byte| byte & 0x80 != 0) {
        return None;
    }
    Some(
        (u32::from(bytes[0]) << 21)
            | (u32::from(bytes[1]) << 14)
            | (u32::from(bytes[2]) << 7)
            | u32::from(bytes[3]),
    )
}

/// Decode the inner `01 01` base64 stream of a Markers2 payload.
fn markers2_binary(payload: &[u8]) -> Option<Vec<u8>> {
    if payload.get(..2)? != [1, 1] {
        return None;
    }
    let text: Vec<u8> = payload[2..]
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace() && *byte != 0)
        .collect();
    let mut text = String::from_utf8(text).ok()?;
    while text.len() % 4 != 0 {
        text.push('=');
    }
    let decoded = STANDARD.decode(text).ok()?;
    if decoded.get(..2)? != [1, 1] {
        return None;
    }
    Some(decoded)
}

/// Decode a `Serato Markers2` GEOB object.
pub(crate) fn markers2(payload: &[u8]) -> RefMarkers2 {
    let mut out = RefMarkers2::default();
    let Some(binary) = markers2_binary(payload) else {
        return out;
    };
    let mut pos = 2usize;
    while let Some(&first) = binary.get(pos) {
        if first == 0 {
            break;
        }
        let Some(name_len) = binary[pos..].iter().position(|byte| *byte == 0) else {
            break;
        };
        let name = String::from_utf8_lossy(&binary[pos..pos + name_len]).into_owned();
        pos += name_len + 1;
        let Some(len_bytes) = binary.get(pos..pos + 4) else {
            break;
        };
        let len =
            u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        pos += 4;
        let Some(record) = binary.get(pos..pos + len) else {
            break;
        };
        pos += len;
        out.order.push(name.clone());
        match name.as_str() {
            "CUE" if record.len() >= 13 => {
                let end = record[12..]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map(|offset| 12 + offset)
                    .unwrap_or(record.len());
                out.cues.push(RefCue {
                    slot: record[1],
                    position_ms: u32::from_be_bytes([record[2], record[3], record[4], record[5]]),
                    rgb: (record[7], record[8], record[9]),
                    reserved: [record[0], record[6], record[10], record[11]],
                    label: String::from_utf8_lossy(&record[12..end]).into_owned(),
                    suffix: record.get(end + 1..).unwrap_or_default().to_vec(),
                });
            }
            "LOOP" if record.len() >= 20 => {
                let end = record[20..]
                    .iter()
                    .position(|byte| *byte == 0)
                    .map(|offset| 20 + offset)
                    .unwrap_or(record.len());
                out.loops.push(RefLoop {
                    slot: record[1],
                    in_ms: u32::from_be_bytes([record[2], record[3], record[4], record[5]]),
                    out_ms: u32::from_be_bytes([record[6], record[7], record[8], record[9]]),
                    prefix: [record[10], record[11], record[12], record[13]],
                    argb: u32::from_be_bytes([record[14], record[15], record[16], record[17]]),
                    reserved: record[18],
                    locked: record[19] != 0,
                    label: String::from_utf8_lossy(&record[20..end]).into_owned(),
                    suffix: record.get(end + 1..).unwrap_or_default().to_vec(),
                });
            }
            "COLOR" if record.len() >= 4 => {
                out.color = Some(u32::from_be_bytes([0, record[1], record[2], record[3]]));
            }
            "BPMLOCK" if !record.is_empty() => {
                out.bpm_lock = Some(record[0] != 0);
            }
            _ => out.opaque.push((name, record.to_vec())),
        }
    }
    out
}

/// Decode a legacy `Serato Markers_` GEOB object (positions only).
pub(crate) fn legacy(payload: &[u8]) -> RefLegacy {
    let mut out = RefLegacy::default();
    if payload.len() < 6 + 14 * 22 + 4 || payload[..2] != [2, 5] {
        return out;
    }
    for index in 0..14 {
        let entry = &payload[6 + index * 22..6 + (index + 1) * 22];
        let start_set = entry[0] == 0;
        let start = unpack(entry[1], entry[2], entry[3], entry[4]);
        let end_set = entry[5] == 0;
        let end = unpack(entry[6], entry[7], entry[8], entry[9]);
        out.shadows[index] = entry.to_vec();
        if index < 5 {
            out.cues[index] = start_set.then_some(start);
        } else {
            out.loops[index - 5] = if start_set && end_set {
                Some((start, end))
            } else {
                None
            };
        }
    }
    let footer = &payload[6 + 14 * 22..6 + 14 * 22 + 4];
    out.footer_color = unpack(footer[0], footer[1], footer[2], footer[3]);
    out
}

/// Independent `serato32` unpacking (the implementation shares no code here).
fn unpack(w: u8, x: u8, y: u8, z: u8) -> u32 {
    let c = (z & 0x7F) | ((y & 0x01) << 7);
    let b = ((y & 0x7F) >> 1) | ((x & 0x03) << 6);
    let a = ((x & 0x7F) >> 2) | ((w & 0x07) << 5);
    (u32::from(a) << 16) | (u32::from(b) << 8) | u32::from(c)
}

/// Decode a `Serato BeatGrid` GEOB object.
pub(crate) fn beatgrid(payload: &[u8]) -> Option<RefGrid> {
    if payload.len() < 7 || payload[..2] != [1, 0] {
        return None;
    }
    let count = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]) as usize;
    let total = 6 + count * 8 + 1;
    if payload.len() < total {
        return None;
    }
    let mut markers = Vec::with_capacity(count);
    for index in 0..count {
        let marker = &payload[6 + index * 8..6 + (index + 1) * 8];
        let position = f32::from_be_bytes([marker[0], marker[1], marker[2], marker[3]]);
        let value = if index + 1 == count {
            f32::from_be_bytes([marker[4], marker[5], marker[6], marker[7]])
        } else {
            u32::from_be_bytes([marker[4], marker[5], marker[6], marker[7]]) as f32
        };
        markers.push((position, value));
    }
    Some(RefGrid {
        markers,
        footer: payload[total - 1],
    })
}
