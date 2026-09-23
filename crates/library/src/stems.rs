//! Persist and ensure offline stem separations for library tracks.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use audio_core::LoadedAudio;
use library_core::{Library, LibraryError, LoadableAudio, Result, TrackId};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::db::{self, Db};
use crate::entity::{track_stem, TrackStemEntity};
use crate::LibraryManager;

/// Optional progress reporter: `(phase, fraction)` where fraction is `0.0..=1.0` when known.
pub type StemProgressFn = Arc<dyn Fn(&str, Option<f32>) + Send + Sync>;

/// Map stem filesystem write failures to a concise user-facing message.
pub fn stem_io_message(err: &std::io::Error) -> String {
    use std::io::ErrorKind;
    let raw = err.to_string();
    let lower = raw.to_ascii_lowercase();
    if matches!(err.kind(), ErrorKind::StorageFull)
        || lower.contains("no space")
        || lower.contains("disk quota")
        || lower.contains("quota exceeded")
    {
        return "Stem cache write failed: disk full".into();
    }
    format!("Stem cache write failed: {raw}")
}

#[derive(Debug, Clone)]
pub struct TrackStemsInfo {
    pub backend: String,
    pub source_fingerprint: String,
    pub format: String,
    pub sample_rate: i32,
    pub path: PathBuf,
    pub generated_at: String,
}

impl TrackStemsInfo {
    fn from_row(row: &track_stem::Model) -> Self {
        Self {
            backend: row.backend.clone(),
            source_fingerprint: row.source_fingerprint.clone(),
            format: if row.format.is_empty() {
                "opus".into()
            } else {
                row.format.clone()
            },
            sample_rate: row.sample_rate,
            path: PathBuf::from(&row.path),
            generated_at: row.generated_at.clone(),
        }
    }

    fn all_files_exist(&self) -> bool {
        self.path.is_file()
    }

    fn matches(&self, fingerprint: &str, backend: &str, format: &str) -> bool {
        self.all_files_exist()
            && self.source_fingerprint == fingerprint
            && model_id(&self.backend) == model_id(backend)
            && self.format == format
    }
}

