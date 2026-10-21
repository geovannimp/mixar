//! Track-information write-back into file tags (lofty).
//!
//! Mixar keeps track information authoritative in `library.db`; this module
//! mirrors it into the file's own tags so other software sees the same values.
//! It is the only place Mixar mutates a music file, so it is:
//!
//! - **opt-in** (`LibraryConfig::tag_write_back`),
//! - **field-scoped**: only the fields the caller passes are touched; anything
//!   else in the file (pictures, ReplayGain, Serato frames, foreign keys) is
//!   left exactly as it was,
//! - **idempotent**: fields already carrying the wanted value are left alone, and
//!   a file with nothing to change is never rewritten.
//!
//! Field per container (lofty/TagLib mapping, matching MusicBrainz Picard):
//!
//! |           | ID3v2 (mp3/wav/aiff/aac) | Vorbis comments (flac/ogg/opus) | MP4 `ilst` (m4a)                    | APE (ape/wv/mpc) |
//! |-----------|--------------------------|---------------------------------|-------------------------------------|------------------|
//! | Title     | `TIT2`                   | `TITLE`                         | `©nam`                              | `Title`          |
//! | Artist    | `TPE1`                   | `ARTIST`                        | `©ART`                              | `Artist`         |
//! | Album     | `TALB`                   | `ALBUM`                         | `©alb`                              | `Album`          |
//! | Genre     | `TCON`                   | `GENRE`                         | `©gen`                              | `Genre`          |
//! | BPM       | `TBPM`, **integer**      | `BPM`, text                     | `----:com.apple.iTunes:BPM`, text   | unsupported      |
//! | Key       | `TKEY`                   | `INITIALKEY`                    | `----:com.apple.iTunes:initialkey`  | unsupported      |
//! | Rating    | unsupported¹             | `RATING`, 0.0–1.0               | `rate`, integer 1–5                 | unsupported      |
//! | Comment   | `COMM`                   | `COMMENT`                       | `©cmt`                              | `Comment`        |
//! | Year      | `TDRC`                   | `YEAR`                          | `©day`                              | `Year`           |
//! | Track no. | `TRCK`                   | `TRACKNUMBER`                   | `trkn`                              | `Track`          |
//! | Disc no.  | `TPOS`                   | `DISCNUMBER`                    | `disk`                              | `Disc`           |
//! | Color     | `COLOR`                  | `COLOR`                         | `----:com.apple.iTunes:COLOR`       | `Color`          |
//!
//! ¹ ID3v2 keeps the rating as a binary `POPM` frame. lofty 0.22 retains that
//! frame in the tag's format-specific remainder instead of exposing it through
//! [`lofty::tag::Tag`], so Mixar reports the field `Unsupported` on ID3 rather
//! than writing a POPM it could not read back (and would therefore rewrite on
//! every run).
//!
//! Reading is wider than writing: Vorbis/MP4 text is accepted as normalized
//! `0.0–1.0`, stars `1–5`, a 0–100 scale, or a raw POPM byte; a binary POPM is
//! mapped using its email (`traktor@native-instruments.de` and
//! `users@musicbrainz.org` use the 51-step scale; MusicBee/Windows Media
//! Player/Serato use 1/64/128/196/255).
//!
//! The BPM/key rows follow Mixxx's write-back (same Vorbis keys, same MP4
//! freeform atoms), so DJ software reads them back.
//!
//! Lossiness:
//!
//! - ID3v2 `TBPM` is an integer frame, so a fractional BPM is rounded (reported
//!   as [`FieldWrite::Rounded`]). Mixxx and Mixed In Key round the same way. The
//!   database keeps the exact value; `library.db` stays authoritative.
//! - MP4 gets the freeform `BPM` atom rather than the integer `tmpo` atom: the
//!   freeform field is what Mixed In Key / Serato / Traktor write and Mixxx reads
//!   first, and lofty can only emit text atoms.
//! - APE tags have no BPM/key mapping in the TagLib/lofty table, so those two
//!   fields are reported [`FieldWrite::Unsupported`] there; the text fields work.
//! - Only the recommended keys above are written; legacy aliases (`KEY`,
//!   `TEMPO`, `----:com.apple.iTunes:KEY`) keep whatever they held.
//!
//! Two file shapes are reported [`FieldWrite::Refused`] and left byte-identical
//! rather than risk damage:
//!
//! - Native NI Stem containers (`*.stem.mp4`) are multi-track MP4s whose `stem`
//!   manifest Mixar does not own.
//! - FLAC whose only metadata block is STREAMINFO (e.g. after
//!   `metaflac --remove-all --dont-use-padding`). lofty 0.22 appends the new
//!   comment block *after* a STREAMINFO that is still flagged last, which
//!   decoders then read as audio. Real encoders always emit a comment block, so
//!   this only affects stripped files.
//!
//! One more shape fails as a plain I/O error (callers log and skip it; the file
//! is never touched because the write is the last step): MP4 whose top-level
//! `mdat` uses a 64-bit (extended) size, which lofty 0.22 mis-skips by eight
//! bytes and then reports as "No \"moov\" atom found". The stem muxer always
//! writes such an `mdat`, which is why native Stem files are unreadable to lofty
//! and why they are refused by name above. Encoders that emit 32-bit `mdat`
//! sizes (iTunes, ffmpeg, Rekordbox) are unaffected.

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{ItemKey, Tag, TagType};

