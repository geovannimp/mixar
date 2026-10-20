//! Tag and artwork probing for audio files (lofty + stem container overrides).
//!
//! Artwork can be read as the original embedded bytes ([`read_file_artwork`]),
//! probed together with the container's bit depth ([`read_file_artwork_probe`]),
//! or re-encoded to a small JPEG thumbnail ([`read_file_artwork_thumbnail`] /
//! [`encode_artwork_thumbnail`]), which is what consumers that only show or ship
//! a cover want. Decoding runs under explicit [`Limits`]: cover art is untrusted
//! input and must never be able to allocate unbounded memory.

use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

use audio_core::secs_to_ms;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageReader, Limits, Rgb, RgbImage};
use lofty::file::{AudioFile, TaggedFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};

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
    /// Source bit depth in bits, when the container reports it. Lossy formats
    /// usually do not (the decoder's own output is always `f32`).
    pub bit_depth: Option<u8>,
    pub replaygain_track_gain_db: Option<f64>,
    pub isrc: Option<String>,
    /// Star rating 1–5, from `POPM` (ID3) or `RATING`/`rate` (text tags).
    pub rating: Option<u8>,
    pub comment: Option<String>,
    /// Release year, from `YEAR` or the leading four digits of a date field.
    pub year: Option<u16>,
    pub track_number: Option<u16>,
    pub disc_number: Option<u16>,
    /// Raw `COLOR` tag text; the library parses it into its closed label set.
    pub color: Option<String>,
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
    Ok(read_file_artwork_probe(path)?.picture)
}

/// Embedded picture bytes plus container facts, from a single tag probe.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArtworkProbe {
    /// Raw embedded picture bytes, when the file carries one.
    pub picture: Option<Vec<u8>>,
    /// Source bit depth in bits, when the container reports it.
    pub bit_depth: Option<u8>,
}

/// Read the embedded picture and the source bit depth with a single tag probe.
///
/// Callers that want both — export reads a file's cover *and* its bit depth for
/// every track — would otherwise parse the container twice. `Err` only when the
/// file cannot be probed at all; a file that simply has no artwork is `Ok` with
/// [`ArtworkProbe::picture`] `None`.
pub fn read_file_artwork_probe(path: &Path) -> anyhow::Result<ArtworkProbe> {
    let tagged = Probe::open(path)
        .map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?
        .read()
        .map_err(|e| anyhow::anyhow!("read tags {}: {e}", path.display()))?;

    Ok(ArtworkProbe {
        picture: first_picture(&tagged),
        bit_depth: tagged.properties().bit_depth(),
    })
}

/// Raw bytes of the first embedded picture, if the file carries one.
///
/// Primary tag first, then any tag: the same pick the artwork readers have
/// always made.
fn first_picture(tagged: &TaggedFile) -> Option<Vec<u8>> {
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    let picture = tag.pictures().first()?;
    Some(picture.data().to_vec())
}

/// Quality of the JPEG re-encode [`read_file_artwork_thumbnail`] performs.
///
/// 85 is the usual "no visible loss" point and keeps a small thumbnail at a few
/// kilobytes, which matters because device formats store a cover per track.
const THUMBNAIL_JPEG_QUALITY: u8 = 85;

/// Hard ceiling on either edge of an embedded cover we are willing to decode.
///
/// Embedded covers are untrusted input: a crafted PNG or BMP header declares its
/// dimensions in the first few bytes, and a decoder sizes its buffers from that
/// declaration alone. Real covers are far smaller — even high-resolution album
/// scans stay near 3000 px — so 8000 px leaves room for double-page artwork while
/// bounding a single decode to a few hundred megabytes.
const MAX_ARTWORK_EDGE_PX: u32 = 8_000;

/// Ceiling on the bytes one cover decode may allocate in total (256 MiB).
///
/// The budget covers the whole decode — decoder scratch plus the output buffer —
/// so it rejects a picture whose *area* is enormous even when neither edge
/// exceeds [`MAX_ARTWORK_EDGE_PX`]: roughly 25 megapixels of RGB, or 16 megapixels
/// of RGBA, fit; anything bigger is treated as a decompression bomb. Skipping a
/// cover is always safe, because artwork is decoration, never required data.
const MAX_ARTWORK_ALLOC_BYTES: u64 = 256 * 1024 * 1024;

