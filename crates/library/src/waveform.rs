//! Persist and load track waveform overviews (L0) in the library database.

use std::path::Path;

use audio_core::{
    compute_overview_envelope, peaks_to_rgb_bytes, rgb_bytes_to_peaks, LoadableAudio, SpectralPeak,
    WaveformAmplitudeMode, WaveformAnalysisConfig, WaveformChannelMode, OVERVIEW_SAMPLE_COUNT,
    WAVEFORM_SCHEMA_VERSION,
};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

use library_core::{FileAudioSource, LibraryError, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{
    track_analysis, track_waveform, tracks, TrackAnalysisEntity, TrackEntity, TrackWaveformEntity,
};

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

#[derive(Debug, Clone)]
pub struct BeatGridSnapshot {
    pub beats: Vec<f32>,
    pub bars: Vec<f32>,
    pub downbeats: Vec<f32>,
    /// Analyzed BPM when available (for even visual grids).
    pub bpm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BeatGridJson {
    beats: Vec<f32>,
    bars: Vec<f32>,
    downbeats: Vec<f32>,
}

/// ponytail: constant-tempo grid only. Manual edits assume even spacing; variable-tempo
/// tracks keep stored beat times until a future editor maps phase/BPM onto beat indices.
pub(crate) fn generate_even_beat_grid(
    bpm: f64,
    first_beat_secs: f32,
    duration_secs: f32,
) -> BeatGridJson {
    let beat_period = (60.0 / bpm) as f32;
    let mut beats = Vec::new();
    let mut bars = Vec::new();
    let mut downbeats = Vec::new();
    let mut beat_index: i32 = -1;
    loop {
        beat_index += 1;
        let t = first_beat_secs + beat_index as f32 * beat_period;
        if t > duration_secs {
            break;
        }
        if t >= 0.0 {
            beats.push(t);
            if beat_index.rem_euclid(4) == 0 {
                downbeats.push(t);
                bars.push(t);
            }
        }
    }
    BeatGridJson {
        beats,
        bars,
        downbeats,
    }
}

pub(crate) fn save_track_beat_grid(
    db: &Db,
    track_id: &TrackId,
    bpm: f64,
    first_beat_secs: f32,
) -> Result<BeatGridSnapshot> {
    if !(bpm > 20.0 && bpm < 400.0) {
        return Err(LibraryError::Backend {
            backend: "library",
            message: "beat grid bpm must be between 20 and 400".into(),
        });
    }

    let track = TrackEntity::find_by_id(track_id.as_str())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?
        .ok_or_else(|| LibraryError::Backend {
            backend: "library",
            message: format!("track not found: {}", track_id.as_str()),
        })?;

    let duration_secs = track.duration_ms.unwrap_or(0).max(0) as f32 / 1000.0;
    let duration_secs = if duration_secs > 0.0 {
        duration_secs
    } else {
        600.0
    };

    let grid = generate_even_beat_grid(bpm, first_beat_secs, duration_secs);
    let beat_grid_json = serde_json::to_string(&grid).map_err(|e| LibraryError::Backend {
        backend: "library",
        message: e.to_string(),
    })?;

    let analyzed_at = now_iso();
    let active = track_analysis::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        backend: Set("manual".into()),
        backend_version: Set("1".into()),
        analyzed_at: Set(analyzed_at),
        bpm: Set(Some(bpm)),
        bpm_confidence: Set(None),
        key: Set(track.key.clone()),
        key_confidence: Set(None),
        key_clarity: Set(None),
        grid_stability: Set(Some(1.0)),
        sample_rate: Set(track.sample_rate.unwrap_or(48_000)),
        duration_analyzed_ms: Set(track.duration_ms.unwrap_or(0)),
        loudness_lufs: Set(None),
        beat_grid_json: Set(Some(beat_grid_json)),
    };

    TrackAnalysisEntity::insert(active)
        .on_conflict(
            OnConflict::column(track_analysis::Column::TrackId)
                .update_columns([
                    track_analysis::Column::Backend,
                    track_analysis::Column::BackendVersion,
                    track_analysis::Column::AnalyzedAt,
                    track_analysis::Column::Bpm,
                    track_analysis::Column::GridStability,
                    track_analysis::Column::BeatGridJson,
                ])
                .to_owned(),
        )
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    TrackEntity::update_many()
        .col_expr(tracks::Column::Bpm, sea_orm::sea_query::Expr::value(bpm))
        .filter(tracks::Column::Id.eq(track_id.as_str()))
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;

    Ok(BeatGridSnapshot {
        beats: grid.beats,
        bars: grid.bars,
        downbeats: grid.downbeats,
        bpm: Some(bpm),
    })
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

pub(crate) fn store_overview(db: &Db, track_id: &TrackId, peaks: &[SpectralPeak]) -> Result<()> {
    debug_assert_eq!(peaks.len(), OVERVIEW_SAMPLE_COUNT);
    upsert_track_waveform(db, track_id, peaks, &waveform_config())
}

pub(crate) fn generate_and_store_overview(db: &Db, track_id: &TrackId, path: &Path) -> Result<()> {
    let peaks = generate_overview_from_path(path)?;
    store_overview(db, track_id, &peaks)
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
    fn generate_even_beat_grid_produces_downbeats_every_four_beats() {
        let grid = generate_even_beat_grid(120.0, 0.0, 4.0);
        assert_eq!(grid.downbeats, vec![0.0, 2.0, 4.0]);
        assert_eq!(grid.beats.len(), 9);
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

        let saved = save_track_beat_grid(&db, &id, 130.0, 0.25).unwrap();
        assert_eq!(saved.bpm, Some(130.0));
        assert_eq!(saved.beats.first(), Some(&0.25));

        let loaded = get_track_beat_grid(&db, &id).unwrap().unwrap();
        assert_eq!(loaded.bpm, Some(130.0));
        assert_eq!(loaded.beats.first(), Some(&0.25));

        let track = TrackEntity::find_by_id(id.as_str())
            .one(db.conn().unwrap().as_connection())
            .unwrap()
            .unwrap();
        assert_eq!(track.bpm, Some(130.0));
    }
}