/// Model id from a `{model}/{ep}` tag: stems are interchangeable across EPs.
fn model_id(backend: &str) -> &str {
    backend.split_once('/').map_or(backend, |(model, _)| model)
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

/// Bounded collision-resistant cache file name under `stems_root`.
fn stem_cache_key(track_id: &TrackId) -> String {
    format!("{:016x}", fnv1a64(track_id.as_str().as_bytes()))
}

fn stem_cache_path(stems_root: &Path, track_id: &TrackId) -> PathBuf {
    stems_root.join(format!("{}.stem.mp4", stem_cache_key(track_id)))
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

/// stem-splitter-core keeps a process-global ONNX session; serialize Demucs so
/// analyze + deck-load cannot run two splits for the same (or any) track at once.
fn demucs_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn aac_not_implemented_error() -> LibraryError {
    LibraryError::Backend {
        backend: "stems",
        message: "AAC stem encode is not implemented yet".into(),
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
    format: &str,
) -> Result<bool> {
    Ok(get_track_stems(db, track_id)?
        .is_some_and(|info| info.matches(fingerprint, backend, format)))
}

fn normalize_stems_format(format: &str) -> &'static str {
    match format {
        "flac" => "flac",
        "aac" => "aac",
        _ => "opus",
    }
}

pub(crate) fn get_track_stems(db: &Db, track_id: &TrackId) -> Result<Option<TrackStemsInfo>> {
    let row = TrackStemEntity::find_by_id(track_id.as_str())
        .one(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(row.map(|r| TrackStemsInfo::from_row(&r)))
}

pub(crate) fn delete_track_stems(db: &Db, track_id: &TrackId) -> Result<()> {
    TrackStemEntity::delete_by_id(track_id.as_str())
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

pub(crate) fn upsert_track_stems(db: &Db, track_id: &TrackId, info: &TrackStemsInfo) -> Result<()> {
    let active = track_stem::ActiveModel {
        track_id: Set(track_id.as_str().to_string()),
        backend: Set(info.backend.clone()),
        source_fingerprint: Set(info.source_fingerprint.clone()),
        format: Set(info.format.clone()),
        sample_rate: Set(info.sample_rate),
        path: Set(info.path.to_string_lossy().into_owned()),
        generated_at: Set(info.generated_at.clone()),
    };

    TrackStemEntity::insert(active)
        .on_conflict(
            OnConflict::column(track_stem::Column::TrackId)
                .update_columns([
                    track_stem::Column::Backend,
                    track_stem::Column::SourceFingerprint,
                    track_stem::Column::Format,
                    track_stem::Column::SampleRate,
                    track_stem::Column::Path,
                    track_stem::Column::GeneratedAt,
                ])
                .to_owned(),
        )
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    Ok(())
}

/// Like [`ensure_track_stems`], but for exclusive `&mut LibraryManager` callers
/// (e.g. [`WritableLibrary::analyze_track`](library_core::WritableLibrary::analyze_track)).
pub(crate) fn ensure_track_stems_on(
    library: &mut LibraryManager,
    id: &TrackId,
    stems_root: &Path,
    models_root: &Path,
    format: &str,
) -> Result<()> {
    let backend = expected_backend();
    let format = normalize_stems_format(format);
    if format == "aac" {
        return Err(aac_not_implemented_error());
    }
    let path = {
        let source = library
            .get_track(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        source
            .file()
            .ok_or(LibraryError::Unsupported("stream tracks have no stems"))?
            .path()
            .to_path_buf()
    };

    #[cfg(feature = "analysis")]
    if codec::is_stem_path(&path) {
        return Ok(());
    }

    let cached = {
        let cache = LibraryManager::lock_decode_cache(&library.decode_cache)?;
        cache.get(id).cloned()
    };

    let audio = if let Some(cached) = cached {
        cached
    } else {
        let source = library
            .get_track(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        if source.file().is_none() {
            return Err(LibraryError::Unsupported("stream tracks have no stems"));
        }
        let loaded = Arc::new(source.load().map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("failed to decode track for stems: {e}"),
        })?);
        let mut cache = LibraryManager::lock_decode_cache(&library.decode_cache)?;
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
    if has_valid_track_stems(&library.db, id, &fingerprint, backend, format)? {
        return Ok(());
    }

    let _demucs = demucs_lock().lock().unwrap_or_else(|e| e.into_inner());
    if has_valid_track_stems(&library.db, id, &fingerprint, backend, format)? {
        return Ok(());
    }

    let info = generate_stem_files(
        id,
        stems_root,
        models_root,
        &audio,
        &fingerprint,
        format,
        None,
    )?;
    if has_valid_track_stems(&library.db, id, &fingerprint, backend, format)? {
        return Ok(());
    }
    upsert_track_stems(&library.db, id, &info)
}

fn generate_stem_files(
    id: &TrackId,
    stems_root: &Path,
    models_root: &Path,
    audio: &LoadedAudio,
    fingerprint: &str,
    format: &str,
    progress: Option<StemProgressFn>,
) -> Result<TrackStemsInfo> {
    let report = |phase: &str, fraction: Option<f32>| {
        if let Some(cb) = progress.as_ref() {
            cb(phase, fraction);
        }
    };

    #[cfg(feature = "analysis")]
    {
        let format = normalize_stems_format(format);
        if format == "aac" {
            return Err(aac_not_implemented_error());
        }

        let mux_format = match format {
            "flac" => codec::StemMuxFormat::Flac,
            _ => codec::StemMuxFormat::Opus,
        };
        let stem_rate = analyzer_stems::STEM_SAMPLE_RATE;
        let mux_rate = match mux_format {
            codec::StemMuxFormat::Opus => codec::OPUS_SAMPLE_RATE,
            codec::StemMuxFormat::Flac => stem_rate,
        };

        let handle =
            analyzer_stems::resolve_model(models_root, analyzer_stems::DEFAULT_MODEL, None)
                .map_err(|e| LibraryError::Backend {
                    backend: "stems",
                    message: e.to_string(),
                })?;

        report("stems_separate", None);
        let progress_for_split = progress.clone();
        let (stems, ep) = analyzer_stems::separate_interleaved(
            &handle.local_path,
            &audio.samples,
            audio.sample_rate,
            Some(&|done: usize, total: usize| {
                if total > 0 {
                    if let Some(cb) = progress_for_split.as_ref() {
                        if done == 0 {
                            cb("stems_separate", None);
                        } else {
                            cb("stems_separate", Some(done as f32 / total as f32));
                        }
                    }
                }
            }),
        )
        .map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: e.to_string(),
        })?;

        report("stems_encode", Some(0.0));

        let mixdown = analyzer_stems::resample_interleaved_stereo(
            &audio.samples,
            audio.sample_rate,
            mux_rate,
        )
        .map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("resample mixdown: {e}"),
        })?;

        let stems_at_rate: [Vec<f32>; 4] = if mux_rate == stem_rate {
            stems
        } else {
            let mut out = std::array::from_fn(|_| Vec::new());
            for (i, stem) in stems.into_iter().enumerate() {
                out[i] = analyzer_stems::resample_interleaved_stereo(&stem, stem_rate, mux_rate)
                    .map_err(|e| LibraryError::Backend {
                        backend: "stems",
                        message: format!("resample stem {i}: {e}"),
                    })?;
            }
            out
        };

        let final_path = stem_cache_path(stems_root, id);
        if let Some(parent) = final_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| LibraryError::Backend {
                backend: "stems",
                message: stem_io_message(&e),
            })?;
        }

        let stem_refs = [
            stems_at_rate[0].as_slice(),
            stems_at_rate[1].as_slice(),
            stems_at_rate[2].as_slice(),
            stems_at_rate[3].as_slice(),
        ];
        if let Err(e) = codec::encode_stem_mp4(
            &final_path,
            mux_format,
            mux_rate,
            &mixdown,
            stem_refs,
            &codec::StemAtom::default_ni(),
        ) {
            let _ = std::fs::remove_file(&final_path);
            return Err(LibraryError::Backend {
                backend: "stems",
                message: e.to_string(),
            });
        }

        if !final_path.is_file() {
            return Err(LibraryError::Backend {
                backend: "stems",
                message: format!("missing published stem {}", final_path.display()),
            });
        }

        report("stems_encode", Some(1.0));

        Ok(TrackStemsInfo {
            backend: format!("{}/{ep}", analyzer_stems::DEFAULT_MODEL),
            source_fingerprint: fingerprint.to_string(),
            format: format.to_string(),
            sample_rate: mux_rate as i32,
            path: final_path,
            generated_at: now_iso(),
        })
    }

    #[cfg(not(feature = "analysis"))]
    {
        let _ = (
            id,
            stems_root,
            models_root,
            audio,
            fingerprint,
            format,
            report,
        );
        Err(LibraryError::Unsupported(
            "stem separation requires the analysis feature",
        ))
    }
}

