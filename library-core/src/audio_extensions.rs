//! Canonical list of audio file extensions recognized by the library and GUI.

use std::path::Path;

/// File extensions (without a leading dot) treated as loadable/scannable audio.
pub const SUPPORTED_AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aiff", "aif", "ogg", "m4a", "aac", "opus", "wma", "alac",
];

/// Returns whether `ext` is a supported audio extension (case-insensitive).
pub fn is_supported_audio_extension(ext: &str) -> bool {
    SUPPORTED_AUDIO_EXTENSIONS
        .iter()
        .any(|supported| supported.eq_ignore_ascii_case(ext))
}

/// Returns whether `path` has a supported audio file extension.
pub fn is_supported_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(is_supported_audio_extension)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn recognizes_common_extensions() {
        assert!(is_supported_audio_extension("opus"));
        assert!(is_supported_audio_extension("OPUS"));
        assert!(is_supported_audio_extension("mp3"));
        assert!(!is_supported_audio_extension("txt"));
    }

    #[test]
    fn recognizes_paths() {
        assert!(is_supported_audio_path(Path::new(
            "/music/track.opus"
        )));
        assert!(!is_supported_audio_path(Path::new("/music/readme.txt")));
    }
}