use crate::paths::is_stem_path;

/// One writable tag field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagField {
    /// Track title.
    Title,
    /// Primary artist.
    Artist,
    /// Album name.
    Album,
    /// Genre.
    Genre,
    /// Beats per minute.
    Bpm,
    /// Initial key, in Mixar's canonical musical notation (`"Am"`, `"F#m"`).
    Key,
    /// Star rating 1–5 (`POPM`, `RATING`, `rate`).
    Rating,
    /// Free-form comment.
    Comment,
    /// Release year.
    Year,
    /// Track number within its disc.
    TrackNumber,
    /// Disc number within its release.
    DiscNumber,
    /// Colour label name (`"Pink"` … `"Purple"`).
    Color,
}

impl TagField {
    /// Every writable field, in report order.
    pub const ALL: [Self; 12] = [
        Self::Title,
        Self::Artist,
        Self::Album,
        Self::Genre,
        Self::Bpm,
        Self::Key,
        Self::Rating,
        Self::Comment,
        Self::Year,
        Self::TrackNumber,
        Self::DiscNumber,
        Self::Color,
    ];

    /// Short name for logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Genre => "genre",
            Self::Bpm => "bpm",
            Self::Key => "key",
            Self::Rating => "rating",
            Self::Comment => "comment",
            Self::Year => "year",
            Self::TrackNumber => "track_number",
            Self::DiscNumber => "disc_number",
            Self::Color => "color",
        }
    }
}

/// Track information to mirror into a file's tags.
///
/// A patch, not a full row: `None` leaves that field's tag alone, so a caller
/// can pass exactly the values it owns. (`Some("")` / a non-finite BPM are
/// reported as [`FieldWrite::Absent`] and written nowhere.) Removing a tag is
/// not expressible yet — add that when an editor needs to clear a field.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackTags {
    /// Track title.
    pub title: Option<String>,
    /// Primary artist.
    pub artist: Option<String>,
    /// Album name.
    pub album: Option<String>,
    /// Genre.
    pub genre: Option<String>,
    /// Beats per minute.
    pub bpm: Option<f64>,
    /// Musical key in Mixar's canonical notation (`"Am"`, `"F#m"`) — never Camelot.
    pub key: Option<String>,
    /// Star rating 1–5. `Some(0)` is treated as "unrated" and written nowhere.
    pub rating: Option<u8>,
    /// Free-form comment.
    pub comment: Option<String>,
    /// Release year.
    pub year: Option<u16>,
    /// Track number within its disc. `Some(0)` is treated as absent.
    pub track_number: Option<u16>,
    /// Disc number within its release. `Some(0)` is treated as absent.
    pub disc_number: Option<u16>,
    /// Colour label name (`"Pink"` … `"Purple"`), written to `COLOR`.
    pub color: Option<String>,
}

impl TrackTags {
    /// Whether this patch would touch any field.
    pub fn is_empty(&self) -> bool {
        self.requests().next().is_none()
    }

    /// Requested fields, in [`TagField::ALL`] order. Fields left as `None` are
    /// omitted; unusable values are included and rejected when applied.
    fn requests(&self) -> impl Iterator<Item = Request<'_>> {
        [
            self.title.as_deref().map(Request::Title),
            self.artist.as_deref().map(Request::Artist),
            self.album.as_deref().map(Request::Album),
            self.genre.as_deref().map(Request::Genre),
            self.bpm.map(Request::Bpm),
            self.key.as_deref().map(Request::Key),
            self.rating.map(Request::Rating),
            self.comment.as_deref().map(Request::Comment),
            self.year.map(Request::Year),
            self.track_number.map(Request::TrackNumber),
            self.disc_number.map(Request::DiscNumber),
            self.color.as_deref().map(Request::Color),
        ]
        .into_iter()
        .flatten()
    }
}