/// Generate and persist stems when missing or stale. No-op when `enabled` is false.
///
/// Takes `&Mutex<LibraryManager>` so decode / Demucs work does not hold the library lock.
pub fn ensure_track_stems(
    library: &Mutex<LibraryManager>,
    id: &TrackId,
    stems_root: &Path,
    models_root: &Path,
    enabled: bool,
    format: &str,
) -> Result<()> {
    ensure_track_stems_with_progress(library, id, stems_root, models_root, enabled, format, None)
}

/// Like [`ensure_track_stems`] with optional phase progress callbacks.
pub fn ensure_track_stems_with_progress(
    library: &Mutex<LibraryManager>,
    id: &TrackId,
    stems_root: &Path,
    models_root: &Path,
    enabled: bool,
    format: &str,
    progress: Option<StemProgressFn>,
) -> Result<()> {
    if !enabled {
        return Ok(());
    }

    let report = |phase: &str, fraction: Option<f32>| {
        if let Some(cb) = progress.as_ref() {
            cb(phase, fraction);
        }
    };

    let backend = expected_backend();
    let format = normalize_stems_format(format);
    if format == "aac" {
        return Err(aac_not_implemented_error());
    }
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

    #[cfg(feature = "analysis")]
    if codec::is_stem_path(&path) {
        return Ok(());
    }

    report("decode", None);
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
        if has_valid_track_stems(&lib.db, id, &fingerprint, backend, format)? {
            report("stems_ready", Some(1.0));
            return Ok(());
        }
    }

    let _demucs = demucs_lock().lock().unwrap_or_else(|e| e.into_inner());
    {
        let lib = LibraryManager::lock_library(library)?;
        if has_valid_track_stems(&lib.db, id, &fingerprint, backend, format)? {
            report("stems_ready", Some(1.0));
            return Ok(());
        }
    }

    report("stems_model", None);
    let info = generate_stem_files(
        id,
        stems_root,
        models_root,
        &audio,
        &fingerprint,
        format,
        progress.clone(),
    )?;
    let lib = LibraryManager::lock_library(library)?;
    if has_valid_track_stems(&lib.db, id, &fingerprint, backend, format)? {
        report("stems_ready", Some(1.0));
        return Ok(());
    }
    upsert_track_stems(&lib.db, id, &info)?;
    report("stems_ready", Some(1.0));
    Ok(())
}

