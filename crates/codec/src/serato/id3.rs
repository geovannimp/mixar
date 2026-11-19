//! Bounded ID3v2.3/v2.4 tag scanning for vendor GEOB frames.
//!
//! Only the frames this reader consumes are decoded: the five GEOB
//! descriptions Serato and Mixed In Key write. Everything else in the tag, and
//! every audio/unknown container chunk, is skipped by seeking so a song is never
//! read into memory.
//!
//! Containers are detected by signature — `ID3` for MP3 (and tag-only test
//! fixtures), `RIFF`/`WAVE` for WAV and `FORM`/`AIFF`/`AIFC` for AIFF. Anything
//! else, a missing tag or an unsupported ID3 version yields empty data rather
//! than an error; only file I/O failures are reported as errors.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::{SeratoContainer, SeratoFrameKind};

/// Largest ID3 tag read into memory. Larger tags mark the scan incomplete.
const MAX_TAG_BYTES: u64 = 16 * 1024 * 1024;
/// Largest number of ID3 frames inspected in one tag.
const MAX_FRAMES: usize = 65_536;
/// Largest number of container chunks walked before giving up.
///
/// A crafted file can chain millions of zero-length chunks; the cap keeps the
/// walk proportional to the tag bound rather than to the file size.
const MAX_CONTAINER_CHUNKS: usize = 65_536;
/// Length of an ID3v2 tag header.
const TAG_HEADER_LEN: u64 = 10;

/// One recognized GEOB frame, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawGeoBFrame {
    pub(crate) kind: SeratoFrameKind,
    /// Unmodified logical GEOB object data (after ID3 decoding).
    pub(crate) payload: Vec<u8>,
}

/// Result of scanning one file's tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagScan {
    /// Container a tag can live in, or `None` for a container this reader does
    /// not understand (FLAC, MP4, Ogg, …). Empty frames in such a file are not
    /// evidence that the file carries no vendor metadata.
    pub(crate) container: Option<SeratoContainer>,
    pub(crate) frames: Vec<RawGeoBFrame>,
    /// `false` when a bound was exceeded, a value was structurally invalid or a
    /// frame had to be skipped. Callers must not trust absence of a frame then.
    pub(crate) scan_complete: bool,
}

impl TagScan {
    fn no_tag() -> Self {
        Self {
            container: None,
            frames: Vec::new(),
            scan_complete: true,
        }
    }

    fn incomplete() -> Self {
        Self {
            container: None,
            frames: Vec::new(),
            scan_complete: false,
        }
    }

    /// Record the container the scan actually walked.
    fn in_container(mut self, container: SeratoContainer) -> Self {
        self.container = Some(container);
        self
    }
}

/// Major ID3v2 version (3 or 4) of the tag stored in `path`.
///
/// Only used by write-back, which must keep the version a file already used
/// instead of letting a tag library silently upgrade it (`Id3v2Tag` upgrades
/// v2.2/v2.3 to v2.4 when reading). `None` means "no tag we can see".
pub(crate) fn id3v2_major(path: &Path) -> anyhow::Result<Option<u8>> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let offset = {
        let mut head = [0u8; 12];
        let head_len = read_some(&mut file, &mut head)?;
        let head = &head[..head_len];
        if head.len() >= 3 && &head[..3] == b"ID3" {
            0
        } else {
            let (order, form) =
                if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WAVE" {
                    (ByteOrder::Little, 4)
                } else if head.len() >= 12
                    && &head[..4] == b"FORM"
                    && (&head[8..12] == b"AIFF" || &head[8..12] == b"AIFC")
                {
                    (ByteOrder::Big, 4)
                } else {
                    return Ok(None);
                };
            let declared = order.read_u32(&head[form..form + 4]) as u64;
            let Some(form_end) = declared.checked_add(8) else {
                return Ok(None);
            };
            if form_end > file_len {
                return Ok(None);
            }
            match find_metadata_chunk(&mut file, 12, form_end, order)? {
                ChunkSearch::Found { offset, .. } => offset,
                ChunkSearch::Absent | ChunkSearch::Malformed => return Ok(None),
            }
        }
    };
    if offset + TAG_HEADER_LEN > file_len {
        return Ok(None);
    }
    let mut header = [0u8; TAG_HEADER_LEN as usize];
    read_at(&mut file, offset, &mut header)?;
    if &header[..3] != b"ID3" {
        return Ok(None);
    }
    Ok(Some(header[3]))
}

/// Scan `path` for Serato/Mixed In Key GEOB frames.
pub(crate) fn scan_path(path: &Path) -> anyhow::Result<TagScan> {
    let mut file = File::open(path)?;
    scan_reader(&mut file)
}

