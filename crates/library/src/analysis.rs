//! Persist offline analysis results in the library database.

use analyzer::{
    analyze_file, analyze_pcm, merge_track_metadata, AnalysisConfig, AnalyzerError, TagMetadata,
    TrackAnalysis,
};
use analyzer_core::loudness_lufs_from_replaygain_track_gain_db;
use codec::{decode_stem_file, is_stem_path};
use sea_orm::{EntityTrait, Set};
use std::path::Path;

use library_core::{AnalyzeTrackOptions, CueSource, LibraryError, Result, TrackId};

use crate::beat_grid;
use crate::db::{self, Db};
use crate::entity::{track_analysis, tracks, TrackAnalysisEntity, TrackEntity};
use crate::{is_audio_file, normalize_path, path_label, tags};

/// Append the detector audit and persist the merged track facts as one unit.
pub(crate) fn persist_analysis_result(
    db: &Db,
    track_id: &TrackId,
    computed: &ComputedFileAnalysis,
) -> Result<()> {
    let analysis = &computed.analysis;
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
        sample_rate: Set(i64::from(analysis.metadata.sample_rate)),
        duration_analyzed_ms: Set(analysis.metadata.duration_analyzed_ms),
        loudness_lufs: Set(analysis.loudness_lufs),
        beat_grid_json: Set(beat_grid_json.clone()),
        ..Default::default()
    };

    db.transaction(|conn| {
        let row = TrackEntity::find_by_id(track_id.as_str())
            .one(conn)
            .map_err(db::db_err)?
            .ok_or_else(|| LibraryError::NotFound(track_id.as_str().to_string()))?;
        let existing_grid = beat_grid::get_track_beat_grid_tx(conn, track_id)?;
        TrackAnalysisEntity::insert(active)
            .exec(conn)
            .map_err(db::db_err)?;

        let candidates = merge_track_metadata(
            &TagMetadata::default(),
            analysis,
            computed.force,
            computed.min_bpm_confidence,
            computed.min_key_confidence,
        );
        let detector_bpm = candidates.bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.0);
        let detector_key = candidates
            .key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty());
        let keep_existing_bpm = !computed.force && (row.bpm.is_some() || row.bpm_source.is_some());
        let prefer_track_bpm = keep_existing_bpm || detector_bpm.is_none();
        let mut facts = tracks::ActiveModel {
            id: Set(row.id),
            analysis_completed_at: Set(Some(analysis.metadata.analyzed_at.clone())),
            ..Default::default()
        };
        if let Some(key) = detector_key
            .filter(|_| computed.force || (row.key.is_none() && row.key_source.is_none()))
        {
            facts.key = Set(Some(key.to_string()));
            facts.key_source = Set(Some(beat_grid::SOURCE_ANALYSIS.to_string()));
        }
        if let Some(loudness) = computed
            .canonical_loudness_lufs
            .filter(|lufs| lufs.is_finite())
        {
            facts.loudness_lufs = Set(Some(loudness));
        }
        if row.sample_rate.is_none_or(|rate| rate <= 0) {
            if let Ok(rate) = i32::try_from(analysis.metadata.sample_rate) {
                if rate > 0 {
                    facts.sample_rate = Set(Some(rate));
                }
            }
        }

        let can_replace_grid = existing_grid
            .as_ref()
            .is_none_or(|grid| computed.force && grid.source != beat_grid::SOURCE_IMPORT);
        let mut wrote_grid = false;
        let preferred_bpm = if prefer_track_bpm { row.bpm } else { None };
        let winning_bpm = preferred_bpm.or(detector_bpm);
        if can_replace_grid
            && winning_bpm.is_none_or(|bpm| (20.0..=400.0).contains(&bpm))
            && !(keep_existing_bpm && row.bpm.is_none())
        {
            // A rejected BPM estimate must not sneak into the track through geometry.
            // If there was no estimate at all, the measured markers can stand alone.
            if let Some(json) = beat_grid_json.as_deref().filter(|_| {
                preferred_bpm.is_some() || detector_bpm.is_some() || analysis.bpm.is_none()
            }) {
                let analyzed_ms = analysis.metadata.duration_analyzed_ms;
                let covered_to_ms = (analyzed_ms > 0
                    && row
                        .duration_ms
                        .is_none_or(|duration| analyzed_ms < duration))
                .then_some(i64::from(analyzed_ms));
                let raw_bpm = analysis.bpm.as_ref().map(|bpm| bpm.bpm);
                match beat_grid::from_analysis_json(json, raw_bpm, preferred_bpm, covered_to_ms) {
                    Ok(Some(grid)) => {
                        let source = if preferred_bpm.is_some() {
                            row.bpm_source.as_deref().unwrap_or("legacy")
                        } else {
                            beat_grid::SOURCE_ANALYSIS
                        };
                        beat_grid::write_track_beat_grid_tx(conn, track_id, &grid, source)?;
                        wrote_grid = true;
                    }
                    Ok(None) => tracing::warn!(track_id = track_id.as_str(), "skipping unusable analysis grid"),
                    Err(err) => {
                        tracing::warn!(track_id = track_id.as_str(), error = %err, "skipping malformed analysis grid")
                    }
                }
            }
        }
        if !wrote_grid && existing_grid.is_none() && !keep_existing_bpm {
            if let Some(bpm) = detector_bpm {
                facts.bpm = Set(Some(bpm));
                facts.bpm_source = Set(Some(beat_grid::SOURCE_ANALYSIS.to_string()));
            }
        }
        TrackEntity::update(facts).exec(conn).map_err(db::db_err)?;

        let updated_at = analysis.metadata.analyzed_at.clone();
        match analysis.cue_ms {
            Some(cue_ms) => crate::deck_data::refresh_track_cue_tx(
                conn,
                track_id,
                cue_ms,
                CueSource::Analysis,
                updated_at,
            )?,
            None => crate::deck_data::fill_track_cue_tx(
                conn,
                track_id,
                0,
                CueSource::Analysis,
                updated_at,
            )?,
        };
        Ok(())
    })
}