/// Recursive byte sum of files under `root`. Missing root → 0.
pub fn dir_size(root: &Path) -> u64 {
    fn walk(path: &Path, acc: &mut u64) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_file() {
                *acc = acc.saturating_add(meta.len());
            } else if meta.is_dir() {
                walk(&path, acc);
            }
        }
    }
    let mut total = 0u64;
    if root.is_dir() {
        walk(root, &mut total);
    }
    total
}

/// Size of a SQLite DB file plus WAL/SHM sidecars (missing → 0).
pub fn sqlite_db_bytes(db_path: &Path) -> u64 {
    let mut total = 0u64;
    for path in [
        db_path.to_path_buf(),
        PathBuf::from(format!("{}-wal", db_path.display())),
        PathBuf::from(format!("{}-shm", db_path.display())),
    ] {
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

/// Delete all `track_stem` rows and wipe `stems_root` (recreate empty). Never touches library audio.
pub fn clear_all_track_stems(db: &Db, stems_root: &Path) -> Result<()> {
    TrackStemEntity::delete_many()
        .filter(track_stem::Column::TrackId.is_not_null())
        .exec(db.conn()?.as_connection())
        .map_err(db::db_err)?;
    if stems_root.exists() {
        std::fs::remove_dir_all(stems_root).map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("clear stems cache: {e}"),
        })?;
    }
    std::fs::create_dir_all(stems_root).map_err(|e| LibraryError::Backend {
        backend: "stems",
        message: format!("recreate stems root: {e}"),
    })?;
    Ok(())
}

