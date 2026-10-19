//! Where exported audio files live on the drive.
//!
//! Device players browse an exported drive by directory, so every file gets a
//! FAT32-safe, unique path below [`CONTENTS_DIR`], written with a leading slash
//! because that is how the device database stores it and what the hardware
//! hashes when it looks for a track's analysis files:
//!
//! - tracks reachable through a folder collection keep their source tree under
//!   `Contents/<folder name>/…`;
//! - everything else is placed by metadata as
//!   `Contents/<artist>/<album>/<file name>`.
//!
//! Names are sanitized (illegal characters dropped, over-long components
//! truncated, reserved names avoided) and de-duplicated case-insensitively,
//! because the target is usually FAT32/exFAT.

use std::collections::HashSet;
use std::path::Path;

/// Root directory of the exported audio files.
pub const CONTENTS_DIR: &str = "Contents";

const UNKNOWN_ARTIST: &str = "Unknown Artist";
const UNKNOWN_ALBUM: &str = "Unknown Album";
const UNKNOWN_FILE: &str = "track";

/// Longest accepted path component, in characters. Well below FAT32's 255-byte
/// limit so DeviceSQL string and path limits stay comfortable.
const MAX_COMPONENT_CHARS: usize = 100;

/// Characters Windows and FAT32 forbid in a file name (the POSIX separator is
/// included because on-drive paths always use `/`).
const ILLEGAL_CHARS: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Names FAT32 reserves regardless of extension.
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Allocates unique device paths for one export run.
#[derive(Debug, Default)]
pub struct DevicePaths {
    used: HashSet<String>,
}

impl DevicePaths {
    /// Keep `path` from being handed to a different track.
    ///
    /// Used for paths earlier runs already occupy: the export merges into the
    /// drive's existing database, so a newly added track must not collide with
    /// an unselected one.
    pub fn reserve(&mut self, path: &str) {
        self.used.insert(path.to_lowercase());
    }

    /// Take a path a track already occupies, so a re-export keeps it instead of
    /// duplicating the file under a ` (2)` suffix.
    pub fn take(&mut self, path: &str) -> String {
        self.used.insert(path.to_lowercase());
        path.to_string()
    }

    /// Device path for a track placed by metadata.
    pub fn artist_album(
        &mut self,
        artist: Option<&str>,
        album: Option<&str>,
        file_name: &str,
    ) -> String {
        let artist = component(artist.unwrap_or_default(), UNKNOWN_ARTIST);
        let album = component(album.unwrap_or_default(), UNKNOWN_ALBUM);
        self.allocate(&format!("{CONTENTS_DIR}/{artist}/{album}"), file_name)
    }

    /// Device path for a track mirrored from a folder collection: the folder's
    /// display name followed by the track's path relative to the collection
    /// root, so the drive reproduces the user's tree.
    pub fn folder_mirror(&mut self, folder_name: &str, relative: &Path) -> String {
        let folder = component(folder_name, UNKNOWN_FILE);
        let mut directory = format!("{CONTENTS_DIR}/{folder}");
        let components: Vec<String> = relative
            .parent()
            .map(|parent| {
                parent
                    .components()
                    .filter_map(|part| part.as_os_str().to_str())
                    .filter(|part| !part.is_empty())
                    .map(|part| component(part, UNKNOWN_FILE))
                    .collect()
            })
            .unwrap_or_default();
        for part in components {
            directory.push('/');
            directory.push_str(&part);
        }

        let file_name = relative
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        self.allocate(&directory, file_name)
    }

    /// Reserve `directory/<file_name>`, disambiguating case-insensitively with
    /// a ` (n)` suffix before the extension.
    ///
    /// Returned paths start with a slash: the device database stores them that
    /// way, and the hardware hashes exactly those bytes when it looks for a
    /// track's analysis files.
    fn allocate(&mut self, directory: &str, file_name: &str) -> String {
        let (stem, extension) = split_extension(file_name);
        let stem = component(stem, UNKNOWN_FILE);
        let mut candidate = format!("/{directory}/{stem}{extension}");
        let mut counter = 2u32;
        while !self.used.insert(candidate.to_lowercase()) {
            candidate = format!("/{directory}/{stem} ({counter}){extension}");
            counter += 1;
        }
        candidate
    }
}

