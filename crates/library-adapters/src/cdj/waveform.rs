//! Repacking Mixar's waveform tiers into Pioneer's on-drive waveform
//! encodings.
//!
//! Mixar produces [`SpectralPeak`] triples (peak amplitude per low/mid/high
//! band, normalized to the track peak) at two resolutions: a fixed-length
//! overview for its own displays and a detail tier at 150 entries per second.
//! Pioneer wants five encodings built from them, split the way the files are:
//! [`WaveformPreview`] (PWAV, PWV2, PWV4) comes from the overview, and
//! [`WaveformDetail`] (PWV3, PWV5) from the detail tier.
//!
//! - **PWAV** — 400-byte monochrome preview, `whiteness:3 | height:5`.
//! - **PWV2** — 100-byte tiny preview, 4-bit height.
//! - **PWV3** — monochrome detail, one `whiteness:3 | height:5` byte per entry,
//!   driving needle search and the jog display.
//! - **PWV4** — 1200-entry color preview, six bytes per entry.
//! - **PWV5** — color detail, a two-byte `rgb + height` pack per entry.
//!
//! Only array resampling and bit packing happen here: no audio is touched.
//!
//! Field layouts follow Deep Symmetry's *ANLZ Files* reference
//! (<https://djl-analysis.deepsymmetry.org/rekordbox-export-analysis/anlz.html>),
//! cross-checked against real rekordbox exports: the PWV5 packing below
//! (`R|G|B|height|00`, most significant bits first) is the one that reproduces
//! the band levels of a real `.2EX` 3-band waveform, and the PWV4 field order
//! matches `WaveformColorPreviewColumn` in the `rekordcrate` parser. Pioneer
//! does not document how it derives the monochrome `whiteness` bits; the
//! heuristic used here — the high band's level — is flagged where it matters.

use library_core::SpectralPeak;

/// Bytes per second of Pioneer's detail waveforms, which the export source
/// samples its detail tier at.
pub const DETAIL_ENTRIES_PER_SECOND: u16 = 150;

const PWAV_COLUMNS: usize = 400;
const PWV2_COLUMNS: usize = 100;
const PWV4_COLUMNS: usize = 1200;

/// Bytes per PWV4 color-preview entry.
const PWV4_ENTRY_BYTES: usize = 6;
/// Bytes per PWV5 color-detail entry.
const PWV5_ENTRY_BYTES: usize = 2;

/// Largest value of the five-bit column heights (PWAV, PWV3, PWV5).
const MAX_HEIGHT: u8 = 31;
/// Largest value of PWV2's four-bit column height.
const MAX_TINY_HEIGHT: u8 = 15;
/// Largest value of PWV5's three-bit color components.
const MAX_COLOR: u8 = 7;
/// Largest value of PWV4's eight-bit energies.
const MAX_ENERGY: u8 = 255;

/// The three preview encodings, built from Mixar's overview tier.
///
/// These live in the `.DAT` (and `PWV4` in the `.EXT`), so they are built
/// independently of the detail tier: a `.DAT` write must not pay for the
/// detailed encodings.
#[derive(Clone, Debug, Default)]
pub struct WaveformPreview {
    /// `PWAV`: monochrome preview, 400 bytes.
    pub pwav: Vec<u8>,
    /// `PWV2`: tiny preview, 100 bytes.
    pub pwv2: Vec<u8>,
    /// `PWV4`: color preview, six bytes per entry.
    pub pwv4: Vec<u8>,
}

impl WaveformPreview {
    /// Resample the overview tier into the three preview encodings.
    pub fn build(overview: &[SpectralPeak]) -> Self {
        Self {
            pwav: monochrome(&resample(overview, PWAV_COLUMNS)),
            pwv2: tiny(&resample(overview, PWV2_COLUMNS)),
            pwv4: color_preview(&resample(overview, PWV4_COLUMNS)),
        }
    }
}

