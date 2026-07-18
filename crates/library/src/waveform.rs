//! Persist and load track waveform overviews (L0) in the library database.

use std::path::Path;

use audio_core::{
    compute_overview_envelope, peaks_to_rgb_bytes, rgb_bytes_to_peaks, LoadableAudio, SpectralPeak,
    WaveformAmplitudeMode, WaveformAnalysisConfig, WaveformChannelMode, OVERVIEW_SAMPLE_COUNT,
    WAVEFORM_SCHEMA_VERSION,
};
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, PaginatorTrait, Set};
use serde::Deserialize;

use library_core::{FileAudioSource, LibraryError, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{track_waveform, TrackAnalysisEntity, TrackWaveformEntity};

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

fn now_iso() -> String {
    // RFC3339 without pulling in chrono — good enough for generated_at metadata.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
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
        generated_at: Set(now_iso()),
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
    let count = TrackWaveformEntity::find_by_id(track_id.as_str())
        .count(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(count > 0)
}

#[derive(Debug, Clone)]
pub struct BeatGridSnapshot {
    pub beats: Vec<f32>,
    pub bars: Vec<f32>,
    pub downbeats: Vec<f32>,
    /// Analyzed BPM when available (for even visual grids).
    pub bpm: Option<f64>,
}

#[derive(Deserialize)]
struct BeatGridJson {
    beats: Vec<f32>,
    bars: Vec<f32>,
    downbeats: Vec<f32>,
}

pub(crate) fn get_track_beat_grid(db: &Db, track_id: &TrackId) -> Result<Option<BeatGridSnapshot>> {
    let row = TrackAnalysisEntity::find_by_id(track_id.as_str())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    let Some(row) = row else {
        return Ok(None);
    };

    let Some(json) = row.beat_grid_json else {
        return Ok(None);
    };

    let grid: BeatGridJson = serde_json::from_str(&json).map_err(|e| LibraryError::Backend {
        backend: "library",
        message: e.to_string(),
    })?;

    Ok(Some(BeatGridSnapshot {
        beats: grid.beats,
        bars: grid.bars,
        downbeats: grid.downbeats,
        bpm: row.bpm,
    }))
}

pub(crate) fn generate_and_store_overview(db: &Db, track_id: &TrackId, path: &Path) -> Result<()> {
    let peaks = generate_overview_from_path(path)?;
    debug_assert_eq!(peaks.len(), OVERVIEW_SAMPLE_COUNT);
    upsert_track_waveform(db, track_id, &peaks, &waveform_config())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zstd_round_trip() {
        let data: Vec<u8> = (0..=255).collect();
        let compressed = zstd_compress(&data).unwrap();
        let decoded = zstd_decompress(&compressed).unwrap();
        assert_eq!(data, decoded);
    }
}
