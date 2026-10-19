//! ANLZ analysis-file writers (`ANLZ0000.DAT` / `ANLZ0000.EXT`).
//!
//! ANLZ files are a sequence of tagged big-endian sections behind a `PMAI`
//! header. The `.DAT` carries the path, the VBR seek table, the beat grid and
//! the preview waveforms; the `.EXT` carries the detail waveforms and the
//! extended cue lists. CDJ-3000-class players require both — a missing `.EXT`
//! makes them re-analyze the track on load.
//!
//! Section layouts follow Deep Symmetry's *ANLZ Files* reference, the Kaitai
//! `rekordbox_anlz.ksy` specification and the hardware notes published with the
//! *fourfour* `pioneer-usb-writer` document. Two details matter for the
//! hardware to accept a file at all: the `PPTH` path is UTF-16BE **with a
//! trailing NUL**, and the `PQTZ` header is 24 bytes.
//!
//! Written layouts, in file order:
//!
//! ```text
//! .DAT  PMAI (28) | PPTH | PVBR (1620, zeros) | PQTZ | PWAV | PWV2 | PCOB hot | PCOB memory
//! .EXT  PMAI (28) | PPTH | PWV3 | PCOB hot | PCOB memory | PCO2 hot | PCO2 memory | PWV5 | PWV4
//! ```
//!
//! Rekordbox also writes `PQT2`/`PVB2` (`.EXT`) and `PSSI`/`PWVC` sections that
//! carry data this adapter has no source for — phrase analysis, a second seek
//! table and the CDJ-3000's band calibration. Players re-derive all of those, so
//! the files here are complete without them.
//!
//! Notes on the cue entries:
//!
//! - `PCP2` entries are written at the length real exports use,
//!   `88 + len_comment` bytes: the 48 bytes of named fields plus the comment,
//!   then 20 unknown bytes every public parser also skips, then 20 further bytes
//!   real exports append that no public source describes. Both unknown blocks
//!   are zeroed here.
//! - A cue point carries `loop_time` 0 and a loop its end time; a loop also sets
//!   the entry status to `4`.
//! - `hot_cue` is one-based in these files (Hot Cue A is 1, 0 means "not a hot
//!   cue").

use library_core::ExportCue;

use super::model::AnlzInput;

/// Length of the `PMAI` file header every ANLZ file starts with.
const PMAI_HEADER_LEN: u32 = 0x1C;
/// Length of a `PPTH` section header.
const PPTH_HEADER_LEN: u32 = 0x10;
/// Length of a `PVBR` section header.
const VBR_HEADER_LEN: u32 = 0x10;
/// Length of the `PVBR` section: an all-zero seek table.
const PVBR_TAG_LEN: u32 = 1620;
/// Length of a `PQTZ` section header (24, not the 20 some docs quote).
const PQTZ_HEADER_LEN: u32 = 0x18;
/// Length of a `PWAV`/`PWV2` section header.
const PREVIEW_HEADER_LEN: u32 = 0x14;
/// Length of a `PWV3`/`PWV4`/`PWV5` section header.
const DETAIL_HEADER_LEN: u32 = 0x18;
/// Length of a `PCOB` section header.
const CUE_LIST_HEADER_LEN: u32 = 0x18;
/// Length of a `PCO2` section header.
const EXTENDED_CUE_LIST_HEADER_LEN: u32 = 0x14;
/// Length of a `PCPT` cue entry.
const CUE_ENTRY_LEN: u32 = 0x38;
/// Length of a `PCPT` cue entry's own header.
const CUE_ENTRY_HEADER_LEN: u32 = 0x1C;
/// Length of a `PCP2` extended cue entry's own header.
const EXTENDED_CUE_ENTRY_HEADER_LEN: u32 = 0x10;
/// Unknown bytes every `PCP2` entry carries between the RGB color and the end
/// of the fields public parsers know.
const EXTENDED_CUE_ENTRY_UNKNOWN: usize = 20;
/// Unknown bytes real exports append after those fields, beyond the length any
/// public parser describes.
const EXTENDED_CUE_ENTRY_TAIL: usize = 20;
/// The `0003E8` word a cue entry carries after its type.
const MILLIWORD: u32 = 0x0000_03E8;
/// The `00100000` word a `PCPT` cue entry carries after its status.
const CUE_ENTRY_UNKNOWN_WORD: u32 = 0x0010_0000;
/// `PWV3`'s undocumented header word (150 — the entries-per-second rate).
const PWV3_UNKNOWN: u32 = 0x0096_0000;
/// `PWV4`'s undocumented header word.
const PWV4_UNKNOWN: u32 = 0;
/// `PWV5`'s undocumented header word.
const PWV5_UNKNOWN: u32 = 0x0096_0305;
/// Preview sections' undocumented header word.
const PREVIEW_UNKNOWN: u32 = 0x0001_0000;
/// Cue-list type of the hot-cue list.
const HOT_CUES: u32 = 1;
/// Cue-list type of the memory-cue list.
const MEMORY_CUES: u32 = 0;
/// Cue entry status marking an active loop.
const ACTIVE_LOOP: u32 = 4;
/// Sentinel rekordbox writes into a cue list's `memory_count` word.
const MEMORY_COUNT_SENTINEL: u32 = 0xFFFF_FFFF;
/// Cues are counted in a `u16`, so longer lists cannot be described.
const MAX_CUES: usize = u16::MAX as usize;
/// Tracks shorter than this are left ungridded, as rekordbox leaves them.
const MIN_GRIDDED_MS: u32 = 4_000;
/// Cap on a synthesized grid, so a bogus duration cannot ask for a
/// multi-megabyte section.
const MAX_BEATS: usize = 200_000;

/// Serialize the `.DAT` file bytes for a track.
pub fn dat_bytes(input: &AnlzInput<'_>) -> Vec<u8> {
    let waveforms = super::waveform::WaveformPreview::build(input.waveform_overview);
    assemble(&[
        path_section(input),
        vbr_section(),
        beat_grid_section(input),
        preview_section(*b"PWAV", &waveforms.pwav),
        preview_section(*b"PWV2", &waveforms.pwv2),
        cue_list_section(HOT_CUES, input.hot_cues),
        cue_list_section(MEMORY_CUES, input.memory_cues),
    ])
}

