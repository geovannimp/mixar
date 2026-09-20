//! Persist and ensure offline stem separations for library tracks.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use library_core::{Library, LibraryError, LoadableAudio, Result, TrackId};
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Set};

use crate::db::{self, Db};
use crate::entity::{track_stem, TrackStemEntity};
use crate::LibraryManager;

#[derive(Debug, Clone)]
pub struct TrackStemsInfo {
    pub backend: String,
    pub sample_rate: i32,
    pub vocals_path: PathBuf,
    pub drums_path: PathBuf,
    pub bass_path: PathBuf,
    pub other_path: PathBuf,
    pub generated_at: String,
}

impl TrackStemsInfo {
    fn from_row(row: &track_stem::Model) -> Self {
        Self {
            backend: row.backend.clone(),
            sample_rate: row.sample_rate,
            vocals_path: PathBuf::from(&row.vocals_path),
            drums_path: PathBuf::from(&row.drums_path),
            bass_path: PathBuf::from(&row.bass_path),
            other_path: PathBuf::from(&row.other_path),
            generated_at: row.generated_at.clone(),
        }
    }

    fn paths(&self) -> [&Path; 4] {
        [
            self.vocals_path.as_path(),
            self.drums_path.as_path(),
            self.bass_path.as_path(),
            self.other_path.as_path(),
        ]
    }

    fn all_files_exist(&self) -> bool {
        self.paths().iter().all(|p| p.is_file())
    }
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Keep stem dirs under `stems_root` even when `track_id` looks like an absolute path.
fn stem_output_dir(stems_root: &Path, track_id: &TrackId) -> PathBuf {
    let safe: String = track_id
        .as_str()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    stems_root.join(safe)
}

pub(crate) fn has_track_stems(db: &Db, track_id: &TrackId) -> Result<bool> {
    Ok(get_track_stems(db, track_id)?.is_some_and(|info| info.all_files_exist()))
}

pub(crate) fn get_track_stems(db: &Db, track_id: &TrackId) -> Result<Option<TrackStemsInfo>> {
    let row = TrackStemEntity::find_by_id(track_id.as_str())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(row.map(|r| TrackStemsInfo::from_row(&r)))
}

pub(crate) fn upsert_track_stems(db: &Db, track_id: &TrackId, info: &TrackStemsInfo) -> Result<()> {
    let active = track_stem::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        backend: Set(info.backend.clone()),
        sample_rate: Set(info.sample_rate),
        vocals_path: Set(info.vocals_path.to_string_lossy().into_owned()),
        drums_path: Set(info.drums_path.to_string_lossy().into_owned()),
        bass_path: Set(info.bass_path.to_string_lossy().into_owned()),
        other_path: Set(info.other_path.to_string_lossy().into_owned()),
        generated_at: Set(info.generated_at.clone()),
    };

    TrackStemEntity::insert(active)
        .on_conflict(
            OnConflict::column(track_stem::Column::TrackId)
                .update_columns([
                    track_stem::Column::Backend,
                    track_stem::Column::SampleRate,
                    track_stem::Column::VocalsPath,
                    track_stem::Column::DrumsPath,
                    track_stem::Column::BassPath,
                    track_stem::Column::OtherPath,
                    track_stem::Column::GeneratedAt,
                ])
                .to_owned(),
        )
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

/// Generate and persist stems when missing. No-op when `enabled` is false.
///
/// Takes `&Mutex<LibraryManager>` so decode / Demucs work does not hold the library lock.
pub fn ensure_track_stems(
    library: &Mutex<LibraryManager>,
    id: &TrackId,
    stems_root: &Path,
    enabled: bool,
) -> Result<()> {
    if !enabled {
        return Ok(());
    }

    {
        let lib = LibraryManager::lock_library(library)?;
        if has_track_stems(&lib.db, id)? {
            return Ok(());
        }
        let _ = lib
            .get_track(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
            .file()
            .ok_or(LibraryError::Unsupported("stream tracks have no stems"))?;
    }

    let cached = {
        let lib = LibraryManager::lock_library(library)?;
        let cache = LibraryManager::lock_decode_cache(&lib.decode_cache)?;
        cache.get(id).cloned()
    };

    let audio = if let Some(cached) = cached {
        cached
    } else {
        let source = {
            let lib = LibraryManager::lock_library(library)?;
            lib.get_track(id)?
                .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
        };
        if source.file().is_none() {
            return Err(LibraryError::Unsupported("stream tracks have no stems"));
        }
        let loaded = Arc::new(source.load().map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("failed to decode track for stems: {e}"),
        })?);
        let lib = LibraryManager::lock_library(library)?;
        let mut cache = LibraryManager::lock_decode_cache(&lib.decode_cache)?;
        if let Some(existing) = cache.get(id) {
            Arc::clone(existing)
        } else {
            cache.insert(id.clone(), Arc::clone(&loaded));
            loaded
        }
    };

    if audio.channels != 2 {
        return Err(LibraryError::Backend {
            backend: "stems",
            message: format!(
                "stem separation requires interleaved stereo (got {} channels)",
                audio.channels
            ),
        });
    }

    {
        let lib = LibraryManager::lock_library(library)?;
        if has_track_stems(&lib.db, id)? {
            return Ok(());
        }
    }

    #[cfg(feature = "analysis")]
    {
        let output_dir = stem_output_dir(stems_root, id);
        let result = analyzer_stems::split_interleaved_stereo(analyzer_stems::StemSplitRequest {
            interleaved_stereo: &audio.samples,
            sample_rate: audio.sample_rate,
            output_dir: &output_dir,
            model_name: analyzer_stems::DEFAULT_MODEL,
        })
        .map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: e.to_string(),
        })?;

        let lib = LibraryManager::lock_library(library)?;
        if has_track_stems(&lib.db, id)? {
            return Ok(());
        }
        upsert_track_stems(
            &lib.db,
            id,
            &TrackStemsInfo {
                backend: result.backend,
                sample_rate: result.sample_rate as i32,
                vocals_path: result.paths[0].clone(),
                drums_path: result.paths[1].clone(),
                bass_path: result.paths[2].clone(),
                other_path: result.paths[3].clone(),
                generated_at: now_iso(),
            },
        )
    }

    #[cfg(not(feature = "analysis"))]
    {
        let _ = (audio, stems_root);
        Err(LibraryError::Unsupported(
            "stem separation requires the analysis feature",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use library_core::LibraryConfig;
    use std::sync::Mutex;

    #[test]
    fn ensure_track_stems_disabled_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        let library = Mutex::new(LibraryManager::open(&db_path, LibraryConfig::default()).unwrap());
        let id = TrackId::new("/missing/track.wav");
        ensure_track_stems(&library, &id, dir.path(), false).unwrap();
        let lib = library.lock().unwrap();
        assert!(!has_track_stems(&lib.db, &id).unwrap());
    }
}