/// A requested field with its value. Carrying the field in the variant keeps
/// [`apply`] a single exhaustive match.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Request<'a> {
    Title(&'a str),
    Artist(&'a str),
    Album(&'a str),
    Genre(&'a str),
    Bpm(f64),
    Key(&'a str),
    Rating(u8),
    Comment(&'a str),
    Year(u16),
    TrackNumber(u16),
    DiscNumber(u16),
    Color(&'a str),
}

impl Request<'_> {
    fn field(&self) -> TagField {
        match self {
            Self::Title(_) => TagField::Title,
            Self::Artist(_) => TagField::Artist,
            Self::Album(_) => TagField::Album,
            Self::Genre(_) => TagField::Genre,
            Self::Bpm(_) => TagField::Bpm,
            Self::Key(_) => TagField::Key,
            Self::Rating(_) => TagField::Rating,
            Self::Comment(_) => TagField::Comment,
            Self::Year(_) => TagField::Year,
            Self::TrackNumber(_) => TagField::TrackNumber,
            Self::DiscNumber(_) => TagField::DiscNumber,
            Self::Color(_) => TagField::Color,
        }
    }
}

/// Tag container a file's information is written into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagContainer {
    /// ID3v2 (MP3, WAV, AIFF, AAC).
    Id3v2,
    /// Vorbis comments (FLAC, Ogg Vorbis, Opus, Speex).
    VorbisComments,
    /// MP4/M4A `ilst` atoms.
    Mp4Ilst,
    /// APE tags (Monkey's Audio, WavPack, Musepack).
    Ape,
}

impl TagContainer {
    /// Container for a lofty tag type, or `None` for types Mixar never writes.
    pub fn of(tag_type: TagType) -> Option<Self> {
        match tag_type {
            TagType::Id3v2 => Some(Self::Id3v2),
            TagType::VorbisComments => Some(Self::VorbisComments),
            TagType::Mp4Ilst => Some(Self::Mp4Ilst),
            TagType::Ape => Some(Self::Ape),
            TagType::Id3v1 | TagType::RiffInfo | TagType::AiffText => None,
            // `TagType` is `#[non_exhaustive]`: unknown containers are not writable.
            _ => None,
        }
    }

    /// Short name for logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Id3v2 => "id3v2",
            Self::VorbisComments => "vorbis_comments",
            Self::Mp4Ilst => "mp4_ilst",
            Self::Ape => "ape",
        }
    }
}

/// Outcome for one written field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldWrite {
    /// Stored verbatim.
    Written,
    /// Stored rounded into an integer-only frame (ID3v2 `TBPM`).
    Rounded {
        /// Value actually stored.
        stored: i64,
    },
    /// The file already carried the wanted value; nothing was rewritten.
    Unchanged,
    /// The container has no field for this datum (APE has no BPM/key), or the
    /// file type has no writable tag at all.
    Unsupported,
    /// The file shape is not safe to rewrite (native Stem container; FLAC with
    /// no metadata block after STREAMINFO — see the module docs).
    Refused,
    /// The requested value was empty or unusable (blank text, non-finite or
    /// non-positive BPM); nothing was written.
    Absent,
}

/// What a [`write_track_tags`] call did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagWriteReport {
    /// Container targeted, or `None` when the file type has no writable tag.
    pub container: Option<TagContainer>,
    /// Outcome per requested field, in [`TagField::ALL`] order. Fields the caller
    /// left as `None` are absent from the list.
    pub fields: Vec<(TagField, FieldWrite)>,
}

impl TagWriteReport {
    /// Whether the file on disk was rewritten.
    pub fn wrote(&self) -> bool {
        self.fields
            .iter()
            .any(|(_, outcome)| matches!(outcome, FieldWrite::Written | FieldWrite::Rounded { .. }))
    }

    /// Outcome for one field, when it was requested.
    pub fn outcome(&self, field: TagField) -> Option<FieldWrite> {
        self.fields
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map(|(_, outcome)| *outcome)
    }

    /// Report where every requested field has `outcome`.
    fn all(container: Option<TagContainer>, fields: &[TagField], outcome: FieldWrite) -> Self {
        Self {
            container,
            fields: fields.iter().map(|field| (*field, outcome)).collect(),
        }
    }
}

/// Write track information into `path`'s tags.
///
/// Reads the file, updates only the fields set in [`TrackTags`] that differ from
/// what is already stored, and rewrites the file only when something changed.
/// Errors are I/O or parse failures; per-field outcomes (unsupported container,
/// rounding, refusal) come back in the report instead.
pub fn write_track_tags(path: &Path, tags: &TrackTags) -> anyhow::Result<TagWriteReport> {
    let requests: Vec<Request<'_>> = tags.requests().collect();
    let fields: Vec<TagField> = requests.iter().map(Request::field).collect();
    if requests.is_empty() {
        return Ok(TagWriteReport {
            container: None,
            fields: Vec::new(),
        });
    }

    // `.stem.mp4` files are multi-track MP4s carrying the NI `stem` manifest.
    if is_stem_path(path) {
        return Ok(TagWriteReport::all(None, &fields, FieldWrite::Refused));
    }

    let mut tagged = Probe::open(path)
        .map_err(|e| anyhow::anyhow!("open {}: {e}", path.display()))?
        .read()
        .map_err(|e| anyhow::anyhow!("read tags {}: {e}", path.display()))?;

    let container = TagContainer::of(tagged.file_type().primary_tag_type());
    let Some(container) = container else {
        return Ok(TagWriteReport::all(None, &fields, FieldWrite::Unsupported));
    };

    // A FLAC with nothing but STREAMINFO cannot be written safely (see module docs).
    if tagged.file_type() == FileType::Flac && tagged.tags().is_empty() {
        return Ok(TagWriteReport::all(
            Some(container),
            &fields,
            FieldWrite::Refused,
        ));
    }

    let tag_type = tagged.file_type().primary_tag_type();
    if !tagged.supports_tag_type(tag_type) {
        return Ok(TagWriteReport::all(
            Some(container),
            &fields,
            FieldWrite::Unsupported,
        ));
    }
    if tagged.tag(tag_type).is_none() {
        tagged.insert_tag(Tag::new(tag_type));
    }
    let Some(tag) = tagged.tag_mut(tag_type) else {
        return Ok(TagWriteReport::all(
            Some(container),
            &fields,
            FieldWrite::Unsupported,
        ));
    };

    let report = TagWriteReport {
        container: Some(container),
        fields: requests
            .into_iter()
            .map(|request| {
                let field = request.field();
                (field, apply(tag, request))
            })
            .collect(),
    };
    if report.wrote() {
        tagged
            .save_to_path(path, WriteOptions::default())
            .map_err(|e| anyhow::anyhow!("write tags {}: {e}", path.display()))?;
    }
    Ok(report)
}

