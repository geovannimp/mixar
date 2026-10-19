//! Neutral input types for the Pioneer export writers.
//!
//! The exporter translates Mixar's library model into these flat structs, so
//! `pdb`/`anlz` never reach back into the library, the database or the DSP
//! pipeline.

use library_core::{ExportBeatGrid, ExportCue, SpectralPeak};
use serde::{Deserialize, Serialize};

/// Pioneer's file-type discriminant, written into each track row.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u16)]
pub enum CdjFileType {
    /// Not a file type the hardware can play.
    Unknown = 0x00,
    /// MPEG audio (`.mp3`).
    Mp3 = 0x01,
    /// AAC in an MPEG-4 container (`.m4a`, `.mp4`, `.aac`).
    M4a = 0x04,
    /// FLAC (`.flac`).
    Flac = 0x05,
    /// RIFF WAVE (`.wav`).
    Wav = 0x0B,
    /// AIFF (`.aiff`, `.aif`).
    Aiff = 0x0C,
}

impl CdjFileType {
    /// Classify from a file extension (case-insensitive, without the dot).
    ///
    /// Returns [`CdjFileType::Unknown`] for anything the CDJ/XDJ line cannot
    /// play; the exporter skips those tracks.
    pub fn from_extension(extension: &str) -> Self {
        match extension.to_ascii_lowercase().as_str() {
            "mp3" => Self::Mp3,
            "m4a" | "mp4" | "aac" => Self::M4a,
            "flac" => Self::Flac,
            "wav" => Self::Wav,
            "aiff" | "aif" => Self::Aiff,
            _ => Self::Unknown,
        }
    }
}

/// One track written into the device database and analysis files.
///
/// Paths are relative to the export root and start with `/`.
///
/// Serialized into the drive's export manifest, so repeated runs can merge with
/// what earlier runs wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CdjTrack {
    /// Track title.
    pub title: String,
    /// Primary artist (empty strings are written as-is; the exporter
    /// substitutes placeholders before building the row).
    pub artist: String,
    /// Album name.
    pub album: String,
    /// Genre, omitted from the database when `None` or empty.
    pub genre: Option<String>,
    /// BPM, written as whole hundredths.
    pub bpm: Option<f64>,
    /// Pioneer key-table id (1–24); `None` leaves the key unknown.
    pub key_id: Option<u32>,
    /// Track length in whole seconds.
    pub duration_secs: u32,
    /// Audio file size in bytes.
    pub file_size: u64,
    /// Bitrate in kbps, when known.
    pub bitrate_kbps: Option<u32>,
    /// Sample rate in Hz.
    pub sample_rate_hz: u32,
    /// Bit depth in bits.
    pub bit_depth: u16,
    /// Container/codec discriminant.
    pub file_type: CdjFileType,
    /// Artwork-table id of this track's cover, or `0` for no artwork.
    ///
    /// Ids come from [`CdjArtwork::id`] and are per-export: tracks sharing the
    /// same cover share the id.
    pub artwork_id: u32,
    /// `YYYY-MM-DD` the track entered the library, when known.
    pub date_added: Option<String>,
    /// On-drive audio path, e.g. `/Contents/Artist/Album/01 Title.mp3`.
    pub device_audio_path: String,
    /// On-drive path of the track's `.DAT` analysis file, stored in the
    /// database's `analyze_path` field. The hardware recomputes this path
    /// itself (see [`crate::cdj::path_hash`]) but Rekordbox writes it.
    pub analyze_path: String,
}

/// One playlist written into the device database.
///
/// `entries` index into the track slice handed to the writer, in playlist
/// order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CdjPlaylist {
    /// Playlist name shown on the device.
    pub name: String,
    /// Track indices, in play order.
    pub entries: Vec<usize>,
}

/// One cover thumbnail written to the drive.
///
/// Every distinct cover gets exactly one row and one file, referenced by the
/// tracks that use it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CdjArtwork {
    /// Artwork-table id, 1-based and in the order the exporter interned the
    /// covers.
    pub id: u32,
    /// On-drive path of the thumbnail,
    /// e.g. `/PIONEER/Artwork/00001/a1.jpg`.
    pub path: String,
}

/// Everything the ANLZ writers need for one track.
///
/// The waveform tiers are Mixar's own; the serializer resamples and repacks
/// them into Pioneer's encodings without touching audio.
#[derive(Clone, Copy, Debug)]
pub struct AnlzInput<'a> {
    /// On-drive audio path, written into the `PPTH` section and used for the
    /// analysis-directory hash.
    pub device_audio_path: &'a str,
    /// BPM, written into the beat grid.
    pub bpm: Option<f64>,
    /// Track length in milliseconds.
    pub duration_ms: u32,
    /// Analyzed beat grid, when the track has one.
    pub beat_grid: Option<&'a ExportBeatGrid>,
    /// Fixed-length overview waveform (Mixar's `OVERVIEW_SAMPLE_COUNT` tier).
    pub waveform_overview: &'a [SpectralPeak],
    /// Detail waveform at the export rate.
    pub waveform_detail: &'a [SpectralPeak],
    /// Hot cues and hot loops, ordered by slot.
    pub hot_cues: &'a [ExportCue],
    /// Memory cues and saved loops, ordered by slot.
    pub memory_cues: &'a [ExportCue],
}
