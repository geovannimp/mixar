//! Map codec file tags into library `TrackMetadata`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use codec::{
    read_file_artwork, read_file_tags, read_replaygain_track_gain_db as codec_replaygain, FileTags,
    TagWriteReport, TrackTags,
};
use library_core::{path_label, TagWriteBack, TrackMetadata};

/// Read metadata tags from an audio file.
pub fn read_tags(path: &Path) -> library_core::Result<TrackMetadata> {
    let tags = read_file_tags(path).map_err(|e| io_backend(format!("{e}")))?;
    Ok(track_metadata_from_file_tags(path, tags))
}

/// Deferred write-back of app-managed tags for one analyzed file.
///
/// Planned while the library lock is held (cheap: copies a path and two values)
/// and executed by the caller *after* releasing it. The write reads the file and
/// may rewrite it wholesale, which must not sit behind the manager mutex where a
/// queued UI read would wait on it.
///
/// Holding that mutex used to serialize these writes implicitly; `gate` restores
/// the guarantee now that they run outside it, so two analysis paths reaching the
/// same file at once (deck-load pre-analysis and a queued `AnalyzeTrack`, say)
/// still write one at a time.
pub(crate) struct TagWritePlan {
    path: PathBuf,
    tags: TrackTags,
    gate: Arc<Mutex<()>>,
}

impl TagWritePlan {
    /// Plan for `path`, or `None` when the user has not opted in.
    pub(crate) fn new(
        mode: TagWriteBack,
        path: PathBuf,
        tags: TrackTags,
        gate: Arc<Mutex<()>>,
    ) -> Option<Self> {
        mode.is_on().then_some(Self { path, tags, gate })
    }

    /// Run the write. Best-effort — failures are logged, never propagated.
    pub(crate) fn execute(self) -> Option<TagWriteReport> {
        let _gate = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        write_back_track_tags(&self.path, &self.tags)
    }
}

/// Mirror track information into the file's tags.
///
/// Best-effort: a file we cannot write is logged and skipped, never fatal — the
/// library row is already persisted and `library.db` stays authoritative. Runs on
/// the analysis / beat-grid paths only, never on the realtime audio path.
pub(crate) fn write_back_track_tags(path: &Path, tags: &TrackTags) -> Option<TagWriteReport> {
    match codec::write_track_tags(path, tags) {
        Ok(report) => {
            tracing::debug!(
                path = %path_label(path),
                container = report.container.map(|c| c.as_str()).unwrap_or("none"),
                fields = ?report.fields,
                "tag write-back"
            );
            Some(report)
        }
        Err(err) => {
            tracing::warn!(
                path = %path_label(path),
                error = %err,
                "tag write-back failed; file left unchanged"
            );
            None
        }
    }
}

/// The fields Mixar owns and mirrors today: BPM and key.
///
/// Title/artist/album/genre are deliberately **not** passed even though the
/// writer supports them: they are file-derived today (`track_metadata_from_file_tags`
/// falls back to the file name for a missing title), so mirroring the library row
/// would stamp guesses into files that carry no tags. A track editor that actually
/// owns those values should extend this function.
pub(crate) fn app_managed_tags(metadata: &TrackMetadata) -> TrackTags {
    TrackTags {
        bpm: metadata.bpm,
        key: metadata.key.clone(),
        ..TrackTags::default()
    }
}

/// Read the ReplayGain track gain tag, in decibels, when present and valid.
pub(crate) fn read_replaygain_track_gain_db(path: &Path) -> library_core::Result<Option<f64>> {
    codec_replaygain(path).map_err(|e| io_backend(format!("{e}")))
}

/// Read embedded album artwork from an audio file, if present.
pub fn read_artwork(path: &Path) -> library_core::Result<Option<Vec<u8>>> {
    read_file_artwork(path)
        .map_err(|e| io_backend(format!("read artwork {}: {e}", path_label(path))))
}

fn track_metadata_from_file_tags(path: &Path, tags: FileTags) -> TrackMetadata {
    let mut metadata = TrackMetadata {
        title: tags.title,
        artist: tags.artist,
        album: tags.album,
        genre: tags.genre,
        bpm: tags.bpm,
        key: tags.key.as_deref().map(normalize_key_notation),
        duration_ms: tags.duration_ms,
        sample_rate: tags.sample_rate,
        channels: tags.channels,
        bitrate_kbps: tags.bitrate_kbps,
        replaygain_track_gain_db: tags.replaygain_track_gain_db,
        isrc: tags.isrc,
        ..TrackMetadata::default()
    };

    if metadata.title.is_none() {
        metadata.title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
    }

    metadata
}

/// Convert Camelot/Open Key codes to musical notation; pass through other values.
fn normalize_key_notation(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() < 2 {
        return trimmed.to_string();
    }
    let upper = trimmed.to_uppercase();
    let Some(code) = upper.strip_suffix('A').or_else(|| upper.strip_suffix('B')) else {
        return trimmed.to_string();
    };
    let Ok(num) = code.parse::<usize>() else {
        return trimmed.to_string();
    };
    if !(1..=12).contains(&num) {
        return trimmed.to_string();
    }
    // Mixed In Key: A = minor, B = major.
    let minor = upper.ends_with('A');
    library_core::camelot_code_to_musical(num, minor).unwrap_or_else(|| trimmed.to_string())
}

fn io_backend(message: String) -> library_core::LibraryError {
    library_core::LibraryError::Backend {
        backend: "library",
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_camelot_key_to_musical() {
        assert_eq!(
            library_core::camelot_code_to_musical(8, false).as_deref(),
            Some("C")
        );
        assert_eq!(
            library_core::camelot_code_to_musical(8, true).as_deref(),
            Some("Am")
        );
        assert_eq!(normalize_key_notation("8A"), "Am");
        assert_eq!(normalize_key_notation("8B"), "C");
        assert_eq!(normalize_key_notation("F#m"), "F#m");
    }

    #[test]
    fn maps_file_tags_and_falls_back_title() {
        let tags = FileTags {
            artist: Some("Artist".into()),
            key: Some("8A".into()),
            ..FileTags::default()
        };
        let meta = track_metadata_from_file_tags(Path::new("/music/My Song.flac"), tags);
        assert_eq!(meta.artist.as_deref(), Some("Artist"));
        assert_eq!(meta.key.as_deref(), Some("Am"));
        assert_eq!(meta.title.as_deref(), Some("My Song"));
    }

    #[test]
    fn read_tags_missing_file_errors() {
        let err = read_tags(Path::new("/no/such/file.mp3")).unwrap_err();
        assert!(matches!(
            err,
            library_core::LibraryError::Backend {
                backend: "library",
                ..
            }
        ));
    }
}
