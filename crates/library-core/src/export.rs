//! Export capability: write a target format from a [`Library`].
//!
//! [`Exportable`] is the mirror of [`crate::Migratable`]: importers copy an
//! external library *into* the user's manager, exporters write the manager's
//! collections *out* to a foreign format or device layout (CDJ USB, Rekordbox
//! XML, M3U, …).
//!
//! Exporters live in `library-adapters` and depend on [`ExportSource`] — never
//! on the concrete manager — so format code stays out of the canonical store,
//! exactly like migration. The manager implements [`ExportSource`] because the
//! analysis a device format needs (beat grids, waveforms, cue points) is not
//! part of the read-only [`Library`] trait; adapters never run DSP themselves.

use std::path::Path;

use audio_core::SpectralPeak;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::traits::Library;
use crate::types::{CollectionId, TrackId};

/// Entries per second of [`TrackExportData::waveform_detail`].
///
/// Device analysis files store a high-resolution waveform at this rate; the
/// export source is responsible for producing the tier, exporters for packing
/// it into their own encoding.
pub const EXPORT_WAVEFORM_ENTRIES_PER_SECOND: u16 = 150;

/// Longest edge of [`TrackExportData::artwork`] thumbnails, in pixels.
///
/// Device formats store one small cover per distinct artwork; 80 px is the size
/// Rekordbox writes for the primary thumbnail, so it is what exporters should
/// be handed rather than the user's full-resolution cover.
pub const EXPORT_ARTWORK_EDGE_PX: u32 = 80;

/// Which parts of a library an export writes, and how.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportOptions {
    /// Playlists to write, in the caller's order. Empty exports no playlists.
    pub playlists: Vec<CollectionId>,
    /// Folder collections to mirror as directories of audio files. Empty
    /// exports no folders.
    ///
    /// Folder membership is path-prefix based, so a folder export reproduces
    /// the directory tree below the collection's `fs_path`; tracks are still
    /// registered in the target's track database so device playlists can point
    /// at them.
    pub folders: Vec<CollectionId>,
    /// Copy audio files even when a file with the same name and size already
    /// sits at the target path.
    pub refresh_files: bool,
}

impl ExportOptions {
    /// Export every playlist and folder collection, reusing files already on
    /// the target.
    pub fn all(playlists: Vec<CollectionId>, folders: Vec<CollectionId>) -> Self {
        Self {
            playlists,
            folders,
            refresh_files: false,
        }
    }
}

/// Result of an export run.
///
/// Per-item problems never abort a run: they are recorded in [`errors`] and
/// the remaining items still export, mirroring [`crate::ScanReport`].
///
/// [`errors`]: ExportReport::errors
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportReport {
    /// Tracks this run registered in the target (and copied when their file was
    /// not already there). Exports that merge into an existing target — the CDJ
    /// USB drive accumulates per-collection runs — leave what earlier runs wrote
    /// alone, so these counts describe the run, not the whole target.
    pub tracks: usize,
    /// Tracks whose audio file was already present at the target path and was
    /// reused unchanged.
    pub tracks_reused: usize,
    /// Playlists written to the target.
    pub playlists: usize,
    /// Folder collections mirrored as directories of files.
    pub folders: usize,
    /// Bytes written to the target: audio files and cover thumbnails.
    pub bytes_copied: u64,
    /// Per-item failures (label-only paths); the run continues past them.
    pub errors: Vec<String>,
    /// Non-fatal fidelity notes: dropped data, missing analysis, clamped
    /// values.
    pub warnings: Vec<String>,
}

/// Beat grid in export currency: beat positions in seconds plus the meter.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExportBeatGrid {
    /// Beat positions in seconds from the start of the track.
    pub beats: Vec<f32>,
    /// Beats per bar (meter). Device formats default to 4.
    pub beats_per_bar: u8,
}

/// A hot cue, memory cue or saved loop an exporter may write into a device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportCue {
    /// Hot-cue slot for a hot cue; `None` for a memory cue or saved loop.
    pub hot_cue_slot: Option<u8>,
    /// Cue position — the loop start for loops — in milliseconds.
    pub position_ms: u32,
    /// Loop end in milliseconds, `None` for a plain cue point.
    pub loop_end_ms: Option<u32>,
    /// Loop length in beats when the loop is quantized to the beat grid.
    pub loop_length_beats: Option<u32>,
    /// DJ-assigned label.
    pub label: Option<String>,
    /// Assigned color as `#RRGGBB`.
    pub color: Option<String>,
}

/// Cover thumbnail an exporter may write for a track.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExportArtwork {
    /// JPEG bytes, at most [`EXPORT_ARTWORK_EDGE_PX`] on the longest edge.
    ///
    /// Re-encoded by the export source, so a target never has to decode or scale
    /// whatever format the file's tags happened to carry.
    pub jpeg: Vec<u8>,
}

/// Track data an exporter needs beyond file tags.
///
/// Analysis, cues and the few library facts device formats store (bit depth,
/// date added). Waveform tiers are Mixar's own ([`SpectralPeak`] triples,
/// normalized to the track peak); each target format repacks them into its own
/// encoding.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackExportData {
    /// Detected BPM; used as a constant-tempo fallback when no grid is stored.
    pub bpm: Option<f64>,
    /// Beat grid, when the track has been analyzed.
    pub beat_grid: Option<ExportBeatGrid>,
    /// Fixed-length overview waveform
    /// ([`audio_core::OVERVIEW_SAMPLE_COUNT`] entries).
    pub waveform_overview: Vec<SpectralPeak>,
    /// Detail waveform sampled at [`EXPORT_WAVEFORM_ENTRIES_PER_SECOND`],
    /// covering the whole track. Empty when the file could not be decoded.
    pub waveform_detail: Vec<SpectralPeak>,
    /// Hot cues, memory cues and saved loops, ordered by slot.
    pub cues: Vec<ExportCue>,
    /// Cover thumbnail, when the file carries embedded artwork.
    pub artwork: Option<ExportArtwork>,
    /// Audio bit depth in bits, when the decoder reports it.
    pub bit_depth: Option<u16>,
    /// Date the track entered the library as `YYYY-MM-DD`, when known.
    pub added_at: Option<String>,
}

/// Source of the analysis and extras an [`Exportable`] format needs.
///
/// Implemented by the canonical manager (`library::LibraryManager`); adapters
/// depend on this trait rather than the concrete store.
pub trait ExportSource: Library {
    /// Analysis, waveform tiers and cue points for one track.
    ///
    /// `Ok(None)` means the track carries no data at all — a stream, a track
    /// whose file is missing, or one that was never analyzed. Exporters then
    /// fall back to file tags and empty waveforms instead of failing.
    fn export_data(&self, id: &TrackId) -> Result<Option<TrackExportData>>;
}

/// Writes a target format (CDJ USB layout, Rekordbox XML, M3U, …) from a
/// [`Library`].
///
/// Implementations own their file layout below `destination`; they must never
/// mutate the source library.
pub trait Exportable {
    /// Target format name (e.g. `"cdj-usb"`).
    fn name(&self) -> &'static str;

    /// Write the collections selected by `options` into `destination`,
    /// creating it when missing.
    fn export(
        &self,
        source: &dyn ExportSource,
        destination: &Path,
        options: &ExportOptions,
    ) -> Result<ExportReport>;
}