/// Decoding limits for embedded artwork (see the two constants above).
fn artwork_limits() -> Limits {
    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(MAX_ARTWORK_EDGE_PX);
    limits.max_image_height = Some(MAX_ARTWORK_EDGE_PX);
    limits.max_alloc = Some(MAX_ARTWORK_ALLOC_BYTES);
    limits
}

/// Read the embedded picture and re-encode it as a JPEG whose longest edge is at
/// most `max_edge` pixels.
///
/// Aspect ratio is preserved; images are never upscaled (a smaller cover comes
/// back unchanged in size). Alpha is flattened onto white, because JPEG has none.
/// `Ok(None)` when the file has no picture — or one we cannot decode, since an
/// exotic cover must not fail an import or an export.
///
/// `max_edge == 0` means "no thumbnail": it returns `Ok(None)` without opening
/// the file.
pub fn read_file_artwork_thumbnail(path: &Path, max_edge: u32) -> anyhow::Result<Option<Vec<u8>>> {
    if max_edge == 0 {
        return Ok(None);
    }

    let tagged = Probe::open(path)
        .map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?
        .read()
        .map_err(|e| anyhow::anyhow!("read tags {}: {e}", path.display()))?;

    Ok(first_picture(&tagged).and_then(|picture| encode_artwork_thumbnail(&picture, max_edge)))
}

/// Re-encode raw picture bytes as a JPEG thumbnail.
///
/// This is the half of [`read_file_artwork_thumbnail`] that does not read tags,
/// for callers that already hold the picture bytes — and maybe want to cache the
/// result per cover instead of decoding it again per track.
///
/// `None` when the bytes are not an image we can decode (JPEG, PNG, BMP), when
/// they are too damaged for the decoder to reconstruct, when they exceed the
/// limits above, or when `max_edge == 0`.
pub fn encode_artwork_thumbnail(picture: &[u8], max_edge: u32) -> Option<Vec<u8>> {
    if max_edge == 0 {
        return None;
    }

    let decoded = decode_picture(picture)?;

    let longest = decoded.width().max(decoded.height());
    let scaled = if longest > max_edge {
        let ratio = f64::from(max_edge) / f64::from(longest);
        let width = ((f64::from(decoded.width()) * ratio).round() as u32).max(1);
        let height = ((f64::from(decoded.height()) * ratio).round() as u32).max(1);
        // Triangle: cover art at thumbnail size does not benefit from a wider
        // kernel, and this keeps the cost of a full-resolution cover low.
        decoded.resize(width, height, FilterType::Triangle)
    } else {
        decoded
    };

    let rgb = flatten_onto_white(&scaled);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, THUMBNAIL_JPEG_QUALITY)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .ok()?;
    Some(jpeg)
}

/// Decode picture bytes under [`artwork_limits`].
///
/// `None` when the format cannot be guessed from the bytes, when they are
/// damaged, or when the picture breaks a limit — callers treat all three the
/// same, as "no artwork".
fn decode_picture(picture: &[u8]) -> Option<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(picture))
        .with_guessed_format()
        .ok()?;
    reader.limits(artwork_limits());
    reader.decode().ok()
}

/// Composite an image over opaque white and drop alpha.
///
/// JPEG has no alpha channel; transparent cover pixels are meant to read as
/// paper-white, not as the black a naive channel drop would give.
fn flatten_onto_white(image: &DynamicImage) -> RgbImage {
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let pixel = rgba.get_pixel(x, y);
        let alpha = u32::from(pixel[3]);
        let over_white =
            |channel: u8| ((u32::from(channel) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
        Rgb([
            over_white(pixel[0]),
            over_white(pixel[1]),
            over_white(pixel[2]),
        ])
    })
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
        bit_depth: properties.bit_depth(),
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
        tags.rating = rating_from_tag(tag);
        tags.comment = tag.get_string(&ItemKey::Comment).map(|s| s.to_string());
        tags.year = year_from_tag(tag);
        tags.track_number = number_from_tag(tag, &ItemKey::TrackNumber);
        tags.disc_number = number_from_tag(tag, &ItemKey::DiscNumber);
        tags.color = tag.get_string(&ItemKey::Color).map(|s| s.to_string());
    }

    tags.replaygain_track_gain_db = replaygain_track_gain_db(tagged);
    tags
}

