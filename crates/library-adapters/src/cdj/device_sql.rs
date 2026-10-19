//! DeviceSQL string encoding used throughout the device database.
//!
//! Pioneer's embedded database stores strings in one of three forms,
//! distinguished by the first byte:
//!
//! - **Short ASCII** (≤ 126 bytes): one length/flag byte
//!   `((len + 1) << 1) | 1` followed by the raw ASCII bytes.
//! - **Long ASCII** (> 126 bytes): flag `0x40`, a little-endian `u16` total
//!   length (`content + 4`), a padding byte, then the bytes.
//! - **Long UTF-16LE** (any non-ASCII content): flag `0x90`, a little-endian
//!   `u16` total length (`content_bytes + 4`), a padding byte, then UTF-16LE
//!   code units.
//!
//! See Deep Symmetry's *Database Exports* reference for the byte layout.

const SHORT_ASCII: u8 = 0x01;
const LONG_ASCII: u8 = 0x40;
const LONG_UTF16LE: u8 = 0x90;

/// Encode a string into its DeviceSQL byte form.
///
/// An empty string encodes to the single byte `0x03` (short ASCII, length 0),
/// which is what Rekordbox writes for absent fields.
pub fn encode(value: &str) -> Vec<u8> {
    if !value.is_ascii() {
        return encode_utf16(value);
    }

    let bytes = value.as_bytes();
    let len = bytes.len();
    if len <= 126 {
        let mut out = Vec::with_capacity(1 + len);
        out.push((((len + 1) << 1) as u8) | SHORT_ASCII);
        out.extend_from_slice(bytes);
        out
    } else {
        let mut out = Vec::with_capacity(4 + len);
        out.push(LONG_ASCII);
        out.extend_from_slice(&((len + 4) as u16).to_le_bytes());
        out.push(0);
        out.extend_from_slice(bytes);
        out
    }
}

fn encode_utf16(value: &str) -> Vec<u8> {
    let units: Vec<u16> = value.encode_utf16().collect();
    let byte_len = units.len() * 2;
    let mut out = Vec::with_capacity(4 + byte_len);
    out.push(LONG_UTF16LE);
    out.extend_from_slice(&((byte_len + 4) as u16).to_le_bytes());
    out.push(0);
    for unit in units {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_ascii_length_and_flag() {
        let encoded = encode("Hello");
        assert_eq!(encoded[0], 0x0D);
        assert_eq!(&encoded[1..], b"Hello");
    }

    #[test]
    fn empty_string_is_a_length_zero_short_string() {
        assert_eq!(encode(""), vec![0x03]);
    }

    #[test]
    fn non_ascii_uses_utf16le() {
        let encoded = encode("Déjà");
        assert_eq!(encoded[0], LONG_UTF16LE);
        assert_eq!(u16::from_le_bytes([encoded[1], encoded[2]]), 12);
        assert_eq!(encoded[3], 0);
        assert_eq!(&encoded[4..6], &[0x44, 0x00]);
        assert_eq!(&encoded[6..8], &[0xE9, 0x00]);
    }

    #[test]
    fn long_ascii_carries_the_total_length() {
        let encoded = encode(&"a".repeat(130));
        assert_eq!(encoded[0], LONG_ASCII);
        assert_eq!(u16::from_le_bytes([encoded[1], encoded[2]]), 134);
        assert_eq!(encoded[3], 0);
        assert_eq!(encoded.len(), 134);
    }
}
