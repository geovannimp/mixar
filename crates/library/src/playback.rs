//! Track playback preparation and analysis gating.
use super::*;
#[cfg(feature = "analysis")]
use crate::analysis::compute_file_analysis;
#[cfg(feature = "analysis")]
use crate::stems::{loaded_from_stem_bundle, prepare_native_stem_for_playback};
use codec::is_stem_path;

impl LibraryManager {
    /// True when BPM, key, or loudness needed for playback is missing.
    ///
    /// shortcut: if an analysis row already exists with loudness but BPM/key stay
    /// empty (analyzer could not detect them), do not re-run forever. Upgrade:
    /// surface "analysis incomplete" in UI and allow manual force re-analyze.
    pub fn needs_playback_analysis(&self, id: &TrackId) -> Result<bool> {
        let source = self
            .get_track(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        let meta = source.metadata();
        if self.track_loudness_lufs(id)?.is_none() {
            return Ok(true);
        }
        let bpm_ok = meta.bpm.is_some();
        let key_ok = meta.key.as_ref().is_some_and(|key| !key.trim().is_empty());
        if bpm_ok && key_ok {
            return Ok(false);
        }
        Ok(self.store().count_track_analysis(id)? == 0)
    }

    /// Run Fast analysis when playback fields are missing (BPM, key, loudness).
    ///
    /// Does not generate waveform peaks. Takes `&Mutex<Self>` so analysis does
    /// not hold the library lock.
    #[cfg(feature = "analysis")]
    pub fn ensure_track_analysis(library: &Mutex<Self>, id: &TrackId) -> Result<()> {
        let path = {
            let lib = Self::lock_library(library)?;
            if !lib.needs_playback_analysis(id)? {
                return Ok(());
            }
            lib.get_track(id)?
                .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
                .file()
                .ok_or(LibraryError::Unsupported(
                    "stream track analysis not implemented",
                ))?
                .path()
                .to_path_buf()
        };

        let options = AnalyzeTrackOptions {
            force: false,
            analysis_duration: AnalysisDurationMode::Fast,
        };
        let computed = compute_file_analysis(&path, options)?;

        let lib = Self::lock_library(library)?;
        if !lib.needs_playback_analysis(id)? {
            return Ok(());
        }
        lib.persist_file_analysis(&path, &computed, false)?;
        Ok(())
    }

    #[cfg(not(feature = "analysis"))]
    pub fn ensure_track_analysis(library: &Mutex<Self>, id: &TrackId) -> Result<()> {
        let _ = (library, id);
        Ok(())
    }

    /// Run analysis without holding the library mutex during DSP work.
    pub fn analyze_track_off_mutex(
        library: &Mutex<Self>,
        id: &TrackId,
        options: AnalyzeTrackOptions,
    ) -> Result<AudioSource> {
        let path = {
            let lib = Self::lock_library(library)?;
            let source = lib
                .get_track(id)?
                .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
            match source {
                AudioSource::File(file) => file.path().to_path_buf(),
                AudioSource::Stream(_) => {
                    return Err(LibraryError::Unsupported(
                        "stream track analysis not implemented",
                    ));
                }
            }
        };

        #[cfg(feature = "analysis")]
        {
            let computed = compute_file_analysis(&path, options.clone())?;
            let source = {
                let lib = Self::lock_library(library)?;
                lib.ensure_track_still_at_path(id, &path)?;
                lib.persist_file_analysis(&path, &computed, true)?
            };
            Ok(source)
        }
        #[cfg(not(feature = "analysis"))]
        {
            let _ = options;
            let lib = Self::lock_library(library)?;
            lib.ensure_track_still_at_path(id, &path)?;
            lib.refresh_file_source(&path)
        }
    }

    /// Read the stored integrated loudness for a track, if it has been analyzed.
    pub fn track_loudness_lufs(&self, id: &TrackId) -> Result<Option<f64>> {
        self.store().track_analysis_loudness(id)
    }

    /// Import/refresh a file, ensure library-managed playback metadata, and return a cached decode.
    ///
    /// Takes `&Mutex<Self>` so decode / waveform work does not hold the library lock.
    pub fn prepare_file_path_for_playback(
        library: &Mutex<Self>,
        path: &Path,
    ) -> Result<PreparedTrackPlayback> {
        let source = {
            let lib = Self::lock_library(library)?;
            lib.import_file_path(path)?
        };
        Self::prepare_source_for_playback(library, source)
    }

    /// Resolve a library track, ensure library-managed playback metadata, and return a cached decode.
    ///
    /// Takes `&Mutex<Self>` so decode / waveform work does not hold the library lock.
    pub fn prepare_track_for_playback(
        library: &Mutex<Self>,
        id: &TrackId,
    ) -> Result<PreparedTrackPlayback> {
        let source = {
            let lib = Self::lock_library(library)?;
            lib.get_track(id)?
                .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
        };
        Self::prepare_source_for_playback(library, source)
    }

    pub(crate) fn prepare_source_for_playback(
        library: &Mutex<Self>,
        mut source: AudioSource,
    ) -> Result<PreparedTrackPlayback> {
        let track_id = source.id().clone();

        #[cfg(feature = "analysis")]
        if let Some(path) = source
            .file()
            .map(|f| f.path().to_path_buf())
            .filter(|path| is_stem_path(path))
        {
            return prepare_native_stem_for_playback(library, source, path);
        }

        if source.file().is_some() {
            Self::ensure_track_analysis(library, &track_id)?;
            source = {
                let lib = Self::lock_library(library)?;
                lib.get_track(&track_id)?
                    .ok_or_else(|| LibraryError::NotFound(track_id.to_string()))?
            };
        }
        let loudness_lufs = {
            let lib = Self::lock_library(library)?;
            lib.track_loudness_lufs(&track_id)?
        };
        source.metadata_mut().loudness_lufs = loudness_lufs;

        #[cfg(feature = "analysis")]
        {
            let source_path = source.file().map(|f| f.path().to_path_buf());
            let cache = {
                let lib = Self::lock_library(library)?;
                lib.get_track_stems(&track_id)?
            };
            if let Some(info) = cache {
                let stale = source_path
                    .as_ref()
                    .is_some_and(|path| !stems::cache_matches_source_file(&info, path));
                if stale {
                    tracing::warn!(
                        track_id = %track_id,
                        cache = %info.path.display(),
                        "stem cache fingerprint mismatch; discarding"
                    );
                    let lib = Self::lock_library(library)?;
                    stems::delete_track_stems(&lib.db, &track_id)?;
                    drop(lib);
                    let _ = std::fs::remove_file(&info.path);
                } else {
                    match decode_stem_file(&info.path) {
                        Ok(bundle) => {
                            let (audio, stems) = loaded_from_stem_bundle(&info.path, bundle);
                            return Ok(PreparedTrackPlayback {
                                track_id,
                                source,
                                audio,
                                stems: Some(stems),
                                loudness_lufs,
                            });
                        }
                        Err(err) => {
                            tracing::warn!(
                                track_id = %track_id,
                                cache = %info.path.display(),
                                error = %err,
                                "stem cache decode failed; discarding"
                            );
                            let lib = Self::lock_library(library)?;
                            stems::delete_track_stems(&lib.db, &track_id)?;
                            drop(lib);
                            let _ = std::fs::remove_file(&info.path);
                        }
                    }
                }
            }
        }

        // Double-checked cache: decode off the library lock so artwork/DB reads
        // can proceed. A racing miss may decode twice; re-get before insert drops
        // the duplicate Arc. Upgrade to single-flight if wasted decode becomes costly.
        let cached = {
            let lib = Self::lock_library(library)?;
            let mut cache = Self::lock_decode_cache(&lib.decode_cache)?;
            cache.get(&track_id)
        };

        let audio = if let Some(cached) = cached {
            cached
        } else {
            let loaded = Arc::new(source.load().map_err(|e| LibraryError::Backend {
                backend: "library",
                message: format!("failed to decode track for playback: {e}"),
            })?);
            let lib = Self::lock_library(library)?;
            let mut cache = Self::lock_decode_cache(&lib.decode_cache)?;
            if let Some(existing) = cache.get(&track_id) {
                existing
            } else {
                cache.insert(track_id.clone(), loaded)
            }
        };

        Ok(PreparedTrackPlayback {
            track_id,
            source,
            audio,
            stems: None,
            loudness_lufs,
        })
    }
}