/// Set one field on `tag`. Pure (no I/O) so the per-container matrix is directly
/// testable.
fn apply(tag: &mut Tag, request: Request<'_>) -> FieldWrite {
    match request {
        Request::Title(value) => apply_text(tag, ItemKey::TrackTitle, value),
        Request::Artist(value) => apply_text(tag, ItemKey::TrackArtist, value),
        Request::Album(value) => apply_text(tag, ItemKey::AlbumTitle, value),
        Request::Genre(value) => apply_text(tag, ItemKey::Genre, value),
        Request::Bpm(bpm) => apply_bpm(tag, bpm),
        Request::Key(value) => apply_text(tag, ItemKey::InitialKey, value),
        Request::Rating(rating) => apply_rating(tag, rating),
        Request::Comment(value) => apply_text(tag, ItemKey::Comment, value),
        Request::Year(year) => apply_year(tag, year),
        Request::TrackNumber(number) => apply_number(tag, ItemKey::TrackNumber, number),
        Request::DiscNumber(number) => apply_number(tag, ItemKey::DiscNumber, number),
        Request::Color(value) => apply_text(tag, ItemKey::Color, value),
    }
}

/// Set a track/disc number, skipping zero (the format's own "unset").
fn apply_number(tag: &mut Tag, key: ItemKey, number: u16) -> FieldWrite {
    if number == 0 {
        return FieldWrite::Absent;
    }
    // "3/12" already carries the number in its leading digits; rewriting it as
    // "3" would silently drop the track total.
    if let Some(existing) = tag.get_string(&key) {
        if crate::leading_u16(existing) == Some(number) {
            return FieldWrite::Unchanged;
        }
    }
    apply_text(tag, key, &number.to_string())
}

/// Set the release year in the container's date field.
///
/// ID3v2 has no year frame (lofty maps `YEAR` nowhere) and MP4's field is the
/// `©day` date atom, so those two take `RecordingDate`; Vorbis comments and APE
/// have a dedicated `YEAR`/`Year` field.
fn apply_year(tag: &mut Tag, year: u16) -> FieldWrite {
    if year == 0 {
        return FieldWrite::Absent;
    }
    let key = match tag.tag_type() {
        TagType::Id3v2 | TagType::Mp4Ilst => ItemKey::RecordingDate,
        _ => ItemKey::Year,
    };
    let text = year.to_string();
    // A full date (`2024-05-01`) already carries this year; rewriting it to
    // `2024` would lose the month/day and break the idempotency contract.
    // Vorbis/APE may carry the date under DATE instead of YEAR, and the reader
    // accepts either.
    for existing in [
        tag.get_string(&key),
        tag.get_string(&ItemKey::RecordingDate),
        tag.get_string(&ItemKey::ReleaseDate),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(rest) = existing.trim().strip_prefix(&text) {
            // "2024-05-01" preserves the year, but "1999" must not satisfy 199.
            if !rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return FieldWrite::Unchanged;
            }
        }
    }
    apply_text(tag, key, &text)
}

/// Set the star rating in the container's rating field.
///
/// Vorbis comments follow the Quod Libet / MusicBrainz Picard convention:
/// `RATING` is a decimal between `0.0` and `1.0` (`0.8` = four stars). MP4
/// `rate` is written as the star count itself, which is what lofty's generic
/// `Popularimeter` mapping has always emitted. ID3v2 is reported
/// [`FieldWrite::Unsupported`]: its rating is the binary `POPM` frame, which
/// lofty 0.22 keeps in the tag's format-specific remainder instead of exposing
/// through [`Tag`]. Writing one would be invisible to the reader (so a second
/// write could never be idempotent), and Mixar must not churn an mp3's frames
/// on every analysis just to leave the file functionally unchanged. APE's key
/// map has no rating entry at all.
fn apply_rating(tag: &mut Tag, rating: u8) -> FieldWrite {
    if rating == 0 || rating > 5 {
        return FieldWrite::Absent;
    }
    let text = match tag.tag_type() {
        TagType::VorbisComments => format!("{:.1}", f64::from(rating) / 5.0),
        TagType::Mp4Ilst => rating.to_string(),
        _ => return FieldWrite::Unsupported,
    };
    if tag.get_string(&ItemKey::Popularimeter) == Some(text.as_str()) {
        return FieldWrite::Unchanged;
    }
    if tag.insert_text(ItemKey::Popularimeter, text) {
        FieldWrite::Written
    } else {
        FieldWrite::Unsupported
    }
}

