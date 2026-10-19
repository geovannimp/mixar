//! Pioneer analysis-directory addressing.
//!
//! CDJ/XDJ hardware ignores the `analyze_path` recorded in the device database
//! and recomputes the analysis directory from the audio file's on-drive path.
//! If the analysis files are not at exactly that path, the hardware never finds
//! them and silently re-analyzes the track. The algorithm is the one behind
//! Rekordbox's `CreateAnlzFileFolderPath`, reverse-engineered from disassembly
//! and published in the *fourfour* `PIONEER.md` notes; the test vectors below
//! come from real Rekordbox exports.
//!
//! Layout: `/PIONEER/USBANLZ/P{XXX}/{HHHHHHHH}/ANLZ0000.{DAT,EXT}` where `XXX`
//! is the extracted P value and `HHHHHHHH` the hash, both hex, uppercase.

/// Compute the `(P value, hash)` pair for an audio file path.
///
/// `audio_path` is relative to the export root and starts with a leading slash
/// (e.g. `/Contents/Artist/Album/01 Title.mp3`). The path is hashed exactly as
/// supplied: separators, case and Unicode are not normalized, because the
/// hardware hashes the stored path too.
pub fn path_hash(audio_path: &str) -> (u16, u32) {
    let mut hash: u32 = 0;

    for unit in audio_path.encode_utf16() {
        let code_unit = u32::from(unit);
        let temp = hash.wrapping_mul(0x5BC9).wrapping_add(code_unit);
        hash = temp.wrapping_mul(0x93B5).wrapping_add(code_unit);
    }

    let hash_result = hash % 0x30D43;

    // The P value is assembled from scattered bits of the hash, in the bit
    // order the Rekordbox disassembly extracts them.
    let mut p_value: u16 = 0;
    p_value |= (hash_result & 1) as u16; // bit 0 -> bit 0
    p_value |= ((hash_result >> 1) & 2) as u16; // bit 2 -> bit 1
    p_value |= ((hash_result >> 4) & 4) as u16; // bit 6 -> bit 2
    p_value |= ((hash_result >> 4) & 8) as u16; // bit 7 -> bit 3
    p_value |= ((hash_result >> 5) & 0x10) as u16; // bit 9 -> bit 4
    p_value |= ((hash_result >> 8) & 0x20) as u16; // bit 13 -> bit 5
    p_value |= ((hash_result >> 10) & 0x40) as u16; // bit 16 -> bit 6

    (p_value, hash_result)
}

/// Analysis directory for an audio path, relative to the export root and
/// starting with a slash: `/PIONEER/USBANLZ/P051/0001D603`.
pub fn anlz_dir(audio_path: &str) -> String {
    let (p_value, hash) = path_hash(audio_path);
    format!("/PIONEER/USBANLZ/P{p_value:03X}/{hash:08X}")
}

/// Analysis file path for an audio path with the given uppercase extension
/// (`DAT`, `EXT`), e.g. `/PIONEER/USBANLZ/P051/0001D603/ANLZ0000.EXT`.
pub fn anlz_file(audio_path: &str, extension: &str) -> String {
    format!("{}/ANLZ0000.{extension}", anlz_dir(audio_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vectors captured from Rekordbox-written export drives.
    const VECTORS: [(&str, u16, u32); 6] = [
        (
            "/Contents/ARTISTTEST1/ALBUMTEST1/TITLETEST1.mp3",
            0x051,
            0x0001D603,
        ),
        (
            "/Contents/ARTISTTEST2/ALBUMTEST2/TITLETEST2.mp3",
            0x03C,
            0x0000A6CA,
        ),
        (
            "/Contents/ARTISTTEST3/ALBUMTEST3/TITLETEST3.mp3",
            0x045,
            0x0001096B,
        ),
        (
            "/Contents/BROOKLYN BOUNCE/The Theme (Of Progressive Attack)/This Is The Begining.mp3",
            0x04B,
            0x000154A5,
        ),
        (
            "/Contents/Leo Portela/Bon Vibrant - Leo Portela.flac",
            0x00E,
            0x000281CE,
        ),
        (
            "/Contents/Daniela Cast/Jazzy - Daniela Cast.flac",
            0x00A,
            0x0000CC9C,
        ),
    ];

    #[test]
    fn matches_rekordbox_written_directories() {
        for (path, p_value, hash) in VECTORS {
            assert_eq!(path_hash(path), (p_value, hash), "path hash for {path}");
        }
    }

    #[test]
    fn dir_and_file_formatting() {
        assert_eq!(
            anlz_dir("/Contents/ARTISTTEST1/ALBUMTEST1/TITLETEST1.mp3"),
            "/PIONEER/USBANLZ/P051/0001D603"
        );
        assert_eq!(
            anlz_file("/Contents/ARTISTTEST1/ALBUMTEST1/TITLETEST1.mp3", "EXT"),
            "/PIONEER/USBANLZ/P051/0001D603/ANLZ0000.EXT"
        );
    }

    #[test]
    fn supplementary_plane_characters_hash_as_surrogate_pairs() {
        let path = "/Contents/Artist/Album/01 🧪.mp3";
        let (p_value, hash) = path_hash(path);
        assert_eq!(
            anlz_dir(path),
            format!("/PIONEER/USBANLZ/P{p_value:03X}/{hash:08X}")
        );
        assert_ne!(hash, path_hash("/Contents/Artist/Album/01 x.mp3").1);
    }
}
