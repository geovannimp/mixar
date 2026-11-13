//! Persist and load track waveform overviews (L0) in the library database.

use std::path::Path;
use std::sync::{Arc, Mutex};

use audio_core::{
    compute_overview_envelope, peaks_to_rgb_bytes, rgb_bytes_to_peaks, LoadableAudio, SpectralPeak,
    WaveformAmplitudeMode, WaveformAnalysisConfig, WaveformChannelMode, OVERVIEW_SAMPLE_COUNT,
    WAVEFORM_SCHEMA_VERSION,
};
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Set};

use library_core::{FileAudioSource, Library, LibraryError, Result, TrackId};

use crate::beat_grid::{self, CanonicalBeatGrid, SOURCE_MANUAL};
use crate::db::{self, Db};
use crate::entity::{track_waveform, TrackWaveformEntity};
use crate::LibraryManager;

#[derive(Debug, Clone)]
pub struct TrackWaveformOverview {
    pub peaks: Vec<SpectralPeak>,
    pub amplitude_mode: WaveformAmplitudeMode,
    pub channel_mode: WaveformChannelMode,
    pub overview_count: usize,
}

fn waveform_config() -> WaveformAnalysisConfig {
    WaveformAnalysisConfig::default()
}

fn amplitude_mode_str(mode: WaveformAmplitudeMode) -> &'static str {
    match mode {
        WaveformAmplitudeMode::Peak => "peak",
        WaveformAmplitudeMode::Rms => "rms",
    }
}

fn channel_mode_str(mode: WaveformChannelMode) -> &'static str {
    match mode {
        WaveformChannelMode::Mono => "mono",
        WaveformChannelMode::Stereo => "stereo",
    }
}

fn parse_amplitude_mode(value: &str) -> WaveformAmplitudeMode {
    match value {
        "rms" => WaveformAmplitudeMode::Rms,
        _ => WaveformAmplitudeMode::Peak,
    }
}

fn parse_channel_mode(value: &str) -> WaveformChannelMode {
    match value {
        "stereo" => WaveformChannelMode::Stereo,
        _ => WaveformChannelMode::Mono,
    }
}

pub(crate) fn zstd_compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    zstd::stream::copy_encode(data, &mut out, 3).map_err(|e| LibraryError::Backend {
        backend: "waveform",
        message: e.to_string(),
    })?;
    Ok(out)
}

pub(crate) fn zstd_decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    zstd::stream::copy_decode(data, &mut out).map_err(|e| LibraryError::Backend {
        backend: "waveform",
        message: e.to_string(),
    })?;
    Ok(out)
}

pub fn generate_overview_from_path(path: &Path) -> Result<Vec<SpectralPeak>> {
    let audio = FileAudioSource::from_path(path)
        .load()
        .map_err(|e| LibraryError::Backend {
            backend: "waveform",
            message: e.to_string(),
        })?;
    Ok(compute_overview_envelope(&audio, &waveform_config()))
}

pub(crate) fn upsert_track_waveform(
    db: &Db,
    track_id: &TrackId,
    peaks: &[SpectralPeak],
    config: &WaveformAnalysisConfig,
) -> Result<()> {
    let raw = peaks_to_rgb_bytes(peaks, config.channel_mode);
    let compressed = zstd_compress(&raw)?;

    let active = track_waveform::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        version: Set(WAVEFORM_SCHEMA_VERSION as i32),
        amplitude_mode: Set(amplitude_mode_str(config.amplitude_mode).to_string()),
        channel_mode: Set(channel_mode_str(config.channel_mode).to_string()),
        overview_count: Set(peaks.len() as i32),
        overview_bytes: Set(compressed),
        generated_at: Set(crate::unix_secs_string()),
    };

    TrackWaveformEntity::insert(active)
        .on_conflict(
            OnConflict::column(track_waveform::Column::TrackId)
                .update_columns([
                    track_waveform::Column::Version,
                    track_waveform::Column::AmplitudeMode,
                    track_waveform::Column::ChannelMode,
                    track_waveform::Column::OverviewCount,
                    track_waveform::Column::OverviewBytes,
                    track_waveform::Column::GeneratedAt,
                ])
                .to_owned(),
        )
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

