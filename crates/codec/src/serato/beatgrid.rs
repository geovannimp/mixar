//! Parser for the native `Serato BeatGrid` tag payload.
//!
//! Layout: `01 00`, a big-endian `u32` marker count, `count` eight-byte markers
//! and one footer byte. Every marker but the last stores a `f32` position in
//! seconds plus a `u32` number of beats until the next marker; the last marker
//! stores its `f32` position plus the `f32` terminal BPM.
//!
//! Positions are seconds. Promoting the wire `f32` values to `f64` is exact, so
//! downstream tempo maths never re-rounds them.

use super::{SeratoBeatGrid, SeratoGridMarker};

pub(crate) fn parse(data: &[u8]) -> Result<SeratoBeatGrid, ()> {
    if data.len() < 7 || data[0] != 1 || data[1] != 0 {
        return Err(());
    }
    let count = u32::from_be_bytes([data[2], data[3], data[4], data[5]]) as usize;
    let Some(body_len) = count.checked_mul(8).and_then(|n| n.checked_add(6)) else {
        return Err(());
    };
    let Some(total_len) = body_len.checked_add(1) else {
        return Err(());
    };
    if count == 0 || data.len() < total_len {
        return Err(());
    }
    // `total_len` already includes the one-byte footer at `total_len - 1`, whose
    // value is undocumented noise (real files use `0x37`, `0x20`, …), so it is
    // never validated. Only bytes *after* the footer must be zero padding.
    if data[total_len..].iter().any(|b| *b != 0) {
        return Err(());
    }

    let mut markers = Vec::with_capacity(count);
    let mut previous: Option<f64> = None;
    for index in 0..count {
        let marker = &data[6 + index * 8..6 + (index + 1) * 8];
        let position_seconds =
            f32::from_be_bytes([marker[0], marker[1], marker[2], marker[3]]) as f64;
        if !position_seconds.is_finite() || position_seconds < 0.0 {
            return Err(());
        }
        if let Some(prev) = previous {
            if position_seconds <= prev {
                return Err(());
            }
        }
        previous = Some(position_seconds);

        if index + 1 == count {
            let bpm = f32::from_be_bytes([marker[4], marker[5], marker[6], marker[7]]) as f64;
            if !bpm.is_finite() || bpm <= 0.0 {
                return Err(());
            }
            markers.push(SeratoGridMarker::Terminal {
                position_seconds,
                bpm,
            });
        } else {
            let beats_to_next = u32::from_be_bytes([marker[4], marker[5], marker[6], marker[7]]);
            if beats_to_next == 0 {
                return Err(());
            }
            markers.push(SeratoGridMarker::NonTerminal {
                position_seconds,
                beats_to_next,
            });
        }
    }

    Ok(SeratoBeatGrid { markers })
}