/// Scan an already-open file, read from its current position (usually the
/// start), so one handle can feed several readers during discovery.
pub(crate) fn scan_reader(file: &mut File) -> anyhow::Result<TagScan> {
    let file_len = file.metadata()?.len();
    let mut head = [0u8; 12];
    let head_len = read_some(file, &mut head)?;
    let head = &head[..head_len];

    if head.len() >= 3 && &head[..3] == b"ID3" {
        return Ok(read_tag_at(file, 0, file_len)?.in_container(SeratoContainer::Mp3));
    }

    let (order, form, container) =
        if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WAVE" {
            (ByteOrder::Little, 4, SeratoContainer::Wav)
        } else if head.len() >= 12
            && &head[..4] == b"FORM"
            && (&head[8..12] == b"AIFF" || &head[8..12] == b"AIFC")
        {
            (ByteOrder::Big, 4, SeratoContainer::Aiff)
        } else {
            // Unsupported container (FLAC, MP4, Ogg, …): no auxiliary data, and no
            // claim that the file has none either.
            return Ok(TagScan::no_tag());
        };

    let declared = order.read_u32(&head[form..form + 4]) as u64;
    let Some(form_end) = declared.checked_add(8) else {
        return Ok(TagScan::incomplete().in_container(container));
    };
    if form_end > file_len {
        // The form header claims more data than the file holds.
        return Ok(TagScan::incomplete().in_container(container));
    }

    match find_metadata_chunk(file, 12, form_end, order)? {
        ChunkSearch::Found { offset, len } => {
            Ok(read_tag_at(file, offset, len)?.in_container(container))
        }
        ChunkSearch::Absent => Ok(TagScan::no_tag().in_container(container)),
        ChunkSearch::Malformed => Ok(TagScan::incomplete().in_container(container)),
    }
}

#[derive(Debug, Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    fn read_u32(self, bytes: &[u8]) -> u32 {
        let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match self {
            ByteOrder::Little => u32::from_le_bytes(bytes),
            ByteOrder::Big => u32::from_be_bytes(bytes),
        }
    }
}

enum ChunkSearch {
    Found { offset: u64, len: u64 },
    Absent,
    Malformed,
}

/// Walk top-level container chunks from `start` to `end`, returning the first
/// `id3 `/`ID3 ` chunk. Chunk payloads are skipped with a seek.
fn find_metadata_chunk(
    file: &mut File,
    start: u64,
    end: u64,
    order: ByteOrder,
) -> anyhow::Result<ChunkSearch> {
    let mut pos = start;
    let mut walked = 0usize;
    while pos < end {
        walked += 1;
        if walked > MAX_CONTAINER_CHUNKS {
            return Ok(ChunkSearch::Malformed);
        }
        if end - pos < 8 {
            // A short tail is only valid as even-byte padding.
            return Ok(ChunkSearch::Malformed);
        }
        let mut header = [0u8; 8];
        read_at(file, pos, &mut header)?;
        let size = order.read_u32(&header[4..8]) as u64;
        let data_start = pos + 8;
        let Some(data_end) = data_start.checked_add(size) else {
            return Ok(ChunkSearch::Malformed);
        };
        if data_end > end {
            return Ok(ChunkSearch::Malformed);
        }
        if &header[..4] == b"id3 " || &header[..4] == b"ID3 " {
            return Ok(ChunkSearch::Found {
                offset: data_start,
                len: size,
            });
        }
        // Chunk payloads are padded to an even length.
        pos = data_end + (size & 1);
        if pos > end {
            return Ok(ChunkSearch::Malformed);
        }
    }
    Ok(ChunkSearch::Absent)
}

/// Read and parse the ID3 tag stored at `offset`, bounded by `available`.
fn read_tag_at(file: &mut File, offset: u64, available: u64) -> anyhow::Result<TagScan> {
    if available < TAG_HEADER_LEN {
        return Ok(TagScan::incomplete());
    }
    let mut header = [0u8; TAG_HEADER_LEN as usize];
    read_at(file, offset, &mut header)?;
    if &header[..3] != b"ID3" {
        return Ok(TagScan::no_tag());
    }
    let Some(size) = syncsafe(&header[6..10]) else {
        return Ok(TagScan::incomplete());
    };
    let total = TAG_HEADER_LEN + u64::from(size);
    if total > MAX_TAG_BYTES || total > available {
        return Ok(TagScan::incomplete());
    }
    let mut tag = vec![0u8; total as usize];
    read_at(file, offset, &mut tag)?;
    Ok(parse_tag(&tag))
}

