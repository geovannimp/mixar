//! Export source: the analysis, waveform tiers and cue points format adapters
//! need when writing out to a device or another application's layout.
//!
//! Adapters depend on [`library_core::ExportSource`], not on this manager, so
//! the DSP-adjacent work (decoding, waveform tiers, beat grids, cue reads)
//! stays here and format code stays in `library-adapters`. Embedded cover art is
//! re-encoded here too, once per track, at
//! [`library_core::EXPORT_ARTWORK_EDGE_PX`].

use std::path::Path;
use std::sync::Mutex;

use audio_core::{
    compute_full_track_envelope, compute_overview_envelope, LoadableAudio, WaveformAnalysisConfig,
};
use sea_orm::EntityTrait;

use library_core::{
    ExportArtwork, ExportBeatGrid, ExportCue, ExportSource, FileAudioSource, Library, LibraryError,
    Result, TrackExportData, TrackId, EXPORT_ARTWORK_EDGE_PX, EXPORT_WAVEFORM_ENTRIES_PER_SECOND,
};

use crate::db;
use crate::entity::TrackEntity;
use crate::LibraryManager;

impl ExportSource for LibraryManager {
    /// Collect everything an exporter needs for one track.
    ///
    /// Returns `Ok(None)` for stream tracks and for tracks whose file is
    /// missing; exporters report those themselves and fall back to file tags.
    /// The detail waveform tier is computed here (the library stores a
    /// resolution meant for its own displays), which decodes the file once.
    fn export_data(&self, id: &TrackId) -> Result<Option<TrackExportData>> {
        let Some(source) = self.get_track(id)? else {
            return Ok(None);
        };
        let Some(file) = source.file() else {
            return Ok(None);
        };
        let path = file.path();
        if !path.is_file() {
            return Ok(None);
        }

        let mut data = TrackExportData {
            bpm: file.metadata.bpm,
            added_at: self.track_added_at(id)?,
            bit_depth: source_bit_depth(path),
            artwork: export_artwork(path),
            cues: self.cues_for_export(id)?,
            ..TrackExportData::default()
        };

        if let Some(grid) = self.get_track_beat_grid(id)? {
            if data.bpm.is_none() {
                data.bpm = grid.bpm;
            }
            data.beat_grid = Some(ExportBeatGrid {
                beats: grid.beats,
                beats_per_bar: grid.beats_per_bar.max(1),
            });
        }

        if let Some(overview) = self.get_track_waveform_overview(id)? {
            data.waveform_overview = overview.peaks;
        }

        let audio =
            FileAudioSource::from_path(path)
                .load()
                .map_err(|err| LibraryError::Backend {
                    backend: "export",
                    message: format!("{}: {err}", crate::path_label(path)),
                })?;
        let config = WaveformAnalysisConfig::default();
        if data.waveform_overview.is_empty() {
            data.waveform_overview = compute_overview_envelope(&audio, &config);
        }
        data.waveform_detail = compute_full_track_envelope(&audio, detail_buckets(&audio), &config);
        if data.bpm.is_none() {
            data.bpm = self.beat_grid_bpm(id)?;
        }
        Ok(Some(data))
    }
}

impl LibraryManager {
    /// BPM recorded by analysis, when a grid exists but carried no tempo.
    fn beat_grid_bpm(&self, id: &TrackId) -> Result<Option<f64>> {
        Ok(self
            .get_track_beat_grid(id)?
            .and_then(|grid| grid.bpm)
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0))
    }

    /// Date the track entered the library, as `YYYY-MM-DD`.
    fn track_added_at(&self, id: &TrackId) -> Result<Option<String>> {
        let row = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(row.added_at.parse::<i64>().ok().and_then(|secs| {
            chrono::DateTime::from_timestamp(secs, 0)
                .map(|instant| instant.date_naive().format("%Y-%m-%d").to_string())
        }))
    }

    /// Hot cues, then saved loops, in the shape device formats store them.
    ///
    /// Beat-quantized hot loops get an explicit loop end when a BPM is known,
    /// so targets that cannot express beat lengths still loop correctly.
    fn cues_for_export(&self, id: &TrackId) -> Result<Vec<ExportCue>> {
        let bpm = self
            .get_track_beat_grid(id)?
            .and_then(|grid| grid.bpm)
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0);

        let mut cues = Vec::new();
        for cue in self.list_track_hot_cues(id)? {
            let position_ms = cue.position_ms.max(0) as u32;
            let loop_length_beats = cue.loop_length_beats.filter(|beats| *beats > 0);
            let loop_end_ms = match (loop_length_beats, bpm) {
                (Some(beats), Some(bpm)) => {
                    let beat_ms = 60_000.0 / bpm;
                    Some(position_ms + (beat_ms * f64::from(beats)).round() as u32)
                }
                _ => None,
            };
            cues.push(ExportCue {
                hot_cue_slot: Some(cue.slot_index),
                position_ms,
                loop_end_ms,
                loop_length_beats: loop_length_beats.map(|beats| beats as u32),
                label: non_empty(cue.label),
                color: non_empty(cue.color),
            });
        }
        for saved in self.list_track_loops(id)? {
            cues.push(ExportCue {
                hot_cue_slot: None,
                position_ms: saved.in_ms.max(0) as u32,
                loop_end_ms: Some(saved.out_ms.max(0) as u32),
                loop_length_beats: None,
                label: non_empty(saved.label),
                color: non_empty(saved.color),
            });
        }
        Ok(cues)
    }
}