/// Set a plain text field, skipping blank values.
fn apply_text(tag: &mut Tag, key: ItemKey, value: &str) -> FieldWrite {
    let value = value.trim();
    if value.is_empty() {
        return FieldWrite::Absent;
    }
    if tag.get_string(&key) == Some(value) {
        return FieldWrite::Unchanged;
    }
    if tag.insert_text(key, value.to_string()) {
        FieldWrite::Written
    } else {
        FieldWrite::Unsupported
    }
}

/// Set the BPM field for `tag`'s container, skipping unusable values.
fn apply_bpm(tag: &mut Tag, bpm: f64) -> FieldWrite {
    if !bpm.is_finite() || bpm <= 0.0 {
        return FieldWrite::Absent;
    }

    match tag.tag_type() {
        // `TBPM` is an integer frame; DJ tools (Mixxx, Mixed In Key) round.
        TagType::Id3v2 => {
            let stored = bpm.round() as i64;
            if existing_bpm_integer(tag) == Some(stored) {
                return FieldWrite::Unchanged;
            }
            if !tag.insert_text(ItemKey::IntegerBpm, stored.to_string()) {
                return FieldWrite::Unsupported;
            }
            if (bpm - stored as f64).abs() > f64::EPSILON {
                FieldWrite::Rounded { stored }
            } else {
                FieldWrite::Written
            }
        }
        // Vorbis `BPM` and the MP4 freeform `BPM` atom are text.
        TagType::VorbisComments | TagType::Mp4Ilst => {
            let text = format_bpm(bpm);
            if tag.get_string(&ItemKey::Bpm) == Some(text.as_str()) {
                return FieldWrite::Unchanged;
            }
            if tag.insert_text(ItemKey::Bpm, text) {
                FieldWrite::Written
            } else {
                FieldWrite::Unsupported
            }
        }
        // APE / ID3v1 / RIFF INFO / AIFF text carry no BPM.
        _ => FieldWrite::Unsupported,
    }
}

/// Shortest round-tripping text form: `128.0` → `"128"`, `127.98` → `"127.98"`.
fn format_bpm(bpm: f64) -> String {
    bpm.to_string()
}

