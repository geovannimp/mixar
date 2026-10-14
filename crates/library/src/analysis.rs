//! Persist offline analysis results in the library database.

use analyzer::{
    analyze_file, analyze_pcm, merge_track_metadata, AnalysisConfig, AnalyzerError, TagMetadata,
    TrackAnalysis,
};
use analyzer_core::loudness_lufs_from_replaygain_track_gain_db;
use codec::{decode_stem_file, is_stem_path};
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Set};
use std::path::Path;

use library_core::{AnalyzeTrackOptions, LibraryError, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{track_analysis, TrackAnalysisEntity};
use crate::{is_audio_file, normalize_path, path_label, tags};

pub(crate) fn upsert_track_analysis(
    db: &Db,
    track_id: &TrackId,
    analysis: &TrackAnalysis,
) -> Result<()> {
    let beat_grid_json = analysis
        .beat_grid
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| db::db_err(sea_orm::DbErr::Custom(e.to_string())))?;

    let active = track_analysis::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        backend: Set(analysis.metadata.backend.clone()),
        backend_version: Set(analysis.metadata.backend_version.clone()),
        analyzed_at: Set(analysis.metadata.analyzed_at.clone()),
        bpm: Set(analysis.bpm.as_ref().map(|b| b.bpm)),
        bpm_confidence: Set(analysis.bpm.as_ref().map(|b| f64::from(b.confidence))),
        key: Set(analysis.key.as_ref().map(|k| k.musical.clone())),
        key_confidence: Set(analysis.key.as_ref().map(|k| f64::from(k.confidence))),
        key_clarity: Set(analysis.key.as_ref().map(|k| f64::from(k.clarity))),
        grid_stability: Set(analysis
            .beat_grid
            .as_ref()
            .map(|g| f64::from(g.grid_stability))),
        sample_rate: Set(analysis.metadata.sample_rate as i32),
        duration_analyzed_ms: Set(analysis.metadata.duration_analyzed_ms),
        loudness_lufs: Set(analysis.loudness_lufs),
        beat_grid_json: Set(beat_grid_json),
    };

    TrackAnalysisEntity::insert(active)
        .on_conflict(
            OnConflict::column(track_analysis::Column::TrackId)
                .update_columns([
                    track_analysis::Column::Backend,
                    track_analysis::Column::BackendVersion,
                    track_analysis::Column::AnalyzedAt,
                    track_analysis::Column::Bpm,
                    track_analysis::Column::BpmConfidence,
                    track_analysis::Column::Key,
                    track_analysis::Column::KeyConfidence,
                    track_analysis::Column::KeyClarity,
                    track_analysis::Column::GridStability,
                    track_analysis::Column::SampleRate,
                    track_analysis::Column::DurationAnalyzedMs,
                    track_analysis::Column::LoudnessLufs,
                    track_analysis::Column::BeatGridJson,
                ])
                .to_owned(),
        )
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

pub(crate) fn analyzer_error(err: AnalyzerError) -> LibraryError {
    match err {
        AnalyzerError::Decode(message) => LibraryError::Backend {
            backend: "analyzer",
            message,
        },
        AnalyzerError::Analysis(message) => LibraryError::Backend {
            backend: "analyzer",
            message,
        },
        AnalyzerError::Unsupported(message) => LibraryError::Unsupported(message),
        AnalyzerError::Backend { backend, message } => LibraryError::Backend { backend, message },
    }
}

#[cfg(feature = "analysis")]
pub(crate) struct ComputedFileAnalysis {
    pub(crate) metadata: library_core::TrackMetadata,
    pub(crate) analysis: analyzer::TrackAnalysis,
}

#[cfg(feature = "analysis")]
pub(crate) fn compute_file_analysis(
    path: &Path,
    options: AnalyzeTrackOptions,
) -> Result<ComputedFileAnalysis> {
    let path = normalize_path(path)?;
    if !path.is_file() {
        return Err(LibraryError::PathNotFound(path_label(&path)));
    }
    if !is_audio_file(&path) {
        return Err(LibraryError::UnsupportedFile(path_label(&path)));
    }

    let mut config = AnalysisConfig::default();
    let tag_metadata = tags::read_tags(&path)?;
    config.max_duration_ms = options
        .analysis_duration
        .resolve_max_duration_ms(tag_metadata.duration_ms);
    let mut analysis = if is_stem_path(&path) {
        analyze_stem_mixdown(&path, &config)?
    } else {
        analyze_file(&path, &config).map_err(analyzer_error)?
    };
    let replaygain_track_gain_db = tags::read_replaygain_track_gain_db(&path)?;
    analysis.loudness_lufs =
        preferred_loudness_lufs(analysis.loudness_lufs, replaygain_track_gain_db);

    let tag_side = TagMetadata {
        bpm: tag_metadata.bpm,
        key: tag_metadata.key.clone(),
    };
    let merged = merge_track_metadata(
        &tag_side,
        &analysis,
        options.force,
        config.min_bpm_confidence,
        config.min_key_confidence,
    );

    let mut metadata = tag_metadata;
    metadata.bpm = merged.bpm;
    metadata.key = merged.key;

    Ok(ComputedFileAnalysis { metadata, analysis })
}

/// Decode a `.stem.mp4` mixdown via the repaired Stem path, then analyze mono PCM.
#[cfg(feature = "analysis")]
pub(crate) fn analyze_stem_mixdown(
    path: &Path,
    config: &AnalysisConfig,
) -> Result<analyzer::TrackAnalysis> {
    let bundle = decode_stem_file(path).map_err(|e| LibraryError::Backend {
        backend: "codec",
        message: format!("failed to decode stem mixdown {}: {e}", path.display()),
    })?;
    analyze_stem_mixdown_pcm(&bundle.mixdown, bundle.sample_rate, config)
}

/// Analyze an already-decoded Stem mixdown (Fast window truncates via config).
#[cfg(feature = "analysis")]
pub(crate) fn analyze_stem_mixdown_pcm(
    mixdown: &[f32],
    sample_rate: u32,
    config: &AnalysisConfig,
) -> Result<analyzer::TrackAnalysis> {
    let mut mono = downmix_interleaved_to_mono(mixdown, 2);
    if let Some(max_ms) = config.max_duration_ms {
        let max_frames = (f64::from(max_ms) / 1000.0 * f64::from(sample_rate)).ceil() as usize;
        if mono.len() > max_frames {
            mono.truncate(max_frames);
        }
    }
    analyze_pcm(&mono, sample_rate, config).map_err(analyzer_error)
}

#[cfg(feature = "analysis")]
pub(crate) fn downmix_interleaved_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let frames = interleaved.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let base = frame * channels;
        let sum: f32 = (0..channels).map(|ch| interleaved[base + ch]).sum();
        mono.push(sum / channels as f32);
    }
    mono
}

#[cfg(feature = "analysis")]
pub(crate) fn preferred_loudness_lufs(
    measured_lufs: Option<f64>,
    replaygain_db: Option<f64>,
) -> Option<f64> {
    replaygain_db
        .map(loudness_lufs_from_replaygain_track_gain_db)
        .or(measured_lufs)
}