/// Serialize the `.EXT` file bytes for a track.
pub fn ext_bytes(input: &AnlzInput<'_>) -> Vec<u8> {
    let preview = super::waveform::WaveformPreview::build(input.waveform_overview);
    let detail = super::waveform::WaveformDetail::build(input.waveform_detail, input.duration_ms);
    // Both detail encodings come from the same tier, so their entry counts agree.
    debug_assert_eq!(detail.pwv3.len() as u64, u64::from(detail.entries));
    debug_assert_eq!(detail.pwv5.len() as u64, u64::from(detail.entries) * 2);
    assemble(&[
        path_section(input),
        waveform_detail_section(*b"PWV3", 1, &detail.pwv3, PWV3_UNKNOWN),
        cue_list_section(HOT_CUES, input.hot_cues),
        cue_list_section(MEMORY_CUES, input.memory_cues),
        extended_cue_list_section(HOT_CUES, input.hot_cues),
        extended_cue_list_section(MEMORY_CUES, input.memory_cues),
        waveform_detail_section(*b"PWV5", 2, &detail.pwv5, PWV5_UNKNOWN),
        waveform_detail_section(*b"PWV4", 6, &preview.pwv4, PWV4_UNKNOWN),
    ])
}

/// Put a `PMAI` header in front of `sections`, claiming the whole file's length.
fn assemble(sections: &[Vec<u8>]) -> Vec<u8> {
    let total = PMAI_HEADER_LEN as usize + sections.iter().map(Vec::len).sum::<usize>();
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(b"PMAI");
    bytes.extend_from_slice(&PMAI_HEADER_LEN.to_be_bytes());
    bytes.extend_from_slice(&u32::try_from(total).unwrap_or(u32::MAX).to_be_bytes());
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    bytes.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    for section in sections {
        bytes.extend_from_slice(section);
    }
    bytes
}

/// Wrap `body` — everything after the twelve-byte tag header — in a section.
fn section(fourcc: [u8; 4], len_header: u32, body: &[u8]) -> Vec<u8> {
    let len_tag = 12 + u32::try_from(body.len()).unwrap_or(u32::MAX);
    let mut bytes = Vec::with_capacity(len_tag as usize);
    bytes.extend_from_slice(&fourcc);
    bytes.extend_from_slice(&len_header.to_be_bytes());
    bytes.extend_from_slice(&len_tag.to_be_bytes());
    bytes.extend_from_slice(body);
    bytes
}

/// `PPTH`: the on-drive audio path as UTF-16BE with its trailing NUL.
///
/// `len_path` counts those NUL bytes; hardware rejects the file and re-analyzes
/// the track if it does not.
fn path_section(input: &AnlzInput<'_>) -> Vec<u8> {
    let units: Vec<u16> = input.device_audio_path.encode_utf16().collect();
    let len_path = ((units.len() + 1) * 2) as u32;
    let mut body = Vec::with_capacity(4 + len_path as usize);
    body.extend_from_slice(&len_path.to_be_bytes());
    for unit in units {
        body.extend_from_slice(&unit.to_be_bytes());
    }
    body.extend_from_slice(&0_u16.to_be_bytes());
    section(*b"PPTH", PPTH_HEADER_LEN, &body)
}

/// `PVBR`: the VBR seek table, always zeros for the lossless and CBR files this
/// adapter exports. The tag is a fixed 1620 bytes.
fn vbr_section() -> Vec<u8> {
    section(*b"PVBR", VBR_HEADER_LEN, &[0; PVBR_TAG_LEN as usize - 12])
}

/// `PQTZ`: the beat grid.
fn beat_grid_section(input: &AnlzInput<'_>) -> Vec<u8> {
    let beats = beat_entries(input);
    let mut body = Vec::with_capacity(12 + beats.len() * 8);
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&0x0008_0000_u32.to_be_bytes());
    body.extend_from_slice(&(beats.len() as u32).to_be_bytes());
    for (number, tempo, time) in beats {
        body.extend_from_slice(&number.to_be_bytes());
        body.extend_from_slice(&tempo.to_be_bytes());
        body.extend_from_slice(&time.to_be_bytes());
    }
    section(*b"PQTZ", PQTZ_HEADER_LEN, &body)
}

/// Beat grid entries as `(bar position, centi-BPM, time in milliseconds)`.
///
/// A stored grid is written as-is, with the tempo taken from `bpm` (which is
/// what rekordbox analyzes) or, failing that, from the median beat spacing.
/// Without a stored grid a constant one is laid across the track from `bpm`;
/// tracks with no usable BPM, or shorter than [`MIN_GRIDDED_MS`], stay ungridded.
fn beat_entries(input: &AnlzInput<'_>) -> Vec<(u16, u16, u32)> {
    if let Some(grid) = input.beat_grid {
        if !grid.beats.is_empty() {
            let per_bar = usize::from(grid.beats_per_bar.max(1));
            let tempo = tempo_from_bpm(input.bpm)
                .or_else(|| tempo_from_spacing(&grid.beats))
                .unwrap_or(0);
            return grid
                .beats
                .iter()
                .enumerate()
                .map(|(index, seconds)| {
                    (
                        ((index % per_bar) + 1) as u16,
                        tempo,
                        millis(f64::from(*seconds) * 1_000.0),
                    )
                })
                .collect();
        }
    }

    let Some(bpm) = usable_bpm(input.bpm) else {
        return Vec::new();
    };
    if input.duration_ms < MIN_GRIDDED_MS {
        return Vec::new();
    }
    let tempo = centi_bpm(bpm);
    let per_bar = input
        .beat_grid
        .map_or(4, |grid| usize::from(grid.beats_per_bar.max(1)));
    let beat_ms = 60_000.0 / bpm;
    let beats = (f64::from(input.duration_ms) / beat_ms).floor() as usize + 1;
    (0..beats.min(MAX_BEATS))
        .map(|index| {
            (
                ((index % per_bar) + 1) as u16,
                tempo,
                millis(index as f64 * beat_ms),
            )
        })
        .collect()
}

/// A BPM worth gridding with.
fn usable_bpm(bpm: Option<f64>) -> Option<f64> {
    bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.0)
}

/// BPM as the centi-BPM word the beat grid stores.
fn centi_bpm(bpm: f64) -> u16 {
    (bpm * 100.0).round().clamp(1.0, f64::from(u16::MAX)) as u16
}

/// BPM as the centi-BPM word the beat grid stores, when the BPM is usable.
fn tempo_from_bpm(bpm: Option<f64>) -> Option<u16> {
    usable_bpm(bpm).map(centi_bpm)
}

/// The tempo of a stored grid whose BPM is unknown: the median beat spacing.
fn tempo_from_spacing(beats: &[f32]) -> Option<u16> {
    let mut gaps: Vec<f32> = beats
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|gap| gap.is_finite() && *gap > 0.0)
        .collect();
    if gaps.is_empty() {
        return None;
    }
    gaps.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    Some(centi_bpm(60.0 / f64::from(gaps[gaps.len() / 2])))
}