fn parse_tag(tag: &[u8]) -> TagScan {
    let major = tag[3];
    if major != 3 && major != 4 {
        // Unsupported ID3 version: not an error, just no auxiliary data.
        return TagScan::no_tag();
    }
    let flags = tag[5];
    let Some(size) = syncsafe(&tag[6..10]) else {
        return TagScan::incomplete();
    };
    let Some(body) = tag.get(10..10 + size as usize) else {
        return TagScan::incomplete();
    };

    // ID3v2.3 unsynchronisation applies to the whole tag; undo it first so frame
    // boundaries are interpreted on logical bytes.
    let deunsynchronised;
    let logical_body = if major == 3 && flags & 0x80 != 0 {
        deunsynchronised = deunsynchronise(body);
        deunsynchronised.as_slice()
    } else {
        body
    };

    let mut pos = 0usize;
    if flags & 0x40 != 0 {
        let Some(header_len) = extended_header_len(logical_body, major) else {
            return TagScan::incomplete();
        };
        pos = header_len;
    }

    let mut frames = Vec::new();
    let mut complete = true;
    let mut seen = 0usize;
    while pos + 10 <= logical_body.len() {
        seen += 1;
        if seen > MAX_FRAMES {
            return TagScan::incomplete();
        }
        let id = &logical_body[pos..pos + 4];
        if id[0] == 0 {
            // Padding: everything after this must be zero.
            break;
        }
        if !id
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            return TagScan::incomplete();
        }
        let frame_size = if major == 4 {
            let Some(declared) = syncsafe(&logical_body[pos + 4..pos + 8]) else {
                return TagScan::incomplete();
            };
            declared as usize
        } else {
            u32::from_be_bytes([
                logical_body[pos + 4],
                logical_body[pos + 5],
                logical_body[pos + 6],
                logical_body[pos + 7],
            ]) as usize
        };
        let frame_flags = u16::from_be_bytes([logical_body[pos + 8], logical_body[pos + 9]]);
        let data_start = pos + 10;
        let Some(data_end) = data_start.checked_add(frame_size) else {
            return TagScan::incomplete();
        };
        let Some(raw) = logical_body.get(data_start..data_end) else {
            return TagScan::incomplete();
        };
        pos = data_end;

        let mut frame_unsync = None;
        if major == 4 && (frame_flags & 0x0002 != 0 || flags & 0x80 != 0) {
            // ID3v2.4 unsynchronisation is frame-scoped: the tag-level flag only
            // means "every frame carries it". The frame size read above is the
            // post-unsynchronisation (physical) size, so the encoded bytes are
            // consumed first and decoded here, leaving later frame offsets intact.
            // (v2.3 is the version whose unsynchronisation spans the whole tag.)
            frame_unsync = Some(deunsynchronise(raw));
        }
        let data = frame_unsync.as_deref().unwrap_or(raw);

        let (compressed, encrypted) = if major == 4 {
            (frame_flags & 0x0008 != 0, frame_flags & 0x0004 != 0)
        } else {
            (frame_flags & 0x0080 != 0, frame_flags & 0x0040 != 0)
        };
        let grouping = if major == 4 {
            frame_flags & 0x0040 != 0
        } else {
            frame_flags & 0x0020 != 0
        };
        let data_length_indicator = major == 4 && frame_flags & 0x0001 != 0;

        // Prefix fields precede the frame logical_body: v2.3 stores decompressed size,
        // encryption method, then group id; v2.4 stores group id, encryption
        // method, then the syncsafe data length indicator.
        let mut prefix = 0usize;
        if major == 3 && compressed {
            prefix += 4;
        }
        if encrypted {
            prefix += 1;
        }
        if grouping {
            prefix += 1;
        }
        if data_length_indicator {
            prefix += 4;
        }
        let Some(payload) = data.get(prefix..) else {
            return TagScan::incomplete();
        };

        if id == b"GEOB" {
            if compressed || encrypted {
                // Not decompressing: the frame's content is unknown, so the
                // absence of a frames kind cannot be trusted.
                complete = false;
            } else {
                match parse_geob(payload) {
                    Ok((description, object)) => {
                        if let Some(kind) = frame_kind(&description) {
                            frames.push(RawGeoBFrame {
                                kind,
                                payload: object,
                            });
                        }
                    }
                    // The descriptor may hide behind bytes we cannot decode, so a
                    // missing marker frame cannot be inferred from this tag.
                    Err(()) => complete = false,
                }
            }
        }
    }

    if logical_body
        .get(pos..)
        .is_some_and(|rest| rest.iter().any(|b| *b != 0))
    {
        complete = false;
    }

    TagScan {
        container: None,
        frames,
        scan_complete: complete,
    }
}