fn clamp_cue_threshold_db(threshold_db: f32) -> f32 {
    if threshold_db.is_finite() {
        threshold_db.clamp(
            library_core::MIN_CUE_THRESHOLD_DB,
            library_core::MAX_CUE_THRESHOLD_DB,
        )
    } else {
        library_core::DEFAULT_CUE_THRESHOLD_DB
    }
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
    /// The original ordinary tags, before analysis merged into them.
    ///
    /// Discovery persists these, so an analyzed key can never masquerade as a
    /// standard tag and defeat the Mixed In Key fallback.
    pub(crate) tag_metadata: library_core::TrackMetadata,
    /// Vendor performance metadata read from the same file.
    pub(crate) embedded_metadata: crate::serato_adoption::VendorRead,
    pub(crate) analysis: TrackAnalysis,
    pub(crate) force: bool,
    pub(crate) min_bpm_confidence: f32,
    pub(crate) min_key_confidence: f32,
    pub(crate) canonical_loudness_lufs: Option<f64>,
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
    if let Some(cue_threshold_db) = options.cue_threshold_db {
        config.cue_threshold_db = clamp_cue_threshold_db(cue_threshold_db);
    }
    let (tag_metadata, embedded_metadata) = tags::read_file_metadata(&path)?;
    config.max_duration_ms = options
        .analysis_duration
        .resolve_max_duration_ms(tag_metadata.duration_ms);
    let analysis = if is_stem_path(&path) {
        analyze_stem_mixdown(&path, &config)?
    } else {
        analyze_file(&path, &config).map_err(analyzer_error)?
    };
    let replaygain_track_gain_db = tags::read_replaygain_track_gain_db(&path)?;
    let canonical_loudness_lufs =
        preferred_loudness_lufs(analysis.loudness_lufs, replaygain_track_gain_db);
    Ok(ComputedFileAnalysis {
        tag_metadata,
        embedded_metadata,
        analysis,
        force: options.force,
        min_bpm_confidence: config.min_bpm_confidence,
        min_key_confidence: config.min_key_confidence,
        canonical_loudness_lufs,
    })
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
        .filter(|db| db.is_finite())
        .map(loudness_lufs_from_replaygain_track_gain_db)
        .filter(|lufs| lufs.is_finite())
        .or_else(|| measured_lufs.filter(|lufs| lufs.is_finite()))
}