/// A millisecond count as the word the beat grid stores.
fn millis(value_ms: f64) -> u32 {
    value_ms.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

/// `PWAV`/`PWV2`: a fixed-length preview behind a length word and a
/// `0x00010000` word.
fn preview_section(fourcc: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(8 + data.len());
    body.extend_from_slice(&(data.len() as u32).to_be_bytes());
    body.extend_from_slice(&PREVIEW_UNKNOWN.to_be_bytes());
    body.extend_from_slice(data);
    section(fourcc, PREVIEW_HEADER_LEN, &body)
}

/// `PWV3`/`PWV4`/`PWV5`: a variable-length waveform behind its entry size, entry
/// count and an undocumented word.
fn waveform_detail_section(
    fourcc: [u8; 4],
    entry_bytes: u32,
    data: &[u8],
    unknown: u32,
) -> Vec<u8> {
    let entries = (data.len() as u32) / entry_bytes;
    let mut body = Vec::with_capacity(12 + data.len());
    body.extend_from_slice(&entry_bytes.to_be_bytes());
    body.extend_from_slice(&entries.to_be_bytes());
    body.extend_from_slice(&unknown.to_be_bytes());
    body.extend_from_slice(data);
    section(fourcc, DETAIL_HEADER_LEN, &body)
}

/// `PCOB`: a hot-cue or memory-cue list of fixed-length `PCPT` entries.
fn cue_list_section(list_type: u32, cues: &[ExportCue]) -> Vec<u8> {
    let cues = &cues[..cues.len().min(MAX_CUES)];
    let mut body = Vec::with_capacity(12 + cues.len() * CUE_ENTRY_LEN as usize);
    body.extend_from_slice(&list_type.to_be_bytes());
    body.extend_from_slice(&0_u16.to_be_bytes());
    body.extend_from_slice(&(cues.len() as u16).to_be_bytes());
    body.extend_from_slice(&MEMORY_COUNT_SENTINEL.to_be_bytes());
    for (index, cue) in cues.iter().enumerate() {
        body.extend_from_slice(&cue_entry(cue, index, cues.len()));
    }
    section(*b"PCOB", CUE_LIST_HEADER_LEN, &body)
}

/// One 56-byte `PCPT` cue entry.
fn cue_entry(cue: &ExportCue, index: usize, count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(CUE_ENTRY_LEN as usize);
    bytes.extend_from_slice(b"PCPT");
    bytes.extend_from_slice(&CUE_ENTRY_HEADER_LEN.to_be_bytes());
    bytes.extend_from_slice(&CUE_ENTRY_LEN.to_be_bytes());
    bytes.extend_from_slice(
        &cue.hot_cue_slot
            .map_or(0, |slot| u32::from(slot) + 1)
            .to_be_bytes(),
    );
    let status = if cue.loop_end_ms.is_some() {
        ACTIVE_LOOP
    } else {
        0
    };
    bytes.extend_from_slice(&status.to_be_bytes());
    bytes.extend_from_slice(&CUE_ENTRY_UNKNOWN_WORD.to_be_bytes());
    bytes.extend_from_slice(&order_first(index).to_be_bytes());
    bytes.extend_from_slice(&order_last(index, count).to_be_bytes());
    bytes.push(cue_type(cue));
    bytes.push(0);
    bytes.extend_from_slice(&(MILLIWORD as u16).to_be_bytes());
    bytes.extend_from_slice(&cue.position_ms.to_be_bytes());
    bytes.extend_from_slice(&cue.loop_end_ms.unwrap_or(0).to_be_bytes());
    bytes.extend_from_slice(&[0; 16]);
    bytes
}

/// `PCO2`: a hot-cue or memory-cue list of variable-length `PCP2` entries.
fn extended_cue_list_section(list_type: u32, cues: &[ExportCue]) -> Vec<u8> {
    let cues = &cues[..cues.len().min(MAX_CUES)];
    let mut body = Vec::new();
    body.extend_from_slice(&list_type.to_be_bytes());
    body.extend_from_slice(&(cues.len() as u16).to_be_bytes());
    body.extend_from_slice(&0_u16.to_be_bytes());
    for cue in cues {
        body.extend_from_slice(&extended_cue_entry(cue));
    }
    section(*b"PCO2", EXTENDED_CUE_LIST_HEADER_LEN, &body)
}

/// One variable-length `PCP2` cue entry.
///
/// The entry is `88 + comment bytes` long, the length real exports use: the
/// 16-byte tag header, 32 bytes of cue data, the length-prefixed UTF-16BE
/// comment and the four hot-cue color bytes, then 20 unknown bytes every parser
/// skips, then the 20 further bytes real exports append.
fn extended_cue_entry(cue: &ExportCue) -> Vec<u8> {
    let comment = utf16be(cue.label.as_deref().unwrap_or(""));
    let len_entry = 48 + comment.len() + EXTENDED_CUE_ENTRY_UNKNOWN + EXTENDED_CUE_ENTRY_TAIL;
    let (red, green, blue) = cue_color(cue.color.as_deref());

    let mut bytes = Vec::with_capacity(len_entry);
    bytes.extend_from_slice(b"PCP2");
    bytes.extend_from_slice(&EXTENDED_CUE_ENTRY_HEADER_LEN.to_be_bytes());
    bytes.extend_from_slice(&(len_entry as u32).to_be_bytes());
    bytes.extend_from_slice(
        &cue.hot_cue_slot
            .map_or(0, |slot| u32::from(slot) + 1)
            .to_be_bytes(),
    );
    bytes.push(cue_type(cue));
    bytes.push(0);
    bytes.extend_from_slice(&(MILLIWORD as u16).to_be_bytes());
    bytes.extend_from_slice(&cue.position_ms.to_be_bytes());
    // A cue point has no loop time; a loop stores where the player jumps back to.
    bytes.extend_from_slice(&cue.loop_end_ms.unwrap_or(0).to_be_bytes());
    // Memory cues would carry a color-table id here, which the library model
    // has no equivalent for.
    bytes.push(0);
    // Seven unknown bytes: 0x01 then zeros, which is what rekordbox writes.
    bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let (numerator, denominator) = quantized_loop(cue);
    bytes.extend_from_slice(&numerator.to_be_bytes());
    bytes.extend_from_slice(&denominator.to_be_bytes());
    bytes.extend_from_slice(&(comment.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&comment);
    bytes.push(0);
    bytes.extend_from_slice(&[red, green, blue]);
    bytes.extend_from_slice(&[0; EXTENDED_CUE_ENTRY_UNKNOWN]);
    bytes.extend_from_slice(&[0; EXTENDED_CUE_ENTRY_TAIL]);
    bytes
}

/// Whether an entry is a point (`1`) or a loop (`2`).
fn cue_type(cue: &ExportCue) -> u8 {
    if cue.loop_end_ms.is_some() {
        2
    } else {
        1
    }
}

/// `order_first`: `0xFFFF` for the first cue in a list, otherwise its rank.
fn order_first(index: usize) -> u16 {
    match index {
        0 => u16::MAX,
        index => u16::try_from(index - 1).unwrap_or(u16::MAX),
    }
}

/// `order_last`: the cue's rank, `0xFFFF` for the last cue in a list.
fn order_last(index: usize, count: usize) -> u16 {
    if index + 1 == count {
        u16::MAX
    } else {
        u16::try_from(index + 1).unwrap_or(u16::MAX)
    }
}

/// The loop size as the beat numerator/denominator pair rekordbox writes for
/// quantized loops: a four-beat loop is `4/1`, an unquantized loop `0/0`.
fn quantized_loop(cue: &ExportCue) -> (u16, u16) {
    if cue.loop_end_ms.is_none() {
        return (0, 0);
    }
    match cue.loop_length_beats {
        Some(beats) if beats > 0 => (u16::try_from(beats).unwrap_or(0), 1),
        _ => (0, 0),
    }
}

/// A `#RRGGBB` cue color as bytes, black when absent or malformed.
fn cue_color(color: Option<&str>) -> (u8, u8, u8) {
    let Some(digits) = color.and_then(|color| color.strip_prefix('#')) else {
        return (0, 0, 0);
    };
    let parsed = u32::from_str_radix(digits, 16)
        .ok()
        .filter(|_| digits.len() == 6);
    match parsed {
        Some(rgb) => (
            (rgb >> 16) as u8,
            ((rgb >> 8) & 0xFF) as u8,
            (rgb & 0xFF) as u8,
        ),
        None => (0, 0, 0),
    }
}

/// UTF-16BE bytes with the trailing NUL the format's strings carry.
fn utf16be(text: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    if !bytes.is_empty() {
        bytes.extend_from_slice(&0_u16.to_be_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinRead;
    use library_core::{ExportBeatGrid, SpectralPeak};
    use rekordcrate::anlz::{
        Content, CueListType, CueType, ExtendedCue, ExtendedCueList, WaveformColorDetail,
        WaveformPreview, ANLZ,
    };
    use rekordcrate::util::ColorIndex;

    use super::*;

    /// Owns the analysis data an [`AnlzInput`] borrows.
    struct Fixture {
        path: String,
        bpm: Option<f64>,
        duration_ms: u32,
        grid: Option<ExportBeatGrid>,
        overview: Vec<SpectralPeak>,
        detail: Vec<SpectralPeak>,
        hot: Vec<ExportCue>,
        memory: Vec<ExportCue>,
    }

    impl Fixture {
        fn new(path: &str, duration_ms: u32) -> Self {
            Self {
                path: path.to_string(),
                bpm: None,
                duration_ms,
                grid: None,
                overview: Vec::new(),
                detail: Vec::new(),
                hot: Vec::new(),
                memory: Vec::new(),
            }
        }

        fn input(&self) -> AnlzInput<'_> {
            AnlzInput {
                device_audio_path: &self.path,
                bpm: self.bpm,
                duration_ms: self.duration_ms,
                beat_grid: self.grid.as_ref(),
                waveform_overview: &self.overview,
                waveform_detail: &self.detail,
                hot_cues: &self.hot,
                memory_cues: &self.memory,
            }
        }
    }

    fn peak(low: f32, mid: f32, high: f32) -> SpectralPeak {
        SpectralPeak { low, mid, high }
    }

    fn cue(slot: Option<u8>, position_ms: u32, loop_end_ms: Option<u32>) -> ExportCue {
        ExportCue {
            hot_cue_slot: slot,
            position_ms,
            loop_end_ms,
            loop_length_beats: None,
            label: None,
            color: None,
        }
    }

    /// One tagged section of a written file.
    struct RawSection<'a> {
        fourcc: [u8; 4],
        len_header: u32,
        len_tag: u32,
        body: &'a [u8],
    }

    /// Walk a written ANLZ file, checking the `PMAI` header and every section's
    /// self-reported length as it goes.
    fn raw_sections(bytes: &[u8]) -> Vec<RawSection<'_>> {
        assert_eq!(&bytes[..4], b"PMAI", "file magic");
        let len_header = u32_at(bytes, 4);
        assert_eq!(len_header, PMAI_HEADER_LEN, "PMAI len_header");
        assert_eq!(
            u32_at(bytes, 8) as usize,
            bytes.len(),
            "PMAI len_file must count the whole file"
        );

        let mut sections = Vec::new();
        let mut offset = len_header as usize;
        while offset < bytes.len() {
            let fourcc: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
            let header_len = u32_at(bytes, offset + 4);
            let tag_len = u32_at(bytes, offset + 8);
            assert!(tag_len >= 12, "{fourcc:?} len_tag includes the header");
            let body = &bytes[offset + 12..offset + tag_len as usize];
            assert_eq!(
                tag_len as usize,
                12 + body.len(),
                "{fourcc:?} len_tag matches the bytes written"
            );
            assert!(
                header_len <= tag_len,
                "{fourcc:?} len_header fits inside the tag"
            );
            sections.push(RawSection {
                fourcc,
                len_header: header_len,
                len_tag: tag_len,
                body,
            });
            offset += tag_len as usize;
        }
        assert_eq!(offset, bytes.len(), "sections tile the file exactly");
        sections
    }

    fn u16_at(bytes: &[u8], offset: usize) -> u16 {
        u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap())
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    /// The sections of a written file with the given four-character code.
    fn sections_named<'a>(
        sections: &'a [RawSection<'a>],
        fourcc: &[u8; 4],
    ) -> Vec<&'a RawSection<'a>> {
        sections
            .iter()
            .filter(|section| &section.fourcc == fourcc)
            .collect()
    }

    fn read_anlz(bytes: &[u8]) -> ANLZ {
        ANLZ::read(&mut Cursor::new(bytes)).expect("rekordcrate parses the written file")
    }

    fn beat_grid_of(anlz: &ANLZ) -> &rekordcrate::anlz::BeatGrid {
        anlz.sections
            .iter()
            .find_map(|section| match &section.content {
                Content::BeatGrid(grid) => Some(grid),
                _ => None,
            })
            .expect("beat grid section")
    }

    fn cue_lists_of(anlz: &ANLZ) -> Vec<&rekordcrate::anlz::CueList> {
        anlz.sections
            .iter()
            .filter_map(|section| match &section.content {
                Content::CueList(list) => Some(list),
                _ => None,
            })
            .collect()
    }

    fn extended_cue_lists_of(anlz: &ANLZ) -> Vec<&ExtendedCueList> {
        anlz.sections
            .iter()
            .filter_map(|section| match &section.content {
                Content::ExtendedCueList(list) => Some(list),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn dat_sections_are_ordered_and_report_their_own_lengths() {
        let mut fixture = Fixture::new("/Contents/Artist/Album/track.flac", 30_000);
        fixture.bpm = Some(128.0);
        fixture.overview = vec![peak(0.5, 0.5, 0.5); 2048];
        fixture.detail = vec![peak(0.5, 0.5, 0.5); 4_500];
        fixture.hot = vec![cue(Some(0), 1_000, None)];
        fixture.memory = vec![cue(None, 2_000, Some(4_000))];

        let bytes = dat_bytes(&fixture.input());
        let sections = raw_sections(&bytes);
        let codes: Vec<[u8; 4]> = sections.iter().map(|section| section.fourcc).collect();
        assert_eq!(
            codes,
            [*b"PPTH", *b"PVBR", *b"PQTZ", *b"PWAV", *b"PWV2", *b"PCOB", *b"PCOB"]
        );
        let headers: Vec<u32> = sections.iter().map(|section| section.len_header).collect();
        assert_eq!(headers, [0x10, 0x10, 0x18, 0x14, 0x14, 0x18, 0x18]);
        assert_eq!(sections_named(&sections, b"PVBR")[0].len_tag, PVBR_TAG_LEN);

        let bytes = ext_bytes(&fixture.input());
        let sections = raw_sections(&bytes);
        let codes: Vec<[u8; 4]> = sections.iter().map(|section| section.fourcc).collect();
        assert_eq!(
            codes,
            [*b"PPTH", *b"PWV3", *b"PCOB", *b"PCOB", *b"PCO2", *b"PCO2", *b"PWV5", *b"PWV4"]
        );
        let headers: Vec<u32> = sections.iter().map(|section| section.len_header).collect();
        assert_eq!(headers, [0x10, 0x18, 0x18, 0x18, 0x14, 0x14, 0x18, 0x18]);
    }

    #[test]
    fn ppth_holds_a_utf16be_path_with_its_trailing_nul() {
        for path in [
            "/Contents/Artist/Album/track.flac",
            "/Contents/Ärtist/Álbum/トラック.wav",
        ] {
            let fixture = Fixture::new(path, 5_000);
            let bytes = dat_bytes(&fixture.input());
            let sections = raw_sections(&bytes);
            let section = sections_named(&sections, b"PPTH")[0];

            let units = path.encode_utf16().count();
            assert_eq!(
                u32_at(section.body, 0) as usize,
                (units + 1) * 2,
                "len_path"
            );
            assert_eq!(section.len_tag as usize, 16 + (units + 1) * 2);
            assert_eq!(&section.body[4..], &utf16be(path)[..]);
            assert_eq!(&section.body[section.body.len() - 2..], &[0, 0]);

            let anlz = read_anlz(&bytes);
            let parsed = anlz
                .sections
                .iter()
                .find_map(|section| match &section.content {
                    Content::Path(path) => Some(path.path.to_string()),
                    _ => None,
                })
                .expect("path section");
            assert_eq!(parsed, path);
        }
    }

    #[test]
    fn stored_beat_grids_keep_their_position_tempo_and_time() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 3_000);
        fixture.bpm = Some(120.0);
        fixture.grid = Some(ExportBeatGrid {
            beats: vec![0.0, 0.5, 1.0, 1.5, 2.0],
            beats_per_bar: 4,
        });

        let bytes = dat_bytes(&fixture.input());
        let sections = raw_sections(&bytes);
        let section = sections_named(&sections, b"PQTZ")[0];
        assert_eq!(section.len_tag, 24 + 8 * 5);
        assert_eq!(u32_at(section.body, 0), 0);
        assert_eq!(u32_at(section.body, 4), 0x0008_0000);
        assert_eq!(u32_at(section.body, 8), 5);
        let entries: Vec<(u16, u16, u32)> = (0..5)
            .map(|index| {
                let offset = 12 + index * 8;
                (
                    u16_at(section.body, offset),
                    u16_at(section.body, offset + 2),
                    u32_at(section.body, offset + 4),
                )
            })
            .collect();
        assert_eq!(
            entries,
            [
                (1, 12_000, 0),
                (2, 12_000, 500),
                (3, 12_000, 1_000),
                (4, 12_000, 1_500),
                (1, 12_000, 2_000),
            ]
        );

        let anlz = read_anlz(&bytes);
        let grid = beat_grid_of(&anlz);
        assert_eq!(grid.beats.len(), 5);
        assert_eq!(grid.beats[2].beat_number, 3);
        assert_eq!(grid.beats[2].tempo, 12_000);
        assert_eq!(grid.beats[2].time, 1_000);
        assert_eq!(grid.beats[4].beat_number, 1);
    }

    #[test]
    fn a_grid_is_synthesized_from_bpm_across_the_track() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 60_000);
        fixture.bpm = Some(128.0);

        let bytes = dat_bytes(&fixture.input());
        let anlz = read_anlz(&bytes);
        let grid = beat_grid_of(&anlz);
        assert_eq!(grid.beats.len(), 129);
        assert_eq!(
            (
                grid.beats[0].beat_number,
                grid.beats[0].tempo,
                grid.beats[0].time
            ),
            (1, 12_800, 0)
        );
        assert_eq!((grid.beats[1].beat_number, grid.beats[1].time), (2, 469));
        assert_eq!((grid.beats[4].beat_number, grid.beats[4].time), (1, 1_875));
        assert_eq!(grid.beats[128].time, 60_000);
    }

    #[test]
    fn grids_without_a_usable_bpm_or_length_are_left_empty() {
        // Too short to be worth gridding, as rekordbox leaves them.
        let mut short = Fixture::new("/Contents/track.mp3", 3_000);
        short.bpm = Some(120.0);
        let bytes = dat_bytes(&short.input());
        let sections = raw_sections(&bytes);
        assert_eq!(sections_named(&sections, b"PQTZ")[0].len_tag, 24);
        assert!(beat_grid_of(&read_anlz(&bytes)).beats.is_empty());

        for bpm in [None, Some(0.0), Some(-120.0), Some(f64::NAN)] {
            let mut fixture = Fixture::new("/Contents/track.mp3", 300_000);
            fixture.bpm = bpm;
            assert!(
                beat_grid_of(&read_anlz(&dat_bytes(&fixture.input())))
                    .beats
                    .is_empty(),
                "bpm {bpm:?} cannot produce a grid"
            );
        }
    }

    #[test]
    fn a_stored_grid_is_used_even_when_the_track_is_short() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 2_000);
        fixture.grid = Some(ExportBeatGrid {
            beats: vec![0.0, 1.0],
            beats_per_bar: 4,
        });

        let beats = beat_grid_of(&read_anlz(&dat_bytes(&fixture.input())))
            .beats
            .len();
        assert_eq!(beats, 2, "a stored grid is authoritative");
    }

    #[test]
    fn waveform_sections_carry_pioneer_entry_counts() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 20_000);
        fixture.overview = vec![peak(1.0, 0.0, 0.0); 2048];
        fixture.detail = vec![peak(0.0, 0.0, 0.0); 3_000];

        let dat = dat_bytes(&fixture.input());
        let sections = raw_sections(&dat);
        let preview = sections_named(&sections, b"PWAV")[0];
        assert_eq!(preview.len_tag, 420);
        assert_eq!(u32_at(preview.body, 0), 400);
        assert_eq!(u32_at(preview.body, 4), PREVIEW_UNKNOWN);
        assert!(preview.body[8..].iter().all(|byte| *byte == 31));
        assert_eq!(sections_named(&sections, b"PWV2")[0].len_tag, 120);

        let anlz = read_anlz(&dat);
        let previews: Vec<&WaveformPreview> = anlz
            .sections
            .iter()
            .filter_map(|section| match &section.content {
                Content::WaveformPreview(preview) => Some(preview),
                _ => None,
            })
            .collect();
        assert_eq!(previews.len(), 1);
        assert_eq!(previews[0].data.len(), 400);
        assert_eq!(previews[0].data[0].height(), 31);
        assert_eq!(previews[0].data[0].whiteness(), 0);
        let tiny = anlz
            .sections
            .iter()
            .find_map(|section| match &section.content {
                Content::TinyWaveformPreview(tiny) => Some(tiny),
                _ => None,
            })
            .expect("tiny preview");
        assert_eq!(tiny.data.len(), 100);
        assert_eq!(tiny.data[0].height(), 15);

        let ext = ext_bytes(&fixture.input());
        let sections = raw_sections(&ext);
        let detail = sections_named(&sections, b"PWV3")[0];
        assert_eq!(detail.len_tag, 24 + 3_000);
        assert_eq!(u32_at(detail.body, 0), 1);
        assert_eq!(u32_at(detail.body, 4), 3_000);
        assert_eq!(u32_at(detail.body, 8), PWV3_UNKNOWN);
        let color = sections_named(&sections, b"PWV5")[0];
        assert_eq!(color.len_tag, 24 + 6_000);
        assert_eq!(u32_at(color.body, 0), 2);
        assert_eq!(u32_at(color.body, 4), 3_000);
        assert_eq!(u32_at(color.body, 8), PWV5_UNKNOWN);
        let preview_color = sections_named(&sections, b"PWV4")[0];
        assert_eq!(preview_color.len_tag, 7_224);
        assert_eq!(u32_at(preview_color.body, 0), 6);
        assert_eq!(u32_at(preview_color.body, 4), 1_200);

        let anlz = read_anlz(&ext);
        let detail = anlz
            .sections
            .iter()
            .find_map(|section| match &section.content {
                Content::WaveformDetail(detail) => Some(detail),
                _ => None,
            })
            .expect("detail waveform");
        assert_eq!(detail.data.len(), 3_000);
        let color = anlz
            .sections
            .iter()
            .find_map(|section| match &section.content {
                Content::WaveformColorDetail(color) => Some(color),
                _ => None,
            })
            .expect("color detail waveform");
        assert_eq!(color.data.len(), 3_000);
    }

    /// Decode a PWV5 entry the way rekordbox packs it: `R(3) | G(3) | B(3) |
    /// height(5) | low(2)`, most significant bits first.
    ///
    /// The ordering is verified against real exports — a `.2EX` 3-band waveform
    /// correlates with these fields at 0.80/0.84/0.58 for low/mid/high, while
    /// reading the pair the other way round correlates negatively — which is why
    /// the raw bytes are decoded here instead of through rekordcrate's
    /// `WaveformColorDetailColumn` accessors: that parser reads the pair
    /// little-endian, so its colors come back byte-swapped. Its entry count and
    /// section envelope are still checked below.
    fn pwv5_entry(pair: [u8; 2]) -> (u8, u8, u8, u8) {
        let packed = u16::from_be_bytes(pair);
        (
            ((packed >> 13) & 0b111) as u8,
            ((packed >> 10) & 0b111) as u8,
            ((packed >> 7) & 0b111) as u8,
            ((packed >> 2) & 0b1_1111) as u8,
        )
    }

    #[test]
    fn detail_peaks_survive_into_the_color_waveform() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 4_000);
        fixture.detail = vec![peak(0.0, 0.0, 0.0); 600];
        fixture.detail[150] = peak(1.0, 0.5, 0.0);

        let ext = ext_bytes(&fixture.input());
        let sections = raw_sections(&ext);
        let section = sections_named(&sections, b"PWV5")[0];
        let entry = |index: usize| {
            let offset = 12 + index * 2;
            pwv5_entry([section.body[offset], section.body[offset + 1]])
        };
        assert_eq!(entry(149), (0, 0, 0, 0));
        assert_eq!(entry(150), (7, 4, 0, 31));
        assert_eq!(entry(151), (0, 0, 0, 0));

        let color = read_anlz(&ext);
        let detail = color
            .sections
            .iter()
            .find_map(|section| match &section.content {
                Content::WaveformColorDetail(WaveformColorDetail { data, .. }) => Some(data),
                _ => None,
            })
            .expect("color detail waveform");
        assert_eq!(detail.len(), 600);
    }

    #[test]
    fn color_preview_columns_carry_bands_and_amplitude() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 4_000);
        fixture.overview = vec![peak(1.0, 0.0, 0.0); 1_200];
        fixture.overview[600] = peak(0.0, 0.4, 0.8);

        let ext = ext_bytes(&fixture.input());
        let sections = raw_sections(&ext);
        let section = sections_named(&sections, b"PWV4")[0];
        assert_eq!(&section.body[12..18], &[255, 0, 128, 255, 0, 0]);
        let column = &section.body[12 + 600 * 6..12 + 600 * 6 + 6];
        assert_eq!(column, [204, 0, 51, 0, 102, 204]);

        let anlz = read_anlz(&ext);
        let columns = anlz
            .sections
            .iter()
            .find_map(|section| match &section.content {
                Content::WaveformColorPreview(preview) => Some(preview),
                _ => None,
            })
            .expect("color preview");
        assert_eq!(columns.data.len(), 1_200);
        assert_eq!(columns.data[0].energy_bottom_third_freq, 255);
        assert_eq!(columns.data[600].energy_mid_third_freq, 102);
        assert_eq!(columns.data[600].energy_top_third_freq, 204);
    }

    #[test]
    fn silent_input_still_writes_full_length_waveforms() {
        let fixture = Fixture::new("/Contents/track.mp3", 10_000);

        let dat = dat_bytes(&fixture.input());
        let sections = raw_sections(&dat);
        assert_eq!(sections_named(&sections, b"PWAV")[0].body[8..], [0; 400]);
        assert_eq!(sections_named(&sections, b"PWV2")[0].body[8..], [0; 100]);
        assert_eq!(sections_named(&sections, b"PQTZ")[0].len_tag, 24);

        let ext = ext_bytes(&fixture.input());
        let sections = raw_sections(&ext);
        let detail = sections_named(&sections, b"PWV3")[0];
        assert_eq!(detail.len_tag, 24 + 1_500);
        assert!(detail.body[12..].iter().all(|byte| *byte == 0));
        let color = sections_named(&sections, b"PWV5")[0];
        assert_eq!(color.len_tag, 24 + 3_000);
        assert!(color.body[12..].iter().all(|byte| *byte == 0));
        assert_eq!(sections_named(&sections, b"PWV4")[0].len_tag, 7_224);

        let anlz = read_anlz(&ext);
        let lists = extended_cue_lists_of(&anlz);
        assert_eq!(lists.len(), 2);
        assert_eq!(lists[0].list_type, CueListType::HotCues);
        assert!(lists[0].cues.is_empty());
        assert_eq!(lists[1].list_type, CueListType::MemoryCues);
        assert!(lists[1].cues.is_empty());
    }

    #[test]
    fn cue_lists_split_hot_from_memory_cues() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 60_000);
        fixture.hot = vec![cue(Some(0), 1_000, None), cue(Some(2), 5_000, Some(7_000))];
        fixture.memory = vec![cue(None, 2_000, None), cue(None, 3_000, Some(3_800))];

        let dat = dat_bytes(&fixture.input());
        let sections = raw_sections(&dat);
        let lists = sections_named(&sections, b"PCOB");
        assert_eq!(lists.len(), 2);
        for (index, list_type) in [(0, HOT_CUES), (1, MEMORY_CUES)] {
            let section = lists[index];
            assert_eq!(u32_at(section.body, 0), list_type);
            assert_eq!(u16_at(section.body, 4), 0);
            assert_eq!(u16_at(section.body, 6), 2);
            assert_eq!(u32_at(section.body, 8), MEMORY_COUNT_SENTINEL);
            assert_eq!(section.len_tag, 24 + 2 * CUE_ENTRY_LEN);
        }

        // Byte-level check of the first hot entry (a cue point).
        let entry = &lists[0].body[12..12 + 56];
        assert_eq!(&entry[..4], b"PCPT");
        assert_eq!(u32_at(entry, 4), 0x1C);
        assert_eq!(u32_at(entry, 8), 0x38);
        assert_eq!(u32_at(entry, 12), 1, "hot cue A is one-based");
        assert_eq!(u32_at(entry, 16), 0, "a point is not an active loop");
        assert_eq!(u32_at(entry, 20), CUE_ENTRY_UNKNOWN_WORD);
        assert_eq!(u16_at(entry, 24), 0xFFFF, "order_first of the first cue");
        assert_eq!(u16_at(entry, 26), 1);
        assert_eq!(entry[28], 1, "a cue point is type 1");
        assert_eq!(&entry[29..32], &[0x00, 0x03, 0xE8]);
        assert_eq!(u32_at(entry, 32), 1_000);
        assert_eq!(u32_at(entry, 36), 0, "a cue point has no loop time");
        assert_eq!(entry[40..], [0; 16]);

        let anlz = read_anlz(&dat);
        let lists = cue_lists_of(&anlz);
        assert_eq!(lists.len(), 2);
        assert_eq!(lists[0].list_type, CueListType::HotCues);
        assert_eq!(lists[0].cues.len(), 2);
        assert_eq!(lists[0].cues[0].hot_cue, 1);
        assert_eq!(lists[0].cues[0].cue_type, CueType::Point);
        assert_eq!(lists[0].cues[0].status, 0);
        assert_eq!(lists[0].cues[0].time, 1_000);
        assert_eq!(lists[0].cues[0].loop_time, 0);
        assert_eq!(lists[0].cues[1].hot_cue, 3);
        assert_eq!(lists[0].cues[1].cue_type, CueType::Loop);
        assert_eq!(lists[0].cues[1].status, ACTIVE_LOOP);
        assert_eq!(lists[0].cues[1].time, 5_000);
        assert_eq!(lists[0].cues[1].loop_time, 7_000);
        assert_eq!(
            (lists[0].cues[1].order_first, lists[0].cues[1].order_last),
            (0, 0xFFFF)
        );

        assert_eq!(lists[1].list_type, CueListType::MemoryCues);
        assert_eq!(lists[1].cues.len(), 2);
        assert_eq!(lists[1].cues[0].hot_cue, 0, "a memory cue is not a hot cue");
        assert_eq!(lists[1].cues[0].time, 2_000);
        assert_eq!(lists[1].cues[1].cue_type, CueType::Loop);
        assert_eq!(lists[1].cues[1].loop_time, 3_800);
    }

    #[test]
    fn extended_cues_carry_comments_colors_and_loop_sizes() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 60_000);
        let mut labelled = cue(Some(1), 4_000, None);
        labelled.label = Some("Drop".to_string());
        labelled.color = Some("#FF0000".to_string());
        let mut looped = cue(None, 3_000, Some(5_000));
        looped.loop_length_beats = Some(4);
        looped.color = Some("not a color".to_string());
        fixture.hot = vec![labelled];
        fixture.memory = vec![looped];

        let ext = ext_bytes(&fixture.input());
        let sections = raw_sections(&ext);
        let lists = sections_named(&sections, b"PCO2");
        assert_eq!(lists.len(), 2);
        // "Drop" is four UTF-16 units plus the trailing NUL.
        assert_eq!(u32_at(lists[0].body, 0), HOT_CUES);
        assert_eq!(u16_at(lists[0].body, 4), 1);
        assert_eq!(u16_at(lists[0].body, 6), 0);
        assert_eq!(lists[0].len_tag, 20 + 88 + 10);
        let entry = &lists[0].body[8..];
        assert_eq!(&entry[..4], b"PCP2");
        assert_eq!(u32_at(entry, 4), 0x10);
        assert_eq!(u32_at(entry, 8), 98);
        assert_eq!(u32_at(entry, 12), 2, "hot cue B is one-based");
        assert_eq!(entry[16], 1);
        assert_eq!(&entry[17..20], &[0x00, 0x03, 0xE8]);
        assert_eq!(u32_at(entry, 20), 4_000);
        assert_eq!(u32_at(entry, 24), 0, "a cue point has no loop time");
        assert_eq!(entry[28], 0, "memory-cue color id");
        assert_eq!(&entry[29..36], &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(u16_at(entry, 36), 0);
        assert_eq!(u16_at(entry, 38), 0);
        assert_eq!(u32_at(entry, 40), 10, "len_comment counts the trailing NUL");
        assert_eq!(&entry[44..54], &utf16be("Drop")[..]);
        assert_eq!(entry[54], 0, "color_code");
        assert_eq!(&entry[55..58], &[0xFF, 0x00, 0x00]);
        assert_eq!(entry[58..78], [0; EXTENDED_CUE_ENTRY_UNKNOWN]);
        assert_eq!(entry[78..], [0; EXTENDED_CUE_ENTRY_TAIL]);

        // The memory loop: no label, a black color and a quantized loop size.
        let entry = &lists[1].body[8..];
        assert_eq!(u32_at(entry, 8), 88);
        assert_eq!(u32_at(entry, 12), 0);
        assert_eq!(entry[16], 2);
        assert_eq!(u32_at(entry, 20), 3_000);
        assert_eq!(u32_at(entry, 24), 5_000);
        assert_eq!(u16_at(entry, 36), 4, "loop numerator");
        assert_eq!(u16_at(entry, 38), 1, "loop denominator");
        assert_eq!(u32_at(entry, 40), 0);
        assert_eq!(&entry[44..48], &[0, 0, 0, 0]);
        assert_eq!(lists[1].len_tag, 20 + 88);

        let anlz = read_anlz(&ext);
        let lists = extended_cue_lists_of(&anlz);
        assert_eq!(lists.len(), 2);
        let hot: &ExtendedCue = &lists[0].cues[0];
        assert_eq!(hot.hot_cue, 2);
        assert_eq!(hot.cue_type, CueType::Point);
        assert_eq!(hot.time, 4_000);
        assert_eq!(&*hot.comment, "Drop");
        assert_eq!(hot.hot_cue_color_index, 0);
        assert_eq!(hot.hot_cue_color_rgb, (0xFF, 0x00, 0x00));
        assert_eq!(hot.color, ColorIndex::None);
        assert_eq!(hot.trailing.len(), EXTENDED_CUE_ENTRY_TAIL);
        let memory: &ExtendedCue = &lists[1].cues[0];
        assert_eq!(memory.hot_cue, 0);
        assert_eq!(memory.cue_type, CueType::Loop);
        assert_eq!(memory.loop_numerator, 4);
        assert_eq!(memory.loop_denominator, 1);
        assert_eq!(&*memory.comment, "");
        assert_eq!(memory.hot_cue_color_rgb, (0, 0, 0));
        assert_eq!(memory.trailing.len(), EXTENDED_CUE_ENTRY_TAIL);
    }

    #[test]
    fn malformed_cue_colors_are_written_black() {
        assert_eq!(cue_color(None), (0, 0, 0));
        assert_eq!(cue_color(Some("ff0000")), (0, 0, 0));
        assert_eq!(cue_color(Some("#FF00")), (0, 0, 0));
        assert_eq!(cue_color(Some("#GGGGGG")), (0, 0, 0));
        assert_eq!(cue_color(Some("#0a10Ff")), (0x0A, 0x10, 0xFF));
    }

    #[test]
    fn cue_entries_survive_the_byte_level_round_trip() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 60_000);
        let mut looped = cue(Some(0), 9_000, Some(10_500));
        looped.loop_length_beats = Some(3);
        fixture.hot = vec![looped];

        let ext = ext_bytes(&fixture.input());
        let sections = raw_sections(&ext);
        let entry = &sections_named(&sections, b"PCO2")[0].body[8..];
        assert_eq!(u32_at(entry, 8), 88, "no comment, so no extra bytes");
        assert_eq!(u16_at(entry, 36), 3);
        assert_eq!(u16_at(entry, 38), 1);

        let anlz = read_anlz(&ext);
        let lists = extended_cue_lists_of(&anlz);
        assert_eq!(lists[0].cues[0].loop_numerator, 3);
        assert_eq!(lists[0].cues[0].loop_denominator, 1);
        assert_eq!(lists[0].cues[0].loop_time, 10_500);
    }

    /// Every cue entry is self-describing: a parser keys on the declared length,
    /// so a declared-versus-written drift would silently shift everything after
    /// it in the file.
    #[test]
    fn cue_entries_declare_exactly_the_bytes_they_occupy() {
        let mut fixture = Fixture::new("/Contents/track.mp3", 60_000);
        let mut intro = cue(Some(0), 1_000, None);
        intro.label = Some("Intro".to_string());
        intro.color = Some("#00FF00".to_string());
        let mut looped = cue(Some(1), 5_000, Some(6_000));
        looped.label = Some("Verse".to_string());
        looped.loop_length_beats = Some(8);
        fixture.hot = vec![intro, looped];
        fixture.memory = vec![cue(None, 2_000, None), cue(None, 3_000, Some(3_500))];

        // PCPT entries are fixed-length.
        let dat = dat_bytes(&fixture.input());
        let sections = raw_sections(&dat);
        for section in sections_named(&sections, b"PCOB") {
            let count = usize::from(u16_at(section.body, 6));
            let mut cursor = 12;
            for _ in 0..count {
                assert_eq!(&section.body[cursor..cursor + 4], b"PCPT");
                assert_eq!(u32_at(section.body, cursor + 8), CUE_ENTRY_LEN);
                cursor += CUE_ENTRY_LEN as usize;
            }
            assert_eq!(cursor, section.body.len(), "PCPT entries tile the section");
        }

        // PCP2 entries describe their own length: 88 bytes plus the comment.
        let ext = ext_bytes(&fixture.input());
        let sections = raw_sections(&ext);
        for section in sections_named(&sections, b"PCO2") {
            let count = usize::from(u16_at(section.body, 4));
            let mut cursor = 8;
            for _ in 0..count {
                assert_eq!(&section.body[cursor..cursor + 4], b"PCP2");
                let declared = u32_at(section.body, cursor + 8) as usize;
                let comment = u32_at(section.body, cursor + 40) as usize;
                assert_eq!(declared, 88 + comment, "len_entry covers the whole entry");
                cursor += declared;
            }
            assert_eq!(cursor, section.body.len(), "PCP2 entries tile the section");
        }

        let anlz = read_anlz(&ext);
        let lists = extended_cue_lists_of(&anlz);
        assert_eq!(&*lists[0].cues[0].comment, "Intro");
        assert_eq!(lists[0].cues[0].hot_cue_color_rgb, (0x00, 0xFF, 0x00));
        assert_eq!(&*lists[0].cues[1].comment, "Verse");
        assert_eq!(lists[0].cues[1].loop_numerator, 8);
        assert_eq!(lists[0].cues[1].loop_denominator, 1);
        assert_eq!(lists[1].cues[0].cue_type, CueType::Point);
        assert_eq!(lists[1].cues[1].cue_type, CueType::Loop);
    }
}