pub(crate) fn get_track_waveform_row(
    db: &Db,
    track_id: &TrackId,
) -> Result<Option<TrackWaveformOverview>> {
    let row = TrackWaveformEntity::find_by_id(track_id.as_str())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    let Some(row) = row else {
        return Ok(None);
    };
    if row.version != WAVEFORM_SCHEMA_VERSION as i32 {
        return Ok(None);
    }

    let channel_mode = parse_channel_mode(&row.channel_mode);
    let raw = zstd_decompress(&row.overview_bytes)?;
    let peaks =
        rgb_bytes_to_peaks(&raw, row.overview_count as usize, channel_mode).ok_or_else(|| {
            LibraryError::Backend {
                backend: "waveform",
                message: format!(
                    "corrupt waveform blob for track {} (count={})",
                    row.track_id, row.overview_count
                ),
            }
        })?;

    Ok(Some(TrackWaveformOverview {
        peaks,
        amplitude_mode: parse_amplitude_mode(&row.amplitude_mode),
        channel_mode,
        overview_count: row.overview_count as usize,
    }))
}

pub(crate) fn has_track_waveform(db: &Db, track_id: &TrackId) -> Result<bool> {
    let row = TrackWaveformEntity::find_by_id(track_id.as_str())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(row.is_some_and(|r| r.version == WAVEFORM_SCHEMA_VERSION as i32))
}

/// Total compressed overview blob bytes stored in `track_waveform`.
pub fn waveform_cache_bytes(db: &Db) -> Result<u64> {
    use sea_orm::{ConnectionTrait, Statement};

    let total = db
        .conn()?
        .as_connection()
        .query_one_raw(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT COALESCE(SUM(LENGTH(overview_bytes)), 0) AS total FROM track_waveform"
                .to_string(),
        ))
        .map_err(db::db_err)?
        .and_then(|row| row.try_get::<i64>("", "total").ok())
        .unwrap_or(0);
    Ok(total.max(0) as u64)
}

/// Delete all waveform overview rows (regenerated on next fetch).
pub fn clear_all_track_waveforms(db: &Db) -> Result<()> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    TrackWaveformEntity::delete_many()
        .filter(track_waveform::Column::TrackId.is_not_null())
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

pub(crate) fn save_track_beat_grid(
    db: &Db,
    track_id: &TrackId,
    bpm: f64,
    first_beat_secs: f32,
    replace: bool,
) -> Result<CanonicalBeatGrid> {
    if !(bpm > 20.0 && bpm < 400.0) {
        return Err(LibraryError::Backend {
            backend: "library",
            message: "beat grid bpm must be between 20 and 400".into(),
        });
    }
    if !first_beat_secs.is_finite() {
        return Err(LibraryError::Backend {
            backend: "library",
            message: "beat grid first_beat_secs must be finite".into(),
        });
    }

    let existing = beat_grid::get_track_beat_grid(db, track_id)?;
    if existing
        .as_ref()
        .is_some_and(|grid| grid.segments.len() > 1)
        && !replace
    {
        return Err(LibraryError::Backend {
            backend: "library",
            message: "refusing to replace a segmented beat grid without replace=true".into(),
        });
    }

    let first_segment = existing.as_ref().and_then(|grid| grid.first_segment());
    let beats_per_bar = first_segment.map_or(4, |segment| segment.beats_per_bar);
    let beat_unit = first_segment.map_or(4, |segment| segment.beat_unit);
    let first_beat_ms = (f64::from(first_beat_secs) * 1000.0).round() as i64;
    let grid = beat_grid::from_constant(
        bpm,
        first_beat_ms,
        beats_per_bar,
        beat_unit,
        first_beat_ms,
        None,
        SOURCE_MANUAL,
    );
    beat_grid::write_track_beat_grid(db, track_id, &grid)?;
    Ok(grid)
}

pub(crate) fn get_track_beat_grid(
    db: &Db,
    track_id: &TrackId,
) -> Result<Option<CanonicalBeatGrid>> {
    beat_grid::get_track_beat_grid(db, track_id)
}

pub(crate) fn store_overview(db: &Db, track_id: &TrackId, peaks: &[SpectralPeak]) -> Result<()> {
    debug_assert_eq!(peaks.len(), OVERVIEW_SAMPLE_COUNT);
    upsert_track_waveform(db, track_id, peaks, &waveform_config())
}

pub(crate) fn generate_and_store_overview(db: &Db, track_id: &TrackId, path: &Path) -> Result<()> {
    let peaks = generate_overview_from_path(path)?;
    store_overview(db, track_id, &peaks)
}

