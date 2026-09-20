//! Persist and ensure offline stem separations for library tracks.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use audio_core::LoadedAudio;
use library_core::{Library, LibraryError, LoadableAudio, Result, TrackId};
use sea_orm::sea_query::OnConflict;
use sea_orm::{EntityTrait, Set};

use crate::db::{self, Db};
use crate::entity::{track_stem, TrackStemEntity};
use crate::LibraryManager;

#[derive(Debug, Clone)]
pub struct TrackStemsInfo {
    pub backend: String,
    pub source_fingerprint: String,
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
            source_fingerprint: row.source_fingerprint.clone(),
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

    fn matches(&self, fingerprint: &str, backend: &str) -> bool {
        self.all_files_exist() && self.source_fingerprint == fingerprint && self.backend == backend
    }
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

/// Stable FNV-1a 64-bit (local cache keys; not a crypto hash).
fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Bounded collision-resistant directory name under `stems_root`.
fn stem_cache_key(track_id: &TrackId) -> String {
    format!("{:016x}", fnv1a64(track_id.as_str().as_bytes()))
}

fn stem_output_dir(stems_root: &Path, track_id: &TrackId) -> PathBuf {
    stems_root.join(stem_cache_key(track_id))
}

/// Fingerprint of the decoded source used to build stems (path mtime/size + PCM shape).
fn source_fingerprint(path: &Path, audio: &LoadedAudio) -> String {
    let meta = std::fs::metadata(path).ok();
    let mtime = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size = meta.map(|m| m.len()).unwrap_or(0);
    format!(
        "v1:{}:{}:{}:{}:{}:{}",
        path.display(),
        mtime,
        size,
        audio.sample_rate,
        audio.channels,
        audio.samples.len()
    )
}

fn expected_backend() -> &'static str {
    #[cfg(feature = "analysis")]
    {
        analyzer_stems::DEFAULT_MODEL
    }
    #[cfg(not(feature = "analysis"))]
    {
        ""
    }
}

pub(crate) fn has_track_stems(db: &Db, track_id: &TrackId) -> Result<bool> {
    Ok(get_track_stems(db, track_id)?.is_some_and(|info| info.all_files_exist()))
}

fn has_valid_track_stems(
    db: &Db,
    track_id: &TrackId,
    fingerprint: &str,
    backend: &str,
) -> Result<bool> {
    Ok(get_track_stems(db, track_id)?.is_some_and(|info| info.matches(fingerprint, backend)))
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
        source_fingerprint: Set(info.source_fingerprint.clone()),
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
                    track_stem::Column::SourceFingerprint,
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

/// Write stems into a temp dir, then atomically publish to the track cache dir.
fn publish_stem_dir(tmp_dir: &Path, final_dir: &Path) -> Result<()> {
    if let Some(parent) = final_dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("create stems root: {e}"),
        })?;
    }

    if !final_dir.exists() {
        std::fs::rename(tmp_dir, final_dir).map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("publish stems dir: {e}"),
        })?;
        return Ok(());
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let old_dir = final_dir.with_file_name(format!(
        ".{}.old-{stamp}",
        final_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("stems")
    ));
    std::fs::rename(final_dir, &old_dir).map_err(|e| LibraryError::Backend {
        backend: "stems",
        message: format!("park old stems dir: {e}"),
    })?;
    if let Err(e) = std::fs::rename(tmp_dir, final_dir) {
        let _ = std::fs::rename(&old_dir, final_dir);
        return Err(LibraryError::Backend {
            backend: "stems",
            message: format!("publish stems dir: {e}"),
        });
    }
    let _ = std::fs::remove_dir_all(&old_dir);
    Ok(())
}

/// Generate and persist stems when missing or stale. No-op when `enabled` is false.
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

    let backend = expected_backend();
    let path = {
        let lib = LibraryManager::lock_library(library)?;
        let source = lib
            .get_track(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        source
            .file()
            .ok_or(LibraryError::Unsupported("stream tracks have no stems"))?
            .path()
            .to_path_buf()
    };

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

    let fingerprint = source_fingerprint(&path, &audio);
    {
        let lib = LibraryManager::lock_library(library)?;
        if has_valid_track_stems(&lib.db, id, &fingerprint, backend)? {
            return Ok(());
        }
    }

    #[cfg(feature = "analysis")]
    {
        let key = stem_cache_key(id);
        let final_dir = stem_output_dir(stems_root, id);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp_dir = stems_root.join(format!(".{key}.tmp-{stamp}"));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        std::fs::create_dir_all(&tmp_dir).map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("create temp stems dir: {e}"),
        })?;

        let split_result =
            analyzer_stems::split_interleaved_stereo(analyzer_stems::StemSplitRequest {
                interleaved_stereo: &audio.samples,
                sample_rate: audio.sample_rate,
                output_dir: &tmp_dir,
                model_name: analyzer_stems::DEFAULT_MODEL,
            })
            .map_err(|e| LibraryError::Backend {
                backend: "stems",
                message: e.to_string(),
            });

        let result = match split_result {
            Ok(r) => r,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp_dir);
                return Err(e);
            }
        };

        if let Err(e) = publish_stem_dir(&tmp_dir, &final_dir) {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Err(e);
        }

        let vocals_path = final_dir.join("vocals.wav");
        let drums_path = final_dir.join("drums.wav");
        let bass_path = final_dir.join("bass.wav");
        let other_path = final_dir.join("other.wav");
        for p in [&vocals_path, &drums_path, &bass_path, &other_path] {
            if !p.is_file() {
                return Err(LibraryError::Backend {
                    backend: "stems",
                    message: format!("missing published stem {}", p.display()),
                });
            }
        }

        let lib = LibraryManager::lock_library(library)?;
        if has_valid_track_stems(&lib.db, id, &fingerprint, backend)? {
            return Ok(());
        }
        upsert_track_stems(
            &lib.db,
            id,
            &TrackStemsInfo {
                backend: result.backend,
                source_fingerprint: fingerprint,
                sample_rate: result.sample_rate as i32,
                vocals_path,
                drums_path,
                bass_path,
                other_path,
                generated_at: now_iso(),
            },
        )
    }

    #[cfg(not(feature = "analysis"))]
    {
        let _ = (audio, stems_root, fingerprint, backend);
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

    #[test]
    fn stem_cache_key_is_bounded_and_stable() {
        let a = TrackId::new("/music/foo bar/track.wav");
        let b = TrackId::new("/music/foo_bar/track.wav");
        assert_eq!(stem_cache_key(&a).len(), 16);
        assert_ne!(stem_cache_key(&a), stem_cache_key(&b));
        assert_eq!(stem_cache_key(&a), stem_cache_key(&a));
    }
}
