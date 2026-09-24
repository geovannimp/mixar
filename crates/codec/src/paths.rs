//! Path classification for formats `codec` can open (decode / tags / stems).

use std::path::Path;

/// File extensions (without a leading dot) treated as loadable audio.
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

/// True when `path` looks like an NI Stem container (`.stem.mp4`).
///
/// Filename check only — use for behavioral stem branches (demux, tags, skip Demucs).
pub fn is_stem_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".stem.mp4"))
}

/// Returns whether `path` is a loadable/scannable audio file (including native stems).
pub fn is_loadable_audio_path(path: &Path) -> bool {
    is_supported_audio_path(path) || is_stem_path(path)
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
        assert!(is_supported_audio_path(Path::new("/music/track.opus")));
        assert!(!is_supported_audio_path(Path::new("/music/readme.txt")));
    }

    #[test]
    fn recognizes_stem_mp4_paths() {
        assert!(is_stem_path(Path::new("/music/Track.stem.mp4")));
        assert!(is_stem_path(Path::new("/music/Track.STEM.MP4")));
        assert!(!is_stem_path(Path::new("/music/Track.mp4")));
        assert!(is_loadable_audio_path(Path::new("/music/Track.stem.mp4")));
        assert!(!is_loadable_audio_path(Path::new("/music/readme.txt")));
    }
}