impl LibraryManager {
    /// Load the stored L0 waveform overview for a track, if present.
    pub fn get_track_waveform_overview(
        &self,
        id: &TrackId,
    ) -> Result<Option<TrackWaveformOverview>> {
        get_track_waveform_row(&self.db, id)
    }
}

impl LibraryManager {
    /// Delete all cached waveform overview rows and drop decoded PCM from RAM.
    pub fn clear_waveform_cache(&self) -> Result<()> {
        clear_all_track_waveforms(&self.db)?;
        self.clear_decode_cache();
        Ok(())
    }

    /// Drop decoded PCM even if a prior panic poisoned the cache mutex.
    pub(crate) fn clear_decode_cache(&self) {
        self.decode_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Compressed waveform overview bytes stored in the library DB.
    pub fn waveform_cache_bytes(&self) -> Result<u64> {
        waveform_cache_bytes(&self.db)
    }

    /// Generate and persist the overview when missing (e.g. first waveform fetch).
    ///
    /// Takes `&Mutex<Self>` so overview generation does not hold the library lock.
    pub fn ensure_track_waveform(library: &Mutex<Self>, id: &TrackId) -> Result<()> {
        let path = {
            let lib = Self::lock_library(library)?;
            if has_track_waveform(&lib.db, id)? {
                return Ok(());
            }
            lib.get_track(id)?
                .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
                .file()
                .ok_or(LibraryError::Unsupported("stream tracks have no waveform"))?
                .path()
                .to_path_buf()
        };

        let peaks = generate_overview_from_path(&path)?;

        let lib = Self::lock_library(library)?;
        if has_track_waveform(&lib.db, id)? {
            return Ok(());
        }
        store_overview(&lib.db, id, &peaks)
    }

    /// Beat grid overlay data when the track has been analyzed.
    pub fn get_track_beat_grid(&self, id: &TrackId) -> Result<Option<CanonicalBeatGrid>> {
        get_track_beat_grid(&self.db, id)
    }

    pub fn save_track_beat_grid(
        &self,
        id: &TrackId,
        bpm: f64,
        first_beat_secs: f32,
        replace: bool,
    ) -> Result<CanonicalBeatGrid> {
        save_track_beat_grid(&self.db, id, bpm, first_beat_secs, replace)
    }

    /// Hi-res spectral peaks for a time window (L1). Uses the decode cache when present.
    ///
    /// Takes `&Mutex<Self>` so decode / analysis does not hold the library lock.
    pub fn compute_waveform_window(
        library: &Mutex<Self>,
        id: &TrackId,
        start_ms: i32,
        end_ms: i32,
        buckets: usize,
    ) -> Result<(Vec<audio_core::SpectralPeak>, i32, i32)> {
        // shortcut: caps L1 resolution to bound RPC allocation. Upgrade to
        // chunked peak generation if a larger waveform window is required.
        if buckets == 0 || buckets > audio_core::MAX_WAVEFORM_BUCKETS {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "waveform bucket count is out of range".into(),
            });
        }
        // Double-checked cache: decode off the library lock so UI/DB work can
        // proceed. A racing miss may decode twice; re-get before insert drops the
        // duplicate Arc. Upgrade to single-flight if wasted decode becomes costly.
        let cached = {
            let lib = Self::lock_library(library)?;
            let mut cache = Self::lock_decode_cache(&lib.decode_cache)?;
            cache.get(id)
        };

        let audio = if let Some(cached) = cached {
            cached
        } else {
            let source = {
                let lib = Self::lock_library(library)?;
                lib.get_track(id)?
                    .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
            };
            if source.file().is_none() {
                return Err(LibraryError::Unsupported("stream tracks have no waveform"));
            }
            let loaded = Arc::new(source.load().map_err(|e| LibraryError::Backend {
                backend: "library",
                message: format!("failed to decode track for waveform: {e}"),
            })?);
            let lib = Self::lock_library(library)?;
            let mut cache = Self::lock_decode_cache(&lib.decode_cache)?;
            if let Some(existing) = cache.get(id) {
                existing
            } else {
                cache.insert(id.clone(), loaded)
            }
        };