/// Wipe `models_root` and recreate empty.
pub fn clear_model_cache(models_root: &Path) -> Result<()> {
    if models_root.exists() {
        std::fs::remove_dir_all(models_root).map_err(|e| LibraryError::Backend {
            backend: "stems",
            message: format!("clear model cache: {e}"),
        })?;
    }
    std::fs::create_dir_all(models_root).map_err(|e| LibraryError::Backend {
        backend: "stems",
        message: format!("recreate models root: {e}"),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use library_core::LibraryConfig;
    use std::sync::Mutex;

    #[test]
    fn stem_io_message_maps_disk_full() {
        let enospc = std::io::Error::from(std::io::ErrorKind::StorageFull);
        assert_eq!(
            stem_io_message(&enospc),
            "Stem cache write failed: disk full"
        );
        let quota = std::io::Error::other("Disk quota exceeded");
        assert_eq!(
            stem_io_message(&quota),
            "Stem cache write failed: disk full"
        );
        let other = std::io::Error::other("permission denied");
        assert!(stem_io_message(&other).contains("permission denied"));
    }

    #[test]
    fn ensure_track_stems_disabled_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("library.db");
        let library = Mutex::new(LibraryManager::open(&db_path, LibraryConfig::default()).unwrap());
        let id = TrackId::new("/missing/track.wav");
        ensure_track_stems(&library, &id, dir.path(), dir.path(), false, "opus").unwrap();
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

    #[test]
    fn stem_cache_path_uses_stem_mp4_suffix() {
        let root = Path::new("/stems");
        let id = TrackId::new("/music/track.wav");
        let path = stem_cache_path(root, &id);
        assert!(path.to_string_lossy().ends_with(".stem.mp4"));
        assert_eq!(path.parent().unwrap(), root);
    }

    #[test]
    fn dir_size_sums_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
        std::fs::write(dir.path().join("a/one.bin"), [0u8; 10]).unwrap();
        std::fs::write(dir.path().join("a/b/two.bin"), [0u8; 5]).unwrap();
        assert_eq!(dir_size(dir.path()), 15);
        assert_eq!(dir_size(&dir.path().join("missing")), 0);
    }

    #[test]
    fn dir_size_includes_dot_ephemeral_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("abcd1234efgh5678.stem.mp4"), [0u8; 20]).unwrap();
        std::fs::write(
            dir.path().join(".abcd1234efgh5678.stem.mp4.tmp"),
            [0u8; 100],
        )
        .unwrap();
        assert_eq!(dir_size(dir.path()), 120);
    }

    #[test]
    fn clear_all_track_stems_wipes_files() {
        let dir = tempfile::tempdir().unwrap();
        let stems = dir.path().join("stems");
        std::fs::create_dir_all(stems.join("legacy_dir")).unwrap();
        std::fs::write(stems.join("abcd.stem.mp4"), b"stem").unwrap();
        std::fs::write(stems.join("legacy_dir/vocals.opus"), b"opus").unwrap();
        let db_path = dir.path().join("library.db");
        let library = LibraryManager::open(&db_path, LibraryConfig::default()).unwrap();

        clear_all_track_stems(&library.db, &stems).unwrap();
        assert!(get_track_stems(&library.db, &TrackId::new("/music/t.wav"))
            .unwrap()
            .is_none());
        assert!(stems.is_dir());
        assert!(stems.read_dir().unwrap().next().is_none());
    }

    #[test]
    fn clear_model_cache_empties_dir() {
        let dir = tempfile::tempdir().unwrap();
        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("x.onnx"), b"onnx").unwrap();
        clear_model_cache(&models).unwrap();
        assert!(models.is_dir());
        assert!(models.read_dir().unwrap().next().is_none());
    }

    #[test]
    fn stems_format_mismatch_fails_matches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.stem.mp4");
        std::fs::write(&path, b"x").unwrap();
        let info = TrackStemsInfo {
            backend: "htdemucs_mixxx_v1".into(),
            source_fingerprint: "fp".into(),
            format: "opus".into(),
            sample_rate: 48_000,
            path,
            generated_at: "1".into(),
        };
        assert!(info.matches("fp", "htdemucs_mixxx_v1", "opus"));
        assert!(!info.matches("fp", "htdemucs_mixxx_v1", "flac"));
    }

    #[test]
    fn normalize_stems_format_defaults_unknown_to_opus() {
        assert_eq!(normalize_stems_format("flac"), "flac");
        assert_eq!(normalize_stems_format("opus"), "opus");
        assert_eq!(normalize_stems_format("aac"), "aac");
        assert_eq!(normalize_stems_format("wav"), "opus");
        assert_eq!(normalize_stems_format(""), "opus");
    }

    #[cfg(feature = "analysis")]
    #[test]
    fn is_stem_source_path_is_detected() {
        assert!(codec::is_stem_path(Path::new("/music/track.stem.mp4")));
        assert!(!codec::is_stem_path(Path::new("/music/track.wav")));
    }
}

#[cfg(test)]
mod options_tests {
    use library_core::AnalyzeTrackOptions;

    #[test]
    fn validate_stems_rejects_enabled_without_root() {
        let opts = AnalyzeTrackOptions {
            stems_enabled: true,
            stems_root: None,
            models_root: Some(std::path::PathBuf::from("models")),
            ..Default::default()
        };
        assert!(opts.validate_stems().is_err());
    }

    #[test]
    fn validate_stems_rejects_enabled_without_models_root() {
        let opts = AnalyzeTrackOptions {
            stems_enabled: true,
            stems_root: Some(std::path::PathBuf::from("stems")),
            models_root: None,
            ..Default::default()
        };
        assert!(opts.validate_stems().is_err());
    }
}