/// Split a file name into stem and lowercase extension (including the dot).
fn split_extension(file_name: &str) -> (&str, String) {
    match file_name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => {
            (stem, format!(".{}", extension.to_ascii_lowercase()))
        }
        _ => (file_name, String::new()),
    }
}

/// Sanitize one path component, falling back to `fallback` when nothing
/// survives.
fn component(value: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_control() || ILLEGAL_CHARS.contains(&ch) {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }

    // Collapse runs of whitespace and trim the trailing dots/spaces FAT32
    // silently strips (which would otherwise change the stored path).
    let mut collapsed = String::with_capacity(out.len());
    let mut last_was_space = false;
    for ch in out.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                collapsed.push(' ');
            }
            last_was_space = true;
        } else {
            collapsed.push(ch);
            last_was_space = false;
        }
    }
    let trimmed = collapsed
        .trim()
        .trim_end_matches(['.', ' '])
        .trim_start_matches('.');

    let mut sanitized: String = trimmed.chars().take(MAX_COMPONENT_CHARS).collect();
    sanitized = sanitized.trim_end_matches(['.', ' ']).to_string();
    if sanitized.is_empty() {
        sanitized = fallback.to_string();
    }

    let upper = sanitized.to_ascii_uppercase();
    if RESERVED_NAMES.contains(&upper.as_str()) {
        sanitized.push('_');
    }
    sanitized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_placement_uses_artist_album_directories() {
        let mut paths = DevicePaths::default();
        assert_eq!(
            paths.artist_album(Some("Slam"), Some("Positive Education"), "Track.mp3"),
            "/Contents/Slam/Positive Education/Track.mp3"
        );
    }

    #[test]
    fn missing_metadata_falls_back_to_placeholders() {
        let mut paths = DevicePaths::default();
        assert_eq!(
            paths.artist_album(None, Some("   "), "  .mp3"),
            "/Contents/Unknown Artist/Unknown Album/track.mp3"
        );
    }

    #[test]
    fn illegal_characters_and_control_characters_are_replaced() {
        let mut paths = DevicePaths::default();
        assert_eq!(
            paths.artist_album(Some("a/b:c*d?e\"f<g>h|i\\j"), None, "x.flac"),
            "/Contents/a b c d e f g h i j/Unknown Album/x.flac"
        );
    }

    #[test]
    fn colliding_names_get_numbered_suffixes() {
        let mut paths = DevicePaths::default();
        let first = paths.artist_album(Some("A"), Some("B"), "Track.mp3");
        let second = paths.artist_album(Some("A"), Some("B"), "Track.mp3");
        let third = paths.artist_album(Some("a"), Some("b"), "track.MP3");
        assert_eq!(first, "/Contents/A/B/Track.mp3");
        assert_eq!(second, "/Contents/A/B/Track (2).mp3");
        // Case-insensitive: FAT32 would treat the third as the second file.
        assert_eq!(third, "/Contents/a/b/track (3).mp3");
    }

    #[test]
    fn folder_mirrors_keep_the_relative_tree() {
        let mut paths = DevicePaths::default();
        assert_eq!(
            paths.folder_mirror("House", Path::new("2024/Deep/Tune 01.wav")),
            "/Contents/House/2024/Deep/Tune 01.wav"
        );
    }

    #[test]
    fn over_long_components_are_truncated() {
        let mut paths = DevicePaths::default();
        let name = "x".repeat(300);
        let path = paths.artist_album(Some(&name), None, &format!("{name}.mp3"));
        let artist = path.split('/').nth(2).expect("artist component");
        assert_eq!(artist.chars().count(), MAX_COMPONENT_CHARS);
    }

    #[test]
    fn reserved_device_names_are_escaped() {
        let mut paths = DevicePaths::default();
        assert_eq!(
            paths.artist_album(Some("NUL"), None, "con.mp3"),
            "/Contents/NUL_/Unknown Album/con_.mp3"
        );
    }
}