/// Star rating from the format's rating field.
///
/// There is no single rating scale. ID3v2 keeps `POPM` as raw bytes under
/// [`ItemKey::Popularimeter`] (email + 0–255 value); Vorbis and MP4 map the
/// same key to text. All observed conventions are accepted:
///
/// - `0.0`–`1.0` with a decimal point: Quod Libet / Picard Vorbis
///   (`0.8` = four stars);
/// - `1`–`5`: a bare star count (Traktor-style tools and older Mixar files);
/// - `6`–`100`: percentage-ish scales (`80` = four stars);
/// - `101`–`255`: the raw POPM byte scale (`204` = four stars).
fn rating_from_tag(tag: &Tag) -> Option<u8> {
    if let Some(text) = tag.get_string(&ItemKey::Popularimeter) {
        return rating_from_text(text);
    }
    let bytes = tag.get_binary(&ItemKey::Popularimeter, false)?;
    // POPM layout: email, NUL, rating, play counter.
    let nul = bytes.iter().position(|byte| *byte == 0)?;
    let email = String::from_utf8_lossy(&bytes[..nul]);
    let rating = *bytes.get(nul + 1)?;
    popm_to_stars(&email, rating)
}

fn rating_from_text(raw: &str) -> Option<u8> {
    let trimmed = raw.trim();
    let value: f64 = trimmed.parse().ok()?;
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    // 6–19 and 5.1–5.9 fit no supported scale (a 0–10 scale would need 20
    // steps); treating them as the lowest star would silently rewrite them.
    if value > 5.0 && value < 20.0 {
        return None;
    }
    // Only a decimal point marks the normalized 0.0–1.0 convention; `"1"` is
    // one star while `"1.0"` is five (Picard writes `1.0`).
    let stars = if trimmed.contains('.') && value <= 1.0 {
        value * 5.0
    } else if value <= 5.0 {
        value
    } else if value <= 100.0 {
        value / 20.0
    } else {
        return Some(linear_popm_scale_to_stars(value));
    };
    Some(stars.round().clamp(1.0, 5.0) as u8)
}

/// Map a POPM byte onto 1–5 stars, honouring the writer's scale.
///
/// Traktor and MusicBrainz Picard use `round(star * 51)`; MusicBee, Windows
/// Media Player and Serato use `1, 64, 128, 196, 255`. The email field names
/// the writer (POPM allows several ratings per file), which is the only
/// reliable way to tell the two apart.
fn popm_to_stars(email: &str, rating: u8) -> Option<u8> {
    if rating == 0 {
        return None;
    }
    let email = email.trim().to_ascii_lowercase();
    if matches!(
        email.as_str(),
        "musicbee" | "windows media player 9 series" | "serato"
    ) {
        let stars = if rating <= 1 {
            1
        } else if rating <= 64 {
            2
        } else if rating <= 128 {
            3
        } else if rating <= 196 {
            4
        } else {
            5
        };
        return Some(stars);
    }
    // Traktor (`traktor@native-instruments.de`), Picard
    // (`users@musicbrainz.org`) and unknown writers use the linear 51 scale.
    Some(linear_popm_scale_to_stars(f64::from(rating)))
}

/// Map the linear POPM 0–255 scale onto 1–5 stars.
fn linear_popm_scale_to_stars(value: f64) -> u8 {
    (value / 51.0).round().clamp(1.0, 5.0) as u8
}

/// Release year from `YEAR`, else the leading four digits of a date tag.
fn year_from_tag(tag: &Tag) -> Option<u16> {
    let raw = tag
        .get_string(&ItemKey::Year)
        .or_else(|| tag.get_string(&ItemKey::RecordingDate))
        .or_else(|| tag.get_string(&ItemKey::ReleaseDate))?;
    let leading: String = raw.trim().chars().take(4).collect();
    leading.parse().ok().filter(|year| *year > 0)
}

