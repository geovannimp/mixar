//! Parser for the `Serato Markers2` native tag payload.
//!
//! The payload is `01 01`, followed by base64 (optionally wrapped with
//! linefeeds) and NUL padding. The decoded data repeats `01 01` and then walks
//! NUL-terminated record names, a big-endian `u32` length and that many payload
//! bytes, finishing with a single terminator byte.
//!
//! Unknown records (including `FLIP`) are retained verbatim as opaque records so
//! that later write-back work can round-trip them; this reader never drops them.

use base64::alphabet;
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::Engine as _;

use super::{OpaqueRecord, SeratoCue, SeratoLoop};

/// Hard cap on records walked inside one Markers2 payload.
const MAX_RECORDS: usize = 16_384;

/// Base64 configuration used by Serato: padding is optional and trailing bits
/// of the final quantum are tolerated.
const BASE64: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_padding_mode(DecodePaddingMode::Indifferent)
        .with_decode_allow_trailing_bits(true),
);

/// Why a Markers2 payload could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Markers2Error {
    /// The payload is malformed; this frame carries no usable data.
    Invalid,
    /// The record bound was exceeded, so the tag cannot be trusted to be complete.
    RecordLimit,
}

/// Decoded Markers2 payload.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Markers2Data {
    pub(crate) cues: Vec<SeratoCue>,
    pub(crate) loops: Vec<SeratoLoop>,
    pub(crate) color_rgb: Option<u32>,
    pub(crate) bpm_locked: Option<bool>,
    pub(crate) opaque: Vec<OpaqueRecord>,
}

/// Parse a logical `Serato Markers2` GEOB object.
///
/// Returns `Err(())` for a payload that cannot be decoded as a whole; partial
/// typed output is never produced from a malformed payload.
pub(crate) fn parse(data: &[u8]) -> Result<Markers2Data, Markers2Error> {
    if data.len() < 2 || data[0] != 1 || data[1] != 1 {
        return Err(Markers2Error::Invalid);
    }

    // Strip trailing NUL padding and ASCII base64 whitespace.
    let mut end = data.len();
    while end > 2 && (data[end - 1] == 0 || data[end - 1].is_ascii_whitespace()) {
        end -= 1;
    }
    let decoded = decode_base64(&data[2..end]).ok_or(Markers2Error::Invalid)?;
    if decoded.len() < 2 || decoded[0] != 1 || decoded[1] != 1 {
        return Err(Markers2Error::Invalid);
    }

    let mut out = Markers2Data::default();
    let mut pos = 2usize;
    let mut records = 0usize;
    // Real files may end exactly at the last record instead of carrying the
    // documented terminator byte (the `very-long-names` fixture does).
    while let Some(&first) = decoded.get(pos) {
        if first == 0 {
            break;
        }
        let Some(name_len) = decoded[pos..].iter().position(|b| *b == 0) else {
            return Err(Markers2Error::Invalid);
        };
        let name = &decoded[pos..pos + name_len];
        pos += name_len + 1;
        let Some(len_bytes) = decoded.get(pos..pos + 4) else {
            return Err(Markers2Error::Invalid);
        };
        let len = u32::from_be_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]);
        pos += 4;
        let Some(end) = pos.checked_add(len as usize) else {
            return Err(Markers2Error::Invalid);
        };
        let Some(payload) = decoded.get(pos..end) else {
            return Err(Markers2Error::Invalid);
        };
        pos = end;
        records += 1;
        if records > MAX_RECORDS {
            return Err(Markers2Error::RecordLimit);
        }

        match name {
            b"CUE" => {
                let cue = parse_cue(payload)?;
                out.cues.push(cue);
            }
            b"LOOP" => {
                let lp = parse_loop(payload)?;
                out.loops.push(lp);
            }
            b"COLOR" => {
                if payload.len() < 4 {
                    return Err(Markers2Error::Invalid);
                }
                if out.color_rgb.is_none() {
                    out.color_rgb =
                        Some(u32::from_be_bytes([0, payload[1], payload[2], payload[3]]));
                }
            }
            b"BPMLOCK" => {
                let Some(&value) = payload.first() else {
                    return Err(Markers2Error::Invalid);
                };
                if out.bpm_locked.is_none() {
                    out.bpm_locked = Some(value != 0);
                }
            }
            _ => out.opaque.push(OpaqueRecord {
                record_type: name.to_vec(),
                payload: payload.to_vec(),
            }),
        }
    }

    Ok(out)
}

fn parse_cue(payload: &[u8]) -> Result<SeratoCue, Markers2Error> {
    if payload.len() < 13 {
        return Err(Markers2Error::Invalid);
    }
    let slot = payload[1];
    if slot > 7 {
        return Err(Markers2Error::Invalid);
    }
    let position_ms = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]);
    let color_rgb = u32::from_be_bytes([0, payload[7], payload[8], payload[9]]);
    let label = decode_name(&payload[12..])?;
    Ok(SeratoCue {
        slot,
        position_ms,
        label,
        color_rgb: Some(color_rgb),
    })
}

fn parse_loop(payload: &[u8]) -> Result<SeratoLoop, Markers2Error> {
    if payload.len() < 20 {
        return Err(Markers2Error::Invalid);
    }
    let slot = payload[1];
    if slot > 7 {
        return Err(Markers2Error::Invalid);
    }
    let in_ms = u32::from_be_bytes([payload[2], payload[3], payload[4], payload[5]]);
    let out_ms = u32::from_be_bytes([payload[6], payload[7], payload[8], payload[9]]);
    if out_ms <= in_ms {
        return Err(Markers2Error::Invalid);
    }
    let argb = u32::from_be_bytes([payload[14], payload[15], payload[16], payload[17]]);
    let label = if payload.len() > 20 {
        decode_name(&payload[20..])?
    } else {
        None
    };
    Ok(SeratoLoop {
        slot,
        in_ms,
        out_ms,
        label,
        color_rgb: Some(argb & 0x00FF_FFFF),
    })
}

/// NUL-terminated UTF-8 name; an empty name means "no label".
fn decode_name(bytes: &[u8]) -> Result<Option<String>, Markers2Error> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    if end == 0 {
        return Ok(None);
    }
    let text = std::str::from_utf8(&bytes[..end]).map_err(|_| Markers2Error::Invalid)?;
    Ok(Some(text.to_string()))
}

/// Decode Serato's base64 variant: whitespace is ignored, padding is optional
/// and a length of `1 mod 4` gets an `A` appended before padding is restored.
fn decode_base64(text: &[u8]) -> Option<Vec<u8>> {
    let mut buf: Vec<u8> = text
        .iter()
        .copied()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    while buf.last() == Some(&b'=') {
        buf.pop();
    }
    if buf.is_empty() {
        return None;
    }
    if buf.len() % 4 == 1 {
        buf.push(b'A');
    }
    BASE64.decode(&buf).ok()
}