/// Export source over a manager that lives behind its host lock.
///
/// Format adapters are handed `&dyn ExportSource`; the host owns the manager
/// inside an `Arc<Mutex<…>>`. This wrapper implements both halves of the
/// contract by locking the manager per call, so an export does not monopolize
/// the library for its whole run — each call holds the lock only for its own
/// database reads, file probing and decode.
pub struct LockedExportSource<'a> {
    manager: &'a Mutex<LibraryManager>,
}

impl<'a> LockedExportSource<'a> {
    /// Borrow a host-held manager as an export source.
    pub fn new(manager: &'a Mutex<LibraryManager>) -> Self {
        Self { manager }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, LibraryManager>> {
        self.manager.lock().map_err(|_| LibraryError::Backend {
            backend: "export",
            message: "library lock poisoned".to_string(),
        })
    }
}

impl Library for LockedExportSource<'_> {
    fn name(&self) -> &'static str {
        "library"
    }

    fn get_track(&self, id: &TrackId) -> Result<Option<library_core::AudioSource>> {
        self.lock()?.get_track(id)
    }

    fn list_collections(&self) -> Result<Vec<library_core::Collection>> {
        self.lock()?.list_collections()
    }

    fn get_collection(
        &self,
        id: &library_core::CollectionId,
    ) -> Result<Option<library_core::Collection>> {
        self.lock()?.get_collection(id)
    }

    fn list_collection_tracks(
        &self,
        collection_id: &library_core::CollectionId,
    ) -> Result<Vec<library_core::AudioSource>> {
        self.lock()?.list_collection_tracks(collection_id)
    }

    fn list_collection_entries(
        &self,
        collection_id: &library_core::CollectionId,
    ) -> Result<Vec<library_core::CollectionEntry>> {
        self.lock()?.list_collection_entries(collection_id)
    }
}

impl ExportSource for LockedExportSource<'_> {
    fn export_data(&self, id: &TrackId) -> Result<Option<TrackExportData>> {
        self.lock()?.export_data(id)
    }
}

/// Keep only non-blank strings.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

/// Detail-tier bucket count for a decoded track: 150 entries per second.
fn detail_buckets(audio: &audio_core::LoadedAudio) -> usize {
    let channels = usize::from(audio.channels.max(1));
    let frames = audio.samples.len() / channels;
    if frames == 0 {
        return 0;
    }
    let seconds = frames as f64 / f64::from(audio.sample_rate.max(1));
    (seconds * f64::from(EXPORT_WAVEFORM_ENTRIES_PER_SECOND)).round() as usize
}

/// Source bit depth from the file's own properties, when the container reports
/// it (the decoder's output depth is always `f32` and says nothing).
fn source_bit_depth(path: &Path) -> Option<u16> {
    codec::read_file_tags(path)
        .ok()?
        .bit_depth
        .map(u16::from)
        .filter(|depth| *depth > 0)
}

/// Cover thumbnail an exporter may write, at [`EXPORT_ARTWORK_EDGE_PX`].
///
/// Best-effort: a file without artwork, with a cover format we cannot decode, or
/// with a tag we cannot read yields `None` — losing a cover must not fail the
/// track's export.
fn export_artwork(path: &Path) -> Option<ExportArtwork> {
    codec::read_file_artwork_thumbnail(path, EXPORT_ARTWORK_EDGE_PX)
        .ok()
        .flatten()
        .map(|jpeg| ExportArtwork { jpeg })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_buckets_follow_the_export_rate() {
        let audio = audio_core::LoadedAudio {
            samples: vec![0.0; 48_000 * 2 * 3],
            sample_rate: 48_000,
            channels: 2,
            source_id: "test".into(),
        };
        assert_eq!(detail_buckets(&audio), 450);
    }

    #[test]
    fn empty_audio_asks_for_no_buckets() {
        let audio = audio_core::LoadedAudio {
            samples: Vec::new(),
            sample_rate: 44_100,
            channels: 2,
            source_id: "test".into(),
        };
        assert_eq!(detail_buckets(&audio), 0);
    }

    #[test]
    fn blank_labels_are_dropped() {
        assert_eq!(non_empty(Some("  ".to_string())), None);
        assert_eq!(non_empty(None), None);
        assert_eq!(
            non_empty(Some("Intro".to_string())),
            Some("Intro".to_string())
        );
    }
}