/// Existing integer BPM in the tag, from either the integer frame or a text one.
fn existing_bpm_integer(tag: &Tag) -> Option<i64> {
    tag.get_string(&ItemKey::IntegerBpm)
        .or_else(|| tag.get_string(&ItemKey::Bpm))
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .map(|bpm| bpm.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read_file_tags;
    use std::io::Write as _;
    use std::path::PathBuf;

    /// Every field, so a test can assert the whole matrix at once.
    fn full_patch() -> TrackTags {
        TrackTags {
            title: Some("Night Drive".into()),
            artist: Some("Someone".into()),
            album: Some("Some Album".into()),
            genre: Some("House".into()),
            bpm: Some(128.0),
            key: Some("Am".into()),
            rating: Some(4),
            comment: Some("Peak-time".into()),
            year: Some(2024),
            track_number: Some(3),
            disc_number: Some(1),
            color: Some("Pink".into()),
        }
    }

    fn write_minimal_wav(path: &Path) {
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

    /// Minimal standalone FLAC: `fLaC`, STREAMINFO, optional empty VORBIS_COMMENT
    /// block, then frames encoded by the same `flacenc` the stem muxer uses.
    ///
    /// `with_comment` false produces the STREAMINFO-only shape that real encoders
    /// never write but `metaflac --remove-all` does.
    fn write_minimal_flac(path: &Path, with_comment: bool) {
        use flacenc::bitsink::ByteSink;
        use flacenc::component::BitRepr;
        use flacenc::error::Verify;
        use flacenc::source::MemSource;

        let rate = 8_000u32;
        let pcm: Vec<i32> = (0..rate as usize)
            .map(|index| ((index as f32 * 0.05).sin() * 1000.0) as i32)
            .collect();
        let config = flacenc::config::Encoder::default().into_verified().unwrap();
        let source = MemSource::from_samples(&pcm, 1, 16, rate as usize);
        let stream =
            flacenc::encode_with_fixed_block_size(&config, source, config.block_size).unwrap();

        let mut info = ByteSink::new();
        stream.stream_info().write(&mut info).unwrap();
        let info = info.as_slice();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"fLaC");
        bytes.push(u8::from(!with_comment) << 7);
        bytes.extend_from_slice(&u32::try_from(info.len()).unwrap().to_be_bytes()[1..]);
        bytes.extend_from_slice(info);
        if with_comment {
            // VORBIS_COMMENT, flagged last, with a vendor string and no items.
            let vendor = b"reference libFLAC";
            let len = 4 + vendor.len() + 4;
            bytes.push(0x80 | 4);
            bytes.extend_from_slice(&u32::try_from(len).unwrap().to_be_bytes()[1..]);
            bytes.extend_from_slice(&u32::try_from(vendor.len()).unwrap().to_le_bytes());
            bytes.extend_from_slice(vendor);
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }
        for index in 0..stream.frame_count() {
            let frame = stream.frame(index).unwrap();
            let mut sink = ByteSink::new();
            frame.write(&mut sink).unwrap();
            bytes.extend_from_slice(sink.as_slice());
        }
        std::fs::write(path, bytes).unwrap();
    }

    fn temp_path(name: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        (dir, path)
    }

    #[test]
    fn id3v2_writes_the_whole_track_information_and_reads_back() {
        let (_dir, path) = temp_path("track.wav");
        write_minimal_wav(&path);

        let report = write_track_tags(&path, &full_patch()).unwrap();

        assert_eq!(report.container, Some(TagContainer::Id3v2));
        assert!(report.wrote());
        for field in TagField::ALL {
            // ID3v2 stores the rating in a binary POPM frame that lofty 0.22
            // keeps out of the generic tag, so it is documented unsupported.
            let expected = if field == TagField::Rating {
                FieldWrite::Unsupported
            } else {
                FieldWrite::Written
            };
            assert_eq!(report.outcome(field), Some(expected), "{field:?}");
        }

        let tags = read_file_tags(&path).unwrap();
        assert_eq!(tags.title.as_deref(), Some("Night Drive"));
        assert_eq!(tags.artist.as_deref(), Some("Someone"));
        assert_eq!(tags.album.as_deref(), Some("Some Album"));
        assert_eq!(tags.genre.as_deref(), Some("House"));
        assert_eq!(tags.bpm, Some(128.0));
        assert_eq!(tags.key.as_deref(), Some("Am"));
        assert_eq!(tags.rating, None, "ID3 POPM is not exposed by lofty 0.22");
        assert_eq!(tags.comment.as_deref(), Some("Peak-time"));
        assert_eq!(tags.year, Some(2024));
        assert_eq!(tags.track_number, Some(3));
        assert_eq!(tags.disc_number, Some(1));
        assert_eq!(tags.color.as_deref(), Some("Pink"));
    }

    #[test]
    fn year_write_keeps_a_full_date() {
        let mut tag = Tag::new(TagType::VorbisComments);
        assert!(tag.insert_text(ItemKey::Year, "2024-05-01".to_string()));
        assert_eq!(apply_year(&mut tag, 2024), FieldWrite::Unchanged);
        assert_eq!(tag.get_string(&ItemKey::Year), Some("2024-05-01"));

        assert_eq!(apply_year(&mut tag, 2025), FieldWrite::Written);
        assert_eq!(tag.get_string(&ItemKey::Year), Some("2025"));

        // A longer stored year must not satisfy a shorter requested prefix.
        assert!(tag.insert_text(ItemKey::Year, "1999".to_string()));
        assert_eq!(apply_year(&mut tag, 199), FieldWrite::Written);
        assert_eq!(tag.get_string(&ItemKey::Year), Some("199"));
    }

    #[test]
    fn year_write_keeps_a_matching_date_tag() {
        let mut tag = Tag::new(TagType::VorbisComments);
        assert!(tag.insert_text(ItemKey::RecordingDate, "2024-05-01".to_string()));
        assert_eq!(apply_year(&mut tag, 2024), FieldWrite::Unchanged);
        assert_eq!(tag.get_string(&ItemKey::Year), None);
    }

    #[test]
    fn number_write_keeps_a_slash_total() {
        let mut tag = Tag::new(TagType::VorbisComments);
        assert!(tag.insert_text(ItemKey::TrackNumber, "3/12".to_string()));
        assert_eq!(
            apply_number(&mut tag, ItemKey::TrackNumber, 3),
            FieldWrite::Unchanged
        );
        assert_eq!(tag.get_string(&ItemKey::TrackNumber), Some("3/12"));

        assert_eq!(
            apply_number(&mut tag, ItemKey::TrackNumber, 4),
            FieldWrite::Written
        );
        assert_eq!(tag.get_string(&ItemKey::TrackNumber), Some("4"));
    }

    #[test]
    fn rating_round_trips_in_text_containers_and_is_unsupported_in_id3() {
        let mut tag = Tag::new(TagType::VorbisComments);
        assert_eq!(apply_rating(&mut tag, 4), FieldWrite::Written);
        assert_eq!(tag.get_string(&ItemKey::Popularimeter), Some("0.8"));
        assert_eq!(apply_rating(&mut tag, 4), FieldWrite::Unchanged);
        assert_eq!(apply_rating(&mut tag, 0), FieldWrite::Absent);
        assert_eq!(apply_rating(&mut tag, 6), FieldWrite::Absent);

        let mut id3 = Tag::new(TagType::Id3v2);
        assert_eq!(apply_rating(&mut id3, 4), FieldWrite::Unsupported);
        assert_eq!(id3.items().count(), 0, "no frame written");
    }

    #[test]
    fn id3v2_rounds_fractional_bpm_and_is_idempotent() {
        let (_dir, path) = temp_path("track.wav");
        write_minimal_wav(&path);

        let report = write_track_tags(
            &path,
            &TrackTags {
                bpm: Some(127.98),
                ..TrackTags::default()
            },
        )
        .unwrap();
        assert_eq!(
            report.outcome(TagField::Bpm),
            Some(FieldWrite::Rounded { stored: 128 })
        );
        assert_eq!(read_file_tags(&path).unwrap().bpm, Some(128.0));

        // A second pass with the same value must not rewrite the file.
        let before = std::fs::read(&path).unwrap();
        let report = write_track_tags(
            &path,
            &TrackTags {
                bpm: Some(127.98),
                ..TrackTags::default()
            },
        )
        .unwrap();
        assert_eq!(report.outcome(TagField::Bpm), Some(FieldWrite::Unchanged));
        assert!(!report.wrote());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn vorbis_comments_keep_fractional_bpm_and_are_idempotent() {
        let (_dir, path) = temp_path("track.flac");
        write_minimal_flac(&path, true);

        let patch = TrackTags {
            bpm: Some(127.98),
            key: Some("F#m".into()),
            title: Some("Night Drive".into()),
            rating: Some(4),
            comment: Some("Peak time".into()),
            year: Some(2024),
            track_number: Some(3),
            disc_number: Some(2),
            color: Some("Aqua".into()),
            ..TrackTags::default()
        };
        let report = write_track_tags(&path, &patch).unwrap();

        assert_eq!(report.container, Some(TagContainer::VorbisComments));
        assert_eq!(report.outcome(TagField::Bpm), Some(FieldWrite::Written));
        assert_eq!(report.outcome(TagField::Key), Some(FieldWrite::Written));
        assert_eq!(report.outcome(TagField::Title), Some(FieldWrite::Written));

        let tags = read_file_tags(&path).unwrap();
        assert_eq!(tags.bpm, Some(127.98));
        assert_eq!(tags.key.as_deref(), Some("F#m"));
        assert_eq!(tags.title.as_deref(), Some("Night Drive"));
        assert_eq!(tags.rating, Some(4));
        assert_eq!(tags.comment.as_deref(), Some("Peak time"));
        assert_eq!(tags.year, Some(2024));
        assert_eq!(tags.track_number, Some(3));
        assert_eq!(tags.disc_number, Some(2));
        assert_eq!(tags.color.as_deref(), Some("Aqua"));

        let before = std::fs::read(&path).unwrap();
        let report = write_track_tags(&path, &patch).unwrap();
        assert!(!report.wrote());
        for field in [
            TagField::Bpm,
            TagField::Key,
            TagField::Title,
            TagField::Rating,
            TagField::Comment,
            TagField::Year,
            TagField::TrackNumber,
            TagField::DiscNumber,
            TagField::Color,
        ] {
            assert_eq!(
                report.outcome(field),
                Some(FieldWrite::Unchanged),
                "{field:?}"
            );
        }
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn flac_with_only_streaminfo_is_refused_untouched() {
        let (_dir, path) = temp_path("stripped.flac");
        write_minimal_flac(&path, false);
        let before = std::fs::read(&path).unwrap();

        let report = write_track_tags(&path, &full_patch()).unwrap();

        assert_eq!(report.container, Some(TagContainer::VorbisComments));
        assert!(!report.wrote());
        for field in TagField::ALL {
            assert_eq!(
                report.outcome(field),
                Some(FieldWrite::Refused),
                "{field:?}"
            );
        }
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn mp4_ilst_maps_track_information_to_its_atoms() {
        let mut tag = Tag::new(TagType::Mp4Ilst);
        for (key, value) in [
            (ItemKey::TrackTitle, "Night Drive"),
            (ItemKey::TrackArtist, "Someone"),
            (ItemKey::AlbumTitle, "Some Album"),
            (ItemKey::Genre, "House"),
            (ItemKey::InitialKey, "Am"),
            (ItemKey::Comment, "Peak time"),
            (ItemKey::Color, "Pink"),
        ] {
            assert_eq!(apply_text(&mut tag, key, value), FieldWrite::Written);
        }
        assert_eq!(apply_bpm(&mut tag, 127.98), FieldWrite::Written);
        assert_eq!(apply_rating(&mut tag, 4), FieldWrite::Written);
        assert_eq!(apply_year(&mut tag, 2024), FieldWrite::Written);
        assert_eq!(
            apply_number(&mut tag, ItemKey::TrackNumber, 3),
            FieldWrite::Written
        );
        assert_eq!(
            apply_number(&mut tag, ItemKey::DiscNumber, 1),
            FieldWrite::Written
        );

        // Round-trip through the MP4 atom mapping proves the fields land on the
        // atoms TagLib/Mixxx read — including the freeform BPM and initialkey
        // names rather than the integer `tmpo` atom.
        let ilst: lofty::mp4::Ilst = tag.into();
        let back: Tag = ilst.into();
        assert_eq!(back.get_string(&ItemKey::TrackTitle), Some("Night Drive"));
        assert_eq!(back.get_string(&ItemKey::TrackArtist), Some("Someone"));
        assert_eq!(back.get_string(&ItemKey::AlbumTitle), Some("Some Album"));
        assert_eq!(back.get_string(&ItemKey::Genre), Some("House"));
        assert_eq!(back.get_string(&ItemKey::Bpm), Some("127.98"));
        assert_eq!(back.get_string(&ItemKey::InitialKey), Some("Am"));
        assert_eq!(back.get_string(&ItemKey::IntegerBpm), None);
        assert_eq!(back.get_string(&ItemKey::Popularimeter), Some("4"));
        assert_eq!(back.get_string(&ItemKey::Comment), Some("Peak time"));
        assert_eq!(back.get_string(&ItemKey::RecordingDate), Some("2024"));
        assert_eq!(back.get_string(&ItemKey::TrackNumber), Some("3"));
        assert_eq!(back.get_string(&ItemKey::DiscNumber), Some("1"));
        assert_eq!(back.get_string(&ItemKey::Color), Some("Pink"));
    }

    #[test]
    fn ape_writes_text_fields_but_has_no_bpm_or_key() {
        let mut tag = Tag::new(TagType::Ape);
        assert_eq!(
            apply_text(&mut tag, ItemKey::TrackTitle, "Night Drive"),
            FieldWrite::Written
        );
        assert_eq!(
            apply_text(&mut tag, ItemKey::Genre, "House"),
            FieldWrite::Written
        );
        assert_eq!(apply_bpm(&mut tag, 128.0), FieldWrite::Unsupported);
        assert_eq!(
            apply_text(&mut tag, ItemKey::InitialKey, "Am"),
            FieldWrite::Unsupported
        );
        for (key, value) in [(ItemKey::Comment, "Peak time"), (ItemKey::Color, "Pink")] {
            assert_eq!(apply_text(&mut tag, key, value), FieldWrite::Written);
        }
        // APE's key map has no rating entry.
        assert_eq!(apply_rating(&mut tag, 4), FieldWrite::Unsupported);
        assert_eq!(apply_year(&mut tag, 2024), FieldWrite::Written);
        assert_eq!(
            apply_number(&mut tag, ItemKey::TrackNumber, 3),
            FieldWrite::Written
        );
        assert_eq!(tag.items().count(), 6);
    }

    #[test]
    fn unrequested_fields_are_not_reported() {
        let (_dir, path) = temp_path("track.wav");
        write_minimal_wav(&path);

        let report = write_track_tags(
            &path,
            &TrackTags {
                genre: Some("House".into()),
                ..TrackTags::default()
            },
        )
        .unwrap();
        assert_eq!(report.fields, vec![(TagField::Genre, FieldWrite::Written)]);
        assert_eq!(report.outcome(TagField::Title), None);
    }

    #[test]
    fn ignores_unusable_values() {
        let mut tag = Tag::new(TagType::VorbisComments);
        assert_eq!(
            apply_text(&mut tag, ItemKey::TrackTitle, "   "),
            FieldWrite::Absent
        );
        assert_eq!(apply_bpm(&mut tag, f64::NAN), FieldWrite::Absent);
        assert_eq!(apply_bpm(&mut tag, -4.0), FieldWrite::Absent);
        assert_eq!(tag.items().count(), 0);

        // A blank request is reported as Absent rather than silently dropped.
        let (_dir, path) = temp_path("track.wav");
        write_minimal_wav(&path);
        let report = write_track_tags(
            &path,
            &TrackTags {
                title: Some(String::new()),
                bpm: Some(f64::NAN),
                ..TrackTags::default()
            },
        )
        .unwrap();
        assert_eq!(report.outcome(TagField::Title), Some(FieldWrite::Absent));
        assert_eq!(report.outcome(TagField::Bpm), Some(FieldWrite::Absent));
        assert!(!report.wrote());
    }

    #[test]
    fn stem_containers_are_never_rewritten() {
        let report = write_track_tags(Path::new("/music/Track.stem.mp4"), &full_patch()).unwrap();
        assert_eq!(report.container, None);
        assert!(!report.wrote());
        for field in TagField::ALL {
            assert_eq!(
                report.outcome(field),
                Some(FieldWrite::Refused),
                "{field:?}"
            );
        }
    }

    #[test]
    fn default_tags_report_nothing_to_write() {
        let (_dir, path) = temp_path("track.wav");
        write_minimal_wav(&path);
        let before = std::fs::read(&path).unwrap();

        let patch = TrackTags::default();
        assert!(patch.is_empty());
        let report = write_track_tags(&path, &patch).unwrap();
        assert!(report.fields.is_empty());
        assert!(!report.wrote());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn non_audio_file_errors() {
        let (_dir, path) = temp_path("notes.txt");
        std::fs::write(&path, b"not audio").unwrap();
        let err = write_track_tags(
            &path,
            &TrackTags {
                bpm: Some(128.0),
                ..TrackTags::default()
            },
        )
        .unwrap_err();
        let message = err.to_string();
        assert!(message.contains("open") || message.contains("read tags"));
    }

    #[test]
    fn container_mapping_matches_written_types() {
        assert_eq!(TagContainer::of(TagType::Id3v2), Some(TagContainer::Id3v2));
        assert_eq!(
            TagContainer::of(TagType::VorbisComments),
            Some(TagContainer::VorbisComments)
        );
        assert_eq!(
            TagContainer::of(TagType::Mp4Ilst),
            Some(TagContainer::Mp4Ilst)
        );
        assert_eq!(TagContainer::of(TagType::Ape), Some(TagContainer::Ape));
        assert_eq!(TagContainer::of(TagType::Id3v1), None);
        assert_eq!(TagContainer::of(TagType::RiffInfo), None);
        assert_eq!(TagContainer::of(TagType::AiffText), None);
    }
}
