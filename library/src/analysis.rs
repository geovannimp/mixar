//! Persist offline analysis results in the library database.

use analyzer::{AnalyzerError, TrackAnalysis};
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Set};

use library_core::{LibraryError, Result, TrackId};

use crate::db::{self, Db};
use crate::entity::{track_analysis, TrackAnalysisEntity};

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
        duration_analyzed_secs: Set(analysis.metadata.duration_analyzed_secs),
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
                    track_analysis::Column::DurationAnalyzedSecs,
                    track_analysis::Column::BeatGridJson,
                ])
                .to_owned(),
        )
        .exec(&*db.conn()?.as_connection())
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
        AnalyzerError::Backend { backend, message } => LibraryError::Backend {
            backend,
            message,
        },
    }
}
