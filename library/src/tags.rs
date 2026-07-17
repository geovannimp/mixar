//! File tag reading for native library import.

use std::path::Path;
use std::time::Duration;

use library_core::TrackMetadata;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};

/// Read metadata tags from an audio file.
pub fn read_tags(path: &Path) -> library_core::Result<TrackMetadata> {
    let tagged = Probe::open(path)
        .map_err(|e| io_backend(format!("open {}: {e}", path.display())))?
        .read()
        .map_err(|e| io_backend(format!("read tags {}: {e}", path.display())))?;

    let properties = tagged.properties();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let mut metadata = TrackMetadata {
        duration_secs: duration_secs(properties.duration()),
        sample_rate: properties.sample_rate(),
        channels: properties.channels().map(|c| c as u16),
        bitrate_kbps: properties.audio_bitrate(),
        ..TrackMetadata::default()
    };

    if let Some(tag) = tag {
        metadata.title = tag.title().map(|s| s.to_string());
        metadata.artist = tag.artist().map(|s| s.to_string());
        metadata.album = tag.album().map(|s| s.to_string());
        metadata.genre = tag.genre().map(|s| s.to_string());

        if let Some(bpm) = tag.get_string(&lofty::tag::ItemKey::Bpm) {
            metadata.bpm = bpm.parse().ok();
        }
        if metadata.bpm.is_none() {
            if let Some(bpm) = tag.get_string(&lofty::tag::ItemKey::IntegerBpm) {
                metadata.bpm = bpm.parse().ok();
            }
        }

        metadata.key = tag
            .get_string(&lofty::tag::ItemKey::InitialKey)
            .map(|s| normalize_key_notation(&s));
    }

    if metadata.title.is_none() {
        metadata.title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
    }

    Ok(metadata)
}

/// Read the ReplayGain track gain tag, in decibels, when present and valid.
pub(crate) fn read_replaygain_track_gain_db(path: &Path) -> library_core::Result<Option<f64>> {
    let tagged = Probe::open(path)
        .map_err(|e| io_backend(format!("open {}: {e}", path.display())))?
        .read()
        .map_err(|e| io_backend(format!("read tags {}: {e}", path.display())))?;
    Ok(replaygain_track_gain_db(&tagged))
}

fn replaygain_track_gain_db(tagged: &impl TaggedFileExt) -> Option<f64> {
    tagged.tags().iter().find_map(|tag| {
        tag.get_string(&ItemKey::ReplayGainTrackGain)
            .and_then(parse_replaygain_track_gain_db)
    })
}

fn parse_replaygain_track_gain_db(raw: &str) -> Option<f64> {
    let normalized = raw.trim().replace('−', "-");
    let value = normalized
        .strip_suffix("dB")
        .or_else(|| normalized.strip_suffix("db"))
        .unwrap_or(&normalized)
        .trim();
    value.parse().ok()
}

/// Convert Camelot/Open Key codes to musical notation; pass through other values.
fn normalize_key_notation(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() < 2 {
        return trimmed.to_string();
    }
    let upper = trimmed.to_uppercase();
    let Some(code) = upper.strip_suffix('A').or_else(|| upper.strip_suffix('B')) else {
        return trimmed.to_string();
    };
    let Ok(num) = code.parse::<usize>() else {
        return trimmed.to_string();
    };
    if !(1..=12).contains(&num) {
        return trimmed.to_string();
    }
    let minor = upper.ends_with('B');
    camelot_to_musical(num, minor).unwrap_or_else(|| trimmed.to_string())
}

fn camelot_to_musical(code: usize, minor: bool) -> Option<String> {
    const MAJOR: [&str; 12] = [
        "C", "G", "D", "A", "E", "B", "F#", "C#", "G#", "D#", "A#", "F",
    ];
    const MINOR: [&str; 12] = [
        "Am", "Em", "Bm", "F#m", "C#m", "G#m", "D#m", "A#m", "Fm", "Cm", "Gm", "Dm",
    ];
    let idx = code - 1;
    if minor {
        MINOR.get(idx).map(|s| (*s).to_string())
    } else {
        MAJOR.get(idx).map(|s| (*s).to_string())
    }
}

/// Read embedded album artwork from an audio file, if present.
pub fn read_artwork(path: &Path) -> library_core::Result<Option<Vec<u8>>> {
    let tagged = Probe::open(path)
        .map_err(|e| io_backend(format!("open {}: {e}", path.display())))?
        .read()
        .map_err(|e| io_backend(format!("read tags {}: {e}", path.display())))?;

    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(None);
    };

    let Some(picture) = tag.pictures().first() else {
        return Ok(None);
    };

    Ok(Some(picture.data().to_vec()))
}

fn duration_secs(duration: Duration) -> Option<f64> {
    let secs = duration.as_secs_f64();
    if secs > 0.0 {
        Some(secs)
    } else {
        None
    }
}

fn io_backend(message: String) -> library_core::LibraryError {
    library_core::LibraryError::Backend {
        backend: "library",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use lofty::file::{FileType, TaggedFile};
    use lofty::properties::FileProperties;
    use lofty::tag::{Tag, TagType};

    #[test]
    fn normalize_camelot_key_to_musical() {
        assert_eq!(camelot_to_musical(8, false), Some("C#".into()));
        assert_eq!(camelot_to_musical(1, true), Some("Am".into()));
        assert_eq!(normalize_key_notation("8A"), "C#");
        assert_eq!(normalize_key_notation("F#m"), "F#m");
    }

    #[test]
    fn parses_replaygain_track_gain_values() {
        assert_eq!(parse_replaygain_track_gain_db("+3.20 dB"), Some(3.2));
        assert_eq!(parse_replaygain_track_gain_db("-1.5 dB"), Some(-1.5));
        assert_eq!(parse_replaygain_track_gain_db("−1.5 dB"), Some(-1.5));
        assert_eq!(parse_replaygain_track_gain_db("not a gain"), None);
    }

    #[test]
    fn reads_first_valid_replaygain_from_secondary_tag() {
        let mut primary = Tag::new(TagType::Id3v2);
        assert!(primary.insert_text(ItemKey::ReplayGainTrackGain, "not a gain".to_string()));
        let mut secondary = Tag::new(TagType::Ape);
        assert!(secondary.insert_text(ItemKey::ReplayGainTrackGain, "+3.20 dB".to_string()));
        let tagged = TaggedFile::new(
            FileType::Mpeg,
            FileProperties::default(),
            vec![primary, secondary],
        );

        assert_eq!(replaygain_track_gain_db(&tagged), Some(3.2));
    }

    #[test]
    fn read_tags_missing_file_errors() {
        let err = read_tags(Path::new("/no/such/file.mp3")).unwrap_err();
        assert!(matches!(
            err,
            library_core::LibraryError::Backend {
                backend: "library",
                ..
            }
        ));
    }

    #[test]
    fn read_tags_rejects_non_audio() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "not audio").unwrap();

        let err = read_tags(&path).unwrap_err();
        assert!(matches!(
            err,
            library_core::LibraryError::Backend {
                backend: "library",
                ..
            }
        ));
    }
}