/// A track/disc number tag, ignoring non-numeric and zero values.
fn number_from_tag(tag: &Tag, key: &ItemKey) -> Option<u16> {
    let raw = tag.get_string(key)?;
    // "3/12" (TRCK/TPOS with a total) carries the number in its leading digits.
    crate::leading_u16(raw).filter(|number| *number > 0)
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

    use lofty::config::WriteOptions;
    use lofty::file::{FileType, TaggedFile};
    use lofty::picture::{MimeType, Picture, PictureType};
    use lofty::properties::FileProperties;
    use lofty::tag::{ItemValue, Tag, TagItem, TagType};

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
    fn rating_scales_map_onto_stars() {
        // Text: normalized 0.0–1.0, bare stars, percent, POPM byte.
        assert_eq!(rating_from_text("0.8"), Some(4));
        assert_eq!(rating_from_text("0.2"), Some(1));
        assert_eq!(rating_from_text("1.0"), Some(5));
        assert_eq!(rating_from_text("1"), Some(1));
        assert_eq!(rating_from_text("4"), Some(4));
        assert_eq!(rating_from_text("80"), Some(4));
        assert_eq!(rating_from_text("204"), Some(4));
        assert_eq!(rating_from_text("0"), None);
        assert_eq!(rating_from_text("6"), None);
        assert_eq!(rating_from_text("19"), None);
        assert_eq!(rating_from_text("5.5"), None);
        assert_eq!(rating_from_text("bogus"), None);

        // POPM: the email selects the writer's scale.
        assert_eq!(popm_to_stars("traktor@native-instruments.de", 51), Some(1));
        assert_eq!(popm_to_stars("traktor@native-instruments.de", 153), Some(3));
        assert_eq!(popm_to_stars("users@musicbrainz.org", 102), Some(2));
        assert_eq!(popm_to_stars("MusicBee", 64), Some(2));
        assert_eq!(popm_to_stars("Windows Media Player 9 Series", 196), Some(4));
        assert_eq!(popm_to_stars("Serato", 1), Some(1));
        // Unknown writers default to the linear 51 scale.
        assert_eq!(popm_to_stars("", 64), Some(1));
        assert_eq!(popm_to_stars("", 204), Some(4));
        assert_eq!(popm_to_stars("", 0), None);
    }

    #[test]
    fn rating_from_tag_reads_text_and_binary_popm() {
        let mut text = Tag::new(TagType::VorbisComments);
        assert!(text.insert_text(ItemKey::Popularimeter, "0.8".into()));
        assert_eq!(rating_from_tag(&text), Some(4));

        let mut binary = Tag::new(TagType::Id3v2);
        binary.insert(TagItem::new(
            ItemKey::Popularimeter,
            ItemValue::Binary(vec![
                b't', b'r', b'a', b'k', b't', b'o', b'r', b'@', 0, 204, 0, 0, 0, 0,
            ]),
        ));
        assert_eq!(rating_from_tag(&binary), Some(4));

        let mut no_nul = Tag::new(TagType::Id3v2);
        no_nul.insert(TagItem::new(
            ItemKey::Popularimeter,
            ItemValue::Binary(vec![b'x', 204]),
        ));
        assert_eq!(rating_from_tag(&no_nul), None);

        let mut no_rating = Tag::new(TagType::Id3v2);
        no_rating.insert(TagItem::new(
            ItemKey::Popularimeter,
            ItemValue::Binary(vec![b'x', 0]),
        ));
        assert_eq!(rating_from_tag(&no_rating), None);
    }

    #[test]
    fn year_comes_from_year_or_the_leading_date_digits() {
        let mut tag = Tag::new(TagType::VorbisComments);
        assert!(tag.insert_text(ItemKey::Year, "1999".to_string()));
        assert_eq!(year_from_tag(&tag), Some(1999));

        let mut tag = Tag::new(TagType::Id3v2);
        assert!(tag.insert_text(ItemKey::RecordingDate, "2026-10-19".to_string()));
        assert_eq!(year_from_tag(&tag), Some(2026));
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

    /// 1s of 16-bit mono PCM at 8 kHz: a WAV lofty probes fine.
    fn write_wav(path: &Path) {
        let sample_rate = 8_000u32;
        let pcm = vec![0u8; sample_rate as usize * 2];
        let data_size = pcm.len() as u32;
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_size).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&(sample_rate * 2).to_le_bytes()).unwrap();
        file.write_all(&2u16.to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();
        file.write_all(&pcm).unwrap();
    }

    /// Embed `picture` as the front cover of a WAV (`id3 ` chunk → ID3v2 APIC).
    ///
    /// Asserts lofty reads the picture back, so a container that silently drops
    /// it cannot make the "no artwork" cases pass for the wrong reason.
    fn write_wav_with_picture(path: &Path, mime: MimeType, picture: Vec<u8>) {
        write_wav(path);

        let mut tagged = Probe::open(path).unwrap().read().unwrap();
        if tagged.primary_tag().is_none() {
            tagged.insert_tag(Tag::new(TagType::Id3v2));
        }
        tagged.primary_tag_mut().expect("id3v2 tag").set_picture(
            0,
            Picture::new_unchecked(PictureType::CoverFront, Some(mime), None, picture),
        );
        tagged
            .save_to_path(path, WriteOptions::default())
            .expect("save artwork tag");

        let reread = Probe::open(path).unwrap().read().unwrap();
        assert_eq!(
            reread.primary_tag().expect("id3v2 tag").pictures().len(),
            1,
            "lofty must read back the picture it wrote"
        );
    }

    fn encode(image: DynamicImage, format: image::ImageFormat) -> Vec<u8> {
        let mut bytes = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut bytes), format)
            .expect("encode test cover");
        bytes
    }

    fn png_bytes(width: u32, height: u32) -> Vec<u8> {
        let rgb = RgbImage::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        encode(DynamicImage::ImageRgb8(rgb), image::ImageFormat::Png)
    }

    fn transparent_red_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let rgba = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 0, 0, 0]));
        encode(DynamicImage::ImageRgba8(rgba), image::ImageFormat::Png)
    }

    #[test]
    fn thumbnail_resizes_a_large_cover_keeping_aspect() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.wav");
        write_wav_with_picture(&path, MimeType::Png, png_bytes(300, 200));

        let jpeg = read_file_artwork_thumbnail(&path, 80)
            .unwrap()
            .expect("thumbnail");
        assert_eq!(jpeg[..2], [0xFF, 0xD8], "encoded bytes are a JPEG (SOI)");

        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (80, 53));
        assert!(decoded.width().max(decoded.height()) <= 80);
    }

    #[test]
    fn thumbnail_does_not_upscale_a_small_cover() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.wav");
        write_wav_with_picture(&path, MimeType::Png, png_bytes(40, 40));

        let jpeg = read_file_artwork_thumbnail(&path, 80)
            .unwrap()
            .expect("thumbnail");
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (40, 40));
    }

    #[test]
    fn thumbnail_flattens_alpha_onto_white() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.wav");
        write_wav_with_picture(&path, MimeType::Png, transparent_red_png_bytes(200, 100));

        let jpeg = read_file_artwork_thumbnail(&path, 80)
            .unwrap()
            .expect("thumbnail");
        let decoded = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        // Fully transparent red composites to white, not to black.
        let pixel = decoded.get_pixel(decoded.width() / 2, decoded.height() / 2);
        assert!(
            pixel.0.iter().all(|channel| *channel >= 240),
            "transparent pixels must flatten onto white, got {pixel:?}"
        );
    }

    #[test]
    fn thumbnail_of_a_coverless_file_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bare.wav");
        write_wav(&path);

        assert!(read_file_artwork_thumbnail(&path, 80).unwrap().is_none());
    }

    #[test]
    fn thumbnail_of_an_undecodable_picture_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("garbage.wav");
        write_wav_with_picture(
            &path,
            MimeType::Jpeg,
            vec![0xFF, 0xD8, 0xFF, 0xD9, 0x01, 0x02, 0x03],
        );

        assert!(read_file_artwork_thumbnail(&path, 80).unwrap().is_none());
    }

    #[test]
    fn zero_max_edge_skips_thumbnailing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.wav");
        write_wav_with_picture(&path, MimeType::Png, png_bytes(300, 200));

        assert!(read_file_artwork_thumbnail(&path, 0).unwrap().is_none());
    }

    #[test]
    fn read_file_artwork_keeps_the_original_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.wav");
        let original = png_bytes(300, 200);
        write_wav_with_picture(&path, MimeType::Png, original.clone());

        assert_eq!(
            read_file_artwork(&path).unwrap().as_deref(),
            Some(&original[..])
        );
    }

    #[test]
    fn artwork_probe_reports_cover_and_bit_depth_together() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.wav");
        let original = png_bytes(300, 200);
        write_wav_with_picture(&path, MimeType::Png, original.clone());

        let probe = read_file_artwork_probe(&path).unwrap();
        assert_eq!(probe.picture.as_deref(), Some(&original[..]));
        // One probe answers what the two readers would have answered separately.
        assert_eq!(probe.picture, read_file_artwork(&path).unwrap());
        assert_eq!(probe.bit_depth, read_file_tags(&path).unwrap().bit_depth);

        let bare = dir.path().join("bare.wav");
        write_wav(&bare);
        let probe = read_file_artwork_probe(&bare).unwrap();
        assert!(probe.picture.is_none());
    }

    /// One PNG chunk: length, type, data, CRC over type and data.
    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut crc_input = kind.to_vec();
        crc_input.extend_from_slice(data);

        let mut chunk = Vec::from(u32::try_from(data.len()).unwrap().to_be_bytes());
        chunk.extend_from_slice(&crc_input);
        chunk.extend_from_slice(&png_crc32(&crc_input).to_be_bytes());
        chunk
    }

    /// CRC-32 as PNG defines it (reflected, polynomial `0xEDB8_8320`).
    fn png_crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// A PNG that *declares* `width`×`height` and carries no pixels.
    ///
    /// This is what a decompression bomb looks like: a few dozen bytes that ask a
    /// decoder for gigabytes. The chunk CRCs are correct, so a decoder cannot
    /// dismiss it as corrupt before it looks at the declared size; the empty
    /// `IDAT`/`IEND` are only there so the stream parses as a complete PNG.
    fn png_header_only(width: u32, height: u32) -> Vec<u8> {
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB, no interlace

        let mut png = Vec::from([0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        png.extend_from_slice(&png_chunk(b"IHDR", &ihdr));
        png.extend_from_slice(&png_chunk(b"IDAT", &[]));
        png.extend_from_slice(&png_chunk(b"IEND", &[]));
        png
    }

    #[test]
    fn oversized_cover_is_refused_by_the_decode_limits() {
        let bomb = png_header_only(60_000, 60_000);

        // The header is genuine: a reader without dimension limits takes the
        // declared size straight from it, and would then allocate 10 GB for it.
        let declared = image::ImageReader::new(std::io::Cursor::new(&bomb))
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap();
        assert_eq!(declared, (60_000, 60_000));

        // Under our limits the very same bytes are refused, and the refusal is a
        // limit (not a corrupt-header error).
        let mut reader = image::ImageReader::new(std::io::Cursor::new(&bomb))
            .with_guessed_format()
            .unwrap();
        reader.limits(artwork_limits());
        let error = reader
            .decode()
            .expect_err("a 60k×60k cover must not decode");
        assert!(
            matches!(error, image::ImageError::Limits(_)),
            "expected a limit error, got {error:?}"
        );

        assert_eq!(encode_artwork_thumbnail(&bomb, 80), None);

        // A file carrying it as its cover reads as coverless, not as broken.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bomb.wav");
        write_wav_with_picture(&path, MimeType::Png, bomb);
        assert!(read_file_artwork_thumbnail(&path, 80).unwrap().is_none());
    }

    #[test]
    fn a_multi_megapixel_cover_still_thumbnails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cover.wav");
        // 3 megapixels: far beyond a thumbnail, well inside the limits, and the
        // size real high-resolution album scans actually reach.
        write_wav_with_picture(&path, MimeType::Png, png_bytes(2_000, 1_500));

        let jpeg = read_file_artwork_thumbnail(&path, 80)
            .unwrap()
            .expect("limits must not refuse a normal cover");
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (80, 60));
    }

    #[test]
    fn encode_artwork_thumbnail_without_an_edge_is_none() {
        assert_eq!(encode_artwork_thumbnail(&png_bytes(300, 200), 0), None);
    }
}
