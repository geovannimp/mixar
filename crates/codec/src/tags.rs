//! Tag and artwork probing for audio files (lofty + stem container overrides).

use std::path::Path;
use std::time::Duration;

use audio_core::secs_to_ms;
use lofty::file::{AudioFile, TaggedFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};

use crate::paths::is_stem_path;
use crate::stem_mp4::stem_container_info;

/// Raw tags and properties read from an audio file.
///
/// Library maps this into `TrackMetadata` (Camelot→musical key, title fallbacks, etc.).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    /// Raw initial-key string from the file (may be Camelot / Open Key).
    pub key: Option<String>,
    pub duration_ms: Option<i32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub bitrate_kbps: Option<u32>,
    pub replaygain_track_gain_db: Option<f64>,
    pub isrc: Option<String>,
}

/// Read tags and properties from an audio file.
///
/// Native `.stem.mp4` files get sample rate / duration overrides from the stem
/// container when available. ReplayGain is omitted for stems (multi-track MP4
/// confuses lofty's probe on a dedicated ReplayGain-only path).
pub fn read_file_tags(path: &Path) -> anyhow::Result<FileTags> {
    if is_stem_path(path) {
        return read_stem_file_tags(path);
    }

    let tagged = Probe::open(path)
        .map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?
        .read()
        .map_err(|e| anyhow::anyhow!("read tags {}: {e}", path.display()))?;

    Ok(file_tags_from_tagged(&tagged))
}

/// Read ReplayGain track gain (dB) when present. Always `None` for `.stem.mp4`.
pub fn read_replaygain_track_gain_db(path: &Path) -> anyhow::Result<Option<f64>> {
    if is_stem_path(path) {
        return Ok(None);
    }
    let tagged = Probe::open(path)
        .map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?
        .read()
        .map_err(|e| anyhow::anyhow!("read tags {}: {e}", path.display()))?;
    Ok(replaygain_track_gain_db(&tagged))
}

/// Read embedded album artwork, if present.
pub fn read_file_artwork(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    let tagged = Probe::open(path)
        .map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?
        .read()
        .map_err(|e| anyhow::anyhow!("read tags {}: {e}", path.display()))?;

    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return Ok(None);
    };
    let Some(picture) = tag.pictures().first() else {
        return Ok(None);
    };
    Ok(Some(picture.data().to_vec()))
}

fn read_stem_file_tags(path: &Path) -> anyhow::Result<FileTags> {
    let mut tags = match Probe::open(path).and_then(|probe| probe.read()) {
        Ok(tagged) => {
            let mut tags = file_tags_from_tagged(&tagged);
            if tags.channels.is_none() {
                tags.channels = Some(2);
            }
            tags
        }
        Err(_) => FileTags {
            title: stem_file_title(path),
            channels: Some(2),
            ..FileTags::default()
        },
    };

    if let Ok((sample_rate, duration_frames)) = stem_container_info(path) {
        tags.sample_rate = Some(sample_rate);
        tags.channels = Some(2);
        if duration_frames > 0 {
            let secs = duration_frames as f64 / f64::from(sample_rate);
            tags.duration_ms = duration_ms(Duration::from_secs_f64(secs));
        }
    }

    if tags.title.is_none() {
        tags.title = stem_file_title(path);
    }

    Ok(tags)
}

fn file_tags_from_tagged(tagged: &TaggedFile) -> FileTags {
    let properties = tagged.properties();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let mut tags = FileTags {
        duration_ms: duration_ms(properties.duration()),
        sample_rate: properties.sample_rate(),
        channels: properties.channels().map(|c| c as u16),
        bitrate_kbps: properties.audio_bitrate(),
        ..FileTags::default()
    };

    if let Some(tag) = tag {
        tags.title = tag.title().map(|s| s.to_string());
        tags.artist = tag.artist().map(|s| s.to_string());
        tags.album = tag.album().map(|s| s.to_string());
        tags.genre = tag.genre().map(|s| s.to_string());

        if let Some(bpm) = tag.get_string(&ItemKey::Bpm) {
            tags.bpm = bpm.parse().ok();
        }
        if tags.bpm.is_none() {
            if let Some(bpm) = tag.get_string(&ItemKey::IntegerBpm) {
                tags.bpm = bpm.parse().ok();
            }
        }

        tags.key = tag.get_string(&ItemKey::InitialKey).map(|s| s.to_string());
        tags.isrc = tag.get_string(&ItemKey::Isrc).map(|s| s.to_string());
    }

    tags.replaygain_track_gain_db = replaygain_track_gain_db(tagged);
    tags
}

fn replaygain_track_gain_db(tagged: &TaggedFile) -> Option<f64> {
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

fn duration_ms(duration: Duration) -> Option<i32> {
    let secs = duration.as_secs_f64();
    if secs > 0.0 {
        Some(secs_to_ms(secs))
    } else {
        None
    }
}

fn stem_file_title(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let lower = name.to_ascii_lowercase();
    let stripped_len = lower.strip_suffix(".stem.mp4")?.len();
    Some(name[..stripped_len].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    use lofty::file::{FileType, TaggedFile};
    use lofty::properties::FileProperties;
    use lofty::tag::{Tag, TagType};

    #[test]
    fn stem_file_title_preserves_case() {
        assert_eq!(
            stem_file_title(Path::new("/music/My Track.stem.mp4")).as_deref(),
            Some("My Track")
        );
        assert_eq!(
            stem_file_title(Path::new("/music/My Track.STEM.MP4")).as_deref(),
            Some("My Track")
        );
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
    fn read_file_tags_missing_file_errors() {
        let err = read_file_tags(Path::new("/no/such/file.mp3")).unwrap_err();
        assert!(err.to_string().contains("open") || err.to_string().contains("No such"));
    }

    #[test]
    fn read_file_tags_rejects_non_audio() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "not audio").unwrap();

        assert!(read_file_tags(&path).is_err());
    }

    #[test]
    fn read_replaygain_skips_stem_paths() {
        assert_eq!(
            read_replaygain_track_gain_db(Path::new("/music/Track.stem.mp4")).unwrap(),
            None
        );
    }
}