/// The two detail encodings, built from Mixar's 150-entries-per-second tier.
#[derive(Clone, Debug, Default)]
pub struct WaveformDetail {
    /// `PWV3`: monochrome detail, one byte per entry.
    pub pwv3: Vec<u8>,
    /// `PWV5`: color detail, two bytes per entry.
    pub pwv5: Vec<u8>,
    /// Entry count both encodings carry.
    pub entries: u32,
}

impl WaveformDetail {
    /// Resample the detail tier into the two detail encodings.
    ///
    /// The tier's own length fixes the entry count; `duration_ms` is only the
    /// fallback used when no detail waveform is available and a silent one must
    /// still cover the track.
    pub fn build(detail: &[SpectralPeak], duration_ms: u32) -> Self {
        let entries = if detail.is_empty() {
            detail_entries_for(duration_ms)
        } else {
            detail.len().try_into().unwrap_or(u32::MAX)
        };
        let columns = resample(detail, entries as usize);
        Self {
            pwv3: monochrome(&columns),
            pwv5: color_detail(&columns),
            entries,
        }
    }
}

/// Detail entries covering `duration_ms`: Pioneer's 150 half-frames per second.
fn detail_entries_for(duration_ms: u32) -> u32 {
    let seconds = u64::from(duration_ms) / 1_000;
    let entries = seconds * u64::from(DETAIL_ENTRIES_PER_SECOND);
    u32::try_from(entries).unwrap_or(u32::MAX)
}

/// One resampled waveform column: the range's peak band level plus its mean
/// band levels.
#[derive(Clone, Copy, Debug, Default)]
struct Column {
    /// `max(low, mid, high)` over the bucket, the column's drawn height.
    peak: f32,
    low: f32,
    mid: f32,
    high: f32,
}

/// Resample `source` into exactly `columns` buckets of the source array.
///
/// Buckets are contiguous, half-open source ranges, so a source shorter than
/// the target leaves buckets empty (silent) rather than repeating data. `peak`
/// takes the bucket maximum and the bands its mean.
fn resample(source: &[SpectralPeak], columns: usize) -> Vec<Column> {
    let mut buckets = Vec::with_capacity(columns);
    if columns == 0 {
        return buckets;
    }
    let len = source.len();
    for index in 0..columns {
        let start = index * len / columns;
        let end = (index + 1) * len / columns;
        let mut bucket = Column::default();
        let (mut low, mut mid, mut high) = (0.0_f32, 0.0_f32, 0.0_f32);
        for peak in &source[start..end] {
            bucket.peak = bucket.peak.max(peak.low.max(peak.mid).max(peak.high));
            low += peak.low;
            mid += peak.mid;
            high += peak.high;
        }
        let count = (end - start) as f32;
        if count > 0.0 {
            bucket.low = low / count;
            bucket.mid = mid / count;
            bucket.high = high / count;
        }
        buckets.push(bucket);
    }
    buckets
}

/// Scale a 0–1 level onto `0..=max`.
///
/// Out-of-range levels saturate and NaN (`clamp` keeps it, and the float-to-int
/// cast turns it into zero) lands on zero instead of panicking, because
/// analysis data can carry either.
fn level(value: f32, max: u8) -> u8 {
    let maximum = f32::from(max);
    (value * maximum).round().clamp(0.0, maximum) as u8
}

/// `whiteness:3 | height:5`, as PWAV and PWV3 store it.
///
/// The whiteness bits track the bucket's high band: Pioneer's own derivation is
/// undocumented, so brighter (treble-heavy) columns are drawn whiter.
fn monochrome_byte(column: &Column) -> u8 {
    (level(column.high, MAX_COLOR) << 5) | level(column.peak, MAX_HEIGHT)
}

/// Monochrome preview/detail bytes, one per column.
fn monochrome(columns: &[Column]) -> Vec<u8> {
    columns.iter().map(monochrome_byte).collect()
}

/// PWV2 bytes: the four-bit height in the low bits, the rest unused.
fn tiny(columns: &[Column]) -> Vec<u8> {
    columns
        .iter()
        .map(|column| level(column.peak, MAX_TINY_HEIGHT))
        .collect()
}