/// Length of the version-specific extended header, validated against the logical_body.
///
/// Both versions require at least the size word plus two flag bytes; a header
/// that claims fewer, or that runs past the tag logical_body, is structural corruption
/// and must not be mistaken for a tag with no frames.
fn extended_header_len(logical_body: &[u8], major: u8) -> Option<usize> {
    let size_bytes = logical_body.get(0..4)?;
    let (declared, header_len) = if major == 3 {
        // v2.3: the size word itself is not counted.
        let size = u32::from_be_bytes([size_bytes[0], size_bytes[1], size_bytes[2], size_bytes[3]]);
        let size = usize::try_from(size).ok()?;
        (size, size.checked_add(4)?)
    } else {
        // v2.4: the syncsafe size includes the size word.
        let size = syncsafe(size_bytes)? as usize;
        (size, size)
    };
    if declared < 6 || header_len > logical_body.len() {
        return None;
    }
    Some(header_len)
}

fn frame_kind(description: &str) -> Option<SeratoFrameKind> {
    match description {
        "Serato Markers2" => Some(SeratoFrameKind::Markers2),
        "Serato Markers_" => Some(SeratoFrameKind::MarkersLegacy),
        "Serato BeatGrid" => Some(SeratoFrameKind::BeatGrid),
        "CuePoints" => Some(SeratoFrameKind::MikCuePoints),
        "Key" => Some(SeratoFrameKind::MikKey),
        _ => None,
    }
}

/// Parse a GEOB frame logical_body into its decoded description and object data.
fn parse_geob(data: &[u8]) -> Result<(String, Vec<u8>), ()> {
    let (&encoding, rest) = data.split_first().ok_or(())?;
    if encoding > 3 {
        return Err(());
    }
    let (_mime, rest) = take_text(rest, 0)?;
    let (_filename, rest) = take_text(rest, encoding)?;
    let (description, object) = take_text(rest, encoding)?;
    Ok((description, object.to_vec()))
}

/// Read one encoded, NUL-terminated string, returning it decoded plus the
/// remaining bytes.
fn take_text(data: &[u8], encoding: u8) -> Result<(String, &[u8]), ()> {
    match encoding {
        0 | 3 => {
            let end = data.iter().position(|b| *b == 0).ok_or(())?;
            let text = if encoding == 0 {
                data[..end].iter().map(|b| char::from(*b)).collect()
            } else {
                std::str::from_utf8(&data[..end])
                    .map_err(|_| ())?
                    .to_string()
            };
            Ok((text, &data[end + 1..]))
        }
        _ => {
            // UTF-16: the terminator is two zero bytes on a code-unit boundary.
            let mut end = 0usize;
            loop {
                let pair = data.get(end..end + 2).ok_or(())?;
                if pair == [0, 0] {
                    break;
                }
                end += 2;
            }
            let text = decode_utf16(&data[..end], encoding)?;
            Ok((text, &data[end + 2..]))
        }
    }
}

fn decode_utf16(bytes: &[u8], encoding: u8) -> Result<String, ()> {
    if !bytes.len().is_multiple_of(2) {
        return Err(());
    }
    // Encoding 1 is BOM-qualified; encoding 2 is UTF-16BE with no BOM. A missing
    // BOM is treated as big-endian, matching the wire order of encoding 2.
    let bom = if encoding == 1 { bytes.get(0..2) } else { None };
    let (skip, little_endian) = match bom {
        Some([0xFF, 0xFE]) => (2, true),
        Some([0xFE, 0xFF]) => (2, false),
        _ => (0, false),
    };
    let units: Vec<u16> = bytes[skip..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();
    String::from_utf16(&units).map_err(|_| ())
}

/// Undo ID3 unsynchronisation: every `FF 00` pair becomes a single `FF`.
fn deunsynchronise(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut index = 0usize;
    while index < data.len() {
        out.push(data[index]);
        if data[index] == 0xFF && data.get(index + 1) == Some(&0x00) {
            index += 2;
        } else {
            index += 1;
        }
    }
    out
}

fn syncsafe(bytes: &[u8]) -> Option<u32> {
    if bytes.len() != 4 || bytes.iter().any(|b| *b & 0x80 != 0) {
        return None;
    }
    Some(
        (u32::from(bytes[0]) << 21)
            | (u32::from(bytes[1]) << 14)
            | (u32::from(bytes[2]) << 7)
            | u32::from(bytes[3]),
    )
}

/// Read up to `buf.len()` bytes from the start of the file.
fn read_some(file: &mut File, buf: &mut [u8]) -> std::io::Result<usize> {
    file.seek(SeekFrom::Start(0))?;
    let mut filled = 0usize;
    while filled < buf.len() {
        let read = file.read(&mut buf[filled..])?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    Ok(filled)
}

fn read_at(file: &mut File, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(buf)
}