        let duration_ms = {
            let channels = usize::from(audio.channels.max(1));
            let frames = audio.samples.len() / channels;
            if audio.sample_rate == 0 || frames == 0 {
                0
            } else {
                audio_core::secs_to_ms(frames as f64 / f64::from(audio.sample_rate))
            }
        };
        let start_ms = start_ms.max(0).min(duration_ms.max(0));
        let end_ms = end_ms.max(start_ms).min(duration_ms.max(0));
        let peaks = audio_core::compute_spectral_window(
            audio.as_ref(),
            audio_core::ms_to_secs(start_ms),
            audio_core::ms_to_secs(end_ms),
            buckets,
            &audio_core::WaveformAnalysisConfig::default(),
        );
        Ok((peaks, start_ms, end_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::store::Store;
    use library_core::TrackMetadata;
    use std::path::Path;

    #[test]
    fn zstd_round_trip() {
        let data: Vec<u8> = (0..=255).collect();
        let compressed = zstd_compress(&data).unwrap();
        let decoded = zstd_decompress(&compressed).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn save_track_beat_grid_rejects_non_finite_first_beat() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/grid.wav");
        store
            .upsert_file_track(
                &id,
                Path::new("/music/grid.wav"),
                &TrackMetadata {
                    bpm: Some(128.0),
                    duration_ms: Some(60_000),
                    ..TrackMetadata::default()
                },
                "1",
            )
            .unwrap();
        assert!(save_track_beat_grid(&db, &id, 128.0, f32::NAN, false).is_err());
        assert!(save_track_beat_grid(&db, &id, 128.0, f32::INFINITY, false).is_err());
    }

    #[test]
    fn save_track_beat_grid_persists_and_updates_track_bpm() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/grid.wav");
        store
            .upsert_file_track(
                &id,
                Path::new("/music/grid.wav"),
                &TrackMetadata {
                    bpm: Some(128.0),
                    duration_ms: Some(60_000),
                    ..TrackMetadata::default()
                },
                "1",
            )
            .unwrap();

        let saved = save_track_beat_grid(&db, &id, 130.0, 0.25, false).unwrap();
        assert_eq!(saved.bpm(), Some(130.0));
        assert_eq!(saved.first_beat_ms(), Some(250));

        let loaded = get_track_beat_grid(&db, &id).unwrap().unwrap();
        assert_eq!(loaded.bpm(), Some(130.0));
        assert_eq!(loaded.first_beat_ms(), Some(250));

        let track = store.get_track(&id).unwrap().unwrap();
        assert_eq!(track.metadata().bpm, Some(130.0));
    }

    #[test]
    fn save_track_beat_grid_requires_replace_for_a_segmented_grid() {
        use crate::beat_grid::{
            write_track_beat_grid, BeatSegment, CanonicalBeatGrid, SOURCE_ANALYSIS, SOURCE_MANUAL,
        };

        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/segmented.wav");
        store
            .upsert_file_track(
                &id,
                Path::new("/music/segmented.wav"),
                &TrackMetadata {
                    bpm: Some(120.0),
                    duration_ms: Some(60_000),
                    ..TrackMetadata::default()
                },
                "1",
            )
            .unwrap();
        let grid = CanonicalBeatGrid {
            source: SOURCE_ANALYSIS.into(),
            covered_to_ms: None,
            segments: vec![
                BeatSegment {
                    segment_index: 0,
                    start_time_ms: 0,
                    bpm: 120.0,
                    beats_per_bar: 4,
                    beat_unit: 4,
                    start_beat_index: 0,
                    beats_to_next: Some(16),
                    downbeat_offset_beats: 0,
                },
                BeatSegment {
                    segment_index: 1,
                    start_time_ms: 8_000,
                    bpm: 140.0,
                    beats_per_bar: 4,
                    beat_unit: 4,
                    start_beat_index: 16,
                    beats_to_next: None,
                    downbeat_offset_beats: 0,
                },
            ],
        };
        write_track_beat_grid(&db, &id, &grid).unwrap();

        assert!(save_track_beat_grid(&db, &id, 130.0, 0.25, false).is_err());
        save_track_beat_grid(&db, &id, 130.0, 0.25, true).unwrap();

        let canonical = crate::beat_grid::get_track_beat_grid(&db, &id)
            .unwrap()
            .unwrap();
        assert_eq!(canonical.source, SOURCE_MANUAL);
        assert_eq!(canonical.segments.len(), 1);
        assert_eq!(canonical.bpm(), Some(130.0));
    }
}