/// PWV4 entries: `[amplitude, 0, energy <10 kHz, low, mid, high]`.
///
/// Deep Symmetry documents the last four bytes as energy in the bottom half and
/// the bottom/mid/top thirds of the frequency range, and the first two as
/// unknown ("somehow encodes the whiteness"). We use byte 0 for the column's
/// amplitude scaled to 0–255 and leave byte 1 zero, which is the best-effort
/// encoding of the two undocumented bytes.
fn color_preview(columns: &[Column]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(columns.len() * PWV4_ENTRY_BYTES);
    for column in columns {
        bytes.push(level(column.peak, MAX_ENERGY));
        bytes.push(0);
        bytes.push(level((column.low + column.mid) / 2.0, MAX_ENERGY));
        bytes.push(level(column.low, MAX_ENERGY));
        bytes.push(level(column.mid, MAX_ENERGY));
        bytes.push(level(column.high, MAX_ENERGY));
    }
    bytes
}

/// PWV5 entries: a big-endian `u16` packing `R(3) | G(3) | B(3) | height(5) | 0(2)`.
///
/// The low band drives red, the mid band green and the high band blue, which is
/// what Deep Symmetry documents and what real exports encode.
fn color_detail(columns: &[Column]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(columns.len() * PWV5_ENTRY_BYTES);
    for column in columns {
        let packed = (u16::from(level(column.low, MAX_COLOR)) << 13)
            | (u16::from(level(column.mid, MAX_COLOR)) << 10)
            | (u16::from(level(column.high, MAX_COLOR)) << 7)
            | (u16::from(level(column.peak, MAX_HEIGHT)) << 2);
        bytes.extend_from_slice(&packed.to_be_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `count` identical peaks.
    fn flat(count: usize, peak: SpectralPeak) -> Vec<SpectralPeak> {
        vec![peak; count]
    }

    fn silent() -> SpectralPeak {
        SpectralPeak {
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }

    #[test]
    fn overview_tiers_get_pioneer_lengths() {
        let overview = flat(2048, silent());
        let waveforms = WaveformPreview::build(&overview);
        assert_eq!(waveforms.pwav.len(), 400);
        assert_eq!(waveforms.pwv2.len(), 100);
        assert_eq!(waveforms.pwv4.len(), 1200 * 6);
    }

    #[test]
    fn detail_tier_length_fixes_the_detail_entry_count() {
        let detail = flat(3_007, silent());
        let waveforms = WaveformDetail::build(&detail, 20_000);
        assert_eq!(waveforms.entries, 3_007);
        assert_eq!(waveforms.pwv3.len(), 3_007);
        assert_eq!(waveforms.pwv5.len(), 3_007 * 2);
    }

    #[test]
    fn missing_tiers_still_cover_the_track_silently() {
        let preview = WaveformPreview::build(&[]);
        assert_eq!(preview.pwav, vec![0; 400]);
        assert_eq!(preview.pwv2, vec![0; 100]);
        assert_eq!(preview.pwv4, vec![0; 7_200]);

        let detail = WaveformDetail::build(&[], 20_000);
        assert_eq!(detail.entries, 3_000);
        assert_eq!(detail.pwv3, vec![0; 3_000]);
        assert_eq!(detail.pwv5, vec![0; 6_000]);
    }

    #[test]
    fn a_zero_duration_shortens_the_detail_tier_to_nothing() {
        let detail = WaveformDetail::build(&[], 0);
        assert_eq!(detail.entries, 0);
        assert!(detail.pwv3.is_empty());
        assert!(detail.pwv5.is_empty());
        assert_eq!(WaveformPreview::build(&[]).pwav.len(), 400);
    }

    #[test]
    fn a_peak_lands_in_its_bucket_and_nowhere_else() {
        let mut overview = flat(2048, silent());
        overview[0] = SpectralPeak {
            low: 1.0,
            mid: 0.0,
            high: 0.0,
        };
        let waveforms = WaveformPreview::build(&overview);
        // The loud source entry falls into the first bucket of every tier.
        assert_eq!(waveforms.pwav[0], MAX_HEIGHT);
        assert_eq!(waveforms.pwav[1..], vec![0; 399][..]);
        assert_eq!(waveforms.pwv2[0], MAX_TINY_HEIGHT);
        assert_eq!(waveforms.pwv2[1..], vec![0; 99][..]);
        assert_eq!(&waveforms.pwv4[..6], &[255, 0, 128, 255, 0, 0]);
        assert_eq!(waveforms.pwv4[6..], vec![0; 7_194][..]);
    }

    #[test]
    fn detail_peaks_survive_verbatim() {
        let mut detail = flat(600, silent());
        detail[7] = SpectralPeak {
            low: 1.0,
            mid: 0.5,
            high: 0.0,
        };
        let waveforms = WaveformDetail::build(&detail, 4_000);
        assert_eq!(waveforms.entries, 600);
        assert_eq!(waveforms.pwv3[7], MAX_HEIGHT);
        assert_eq!(waveforms.pwv3.iter().filter(|byte| **byte != 0).count(), 1);
        // R(3) | G(3) | B(3) | height(5) | 00: blue is zero here.
        let packed = u16::from_be_bytes([waveforms.pwv5[14], waveforms.pwv5[15]]);
        assert_eq!(packed, (7 << 13) | (4 << 10) | (31 << 2));
    }

    #[test]
    fn whiteness_tracks_the_high_band() {
        let overview = flat(
            2048,
            SpectralPeak {
                low: 0.0,
                mid: 0.0,
                high: 1.0,
            },
        );
        let waveforms = WaveformPreview::build(&overview);
        assert_eq!(waveforms.pwav[0], (MAX_COLOR << 5) | MAX_HEIGHT);
        assert!(waveforms.pwav.iter().all(|byte| *byte == 255));
        assert_eq!(&waveforms.pwv4[..6], &[255, 0, 0, 0, 0, 255]);
    }

    #[test]
    fn out_of_range_and_non_finite_levels_are_clamped() {
        // A 400-entry overview maps one-to-one onto PWAV columns, so every
        // source level lands in a byte of its own.
        let mut overview = flat(400, silent());
        overview[0] = SpectralPeak {
            low: 4.0,
            mid: -2.0,
            high: f32::NAN,
        };
        overview[1] = SpectralPeak {
            low: f32::INFINITY,
            mid: 0.5,
            high: f32::NAN,
        };
        let waveforms = WaveformPreview::build(&overview);
        assert_eq!(waveforms.pwav[0], MAX_HEIGHT, "amplitude saturates");
        assert_eq!(waveforms.pwav[0] & 0b1110_0000, 0, "NaN whiteness is zero");
        assert_eq!(waveforms.pwav[1], MAX_HEIGHT, "infinity saturates");
        assert_eq!(waveforms.pwv2[0], MAX_TINY_HEIGHT);
        // The 400-entry overview only fills the PWV4 columns that own a source
        // entry, so the first two of its 1200 are silent.
        assert_eq!(&waveforms.pwv4[..12], &[0; 12]);
        assert_eq!(&waveforms.pwv4[12..18], &[255, 0, 255, 255, 0, 0]);
        assert_eq!(
            WaveformDetail::build(&[], 1_000).pwv3,
            vec![0; 150],
            "no detail tier to encode"
        );
    }

    #[test]
    fn a_short_overview_still_fills_the_columns() {
        // Four source entries cannot fill four hundred buckets, so only the
        // buckets that own a source entry carry it; the rest stay silent.
        let overview = flat(
            4,
            SpectralPeak {
                low: 1.0,
                mid: 1.0,
                high: 1.0,
            },
        );
        let waveforms = WaveformPreview::build(&overview);
        assert_eq!(waveforms.pwav.len(), 400);
        assert_eq!(waveforms.pwav[0], 0);
        assert_eq!(waveforms.pwav[99], 255);
        assert_eq!(
            waveforms.pwav.iter().filter(|byte| **byte == 255).count(),
            4
        );
        assert_eq!(waveforms.pwv2.iter().filter(|byte| **byte == 15).count(), 4);
    }
}
