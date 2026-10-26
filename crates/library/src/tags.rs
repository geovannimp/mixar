//! Map codec file tags into library `TrackMetadata`, and embedded artwork into
//! UI-sized thumbnails.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use codec::{
    read_file_artwork_thumbnail, read_file_tags, read_replaygain_track_gain_db as codec_replaygain,
    FileTags, TagWriteReport, TrackTags,
};
use library_core::{path_label, TagWriteBack, TrackMetadata};

/// Longest edge of the artwork thumbnails this crate hands to the UI, in pixels.
///
/// The largest surface showing artwork is the deck info panel at 64 logical px;
/// 256 covers HiDPI tiers and keeps the FRB payload small next to the user's
/// full-resolution cover.
pub const ARTWORK_UI_EDGE_PX: u32 = 256;

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

/// The fields Mixar owns and mirrors: BPM, key, rating, comment, year,
/// track/disc number and colour label.
///
/// Title/artist/album/genre are deliberately **not** passed even though the
/// writer supports them: they are file-derived today (`track_metadata_from_file_tags`
/// falls back to the file name for a missing title), so mirroring the library row
/// would stamp guesses into files that carry no tags. A track editor that
/// actually owns those values should extend this function.
///
/// The rating goes out as one `POPM` frame per writer email on ID3v2, as
/// `RATING` in Vorbis comments and as `rate` in MP4, so other DJ software sees
/// it even though each app reads its own frame. APE has no rating field.
pub(crate) fn app_managed_tags(metadata: &TrackMetadata) -> TrackTags {
    TrackTags {
        bpm: metadata.bpm,
        key: metadata.key.clone(),
        rating: metadata.rating,
        comment: metadata.comment.clone(),
        year: metadata.year,
        track_number: metadata.track_number,
        disc_number: metadata.disc_number,
        color: metadata.color_label.map(|label| label.label().to_string()),
        ..TrackTags::default()
    }
}

/// Merge user-owned metadata from a stored row into freshly read file tags.
///
/// Before the first editor save, a tag that carries a value wins (the file may
/// have been updated elsewhere) and a missing tag must not clear what the
/// editor stored. Once the editor has written the row, the row is
/// authoritative for all six fields, including `None`: otherwise a cleared
/// rating/comment/year/number/colour would be resurrected from its stale tag
/// on the next analysis.
pub(crate) fn merge_user_metadata(
    metadata: &mut TrackMetadata,
    stored: &TrackMetadata,
    stored_is_authoritative: bool,
) {
    if stored_is_authoritative {
        metadata.rating = stored.rating;
        metadata.comment = stored.comment.clone();
        metadata.year = stored.year;
        metadata.track_number = stored.track_number;
        metadata.disc_number = stored.disc_number;
        metadata.color_label = stored.color_label;
        return;
    }
    metadata.rating = metadata.rating.or(stored.rating);
    let comment = metadata.comment.take().filter(|c| !c.trim().is_empty());
    metadata.comment = comment.or_else(|| stored.comment.clone());
    metadata.year = metadata.year.or(stored.year);
    metadata.track_number = metadata.track_number.or(stored.track_number);
    metadata.disc_number = metadata.disc_number.or(stored.disc_number);
    metadata.color_label = metadata.color_label.or(stored.color_label);
}

/// Read the ReplayGain track gain tag, in decibels, when present and valid.
pub(crate) fn read_replaygain_track_gain_db(path: &Path) -> library_core::Result<Option<f64>> {
    codec_replaygain(path).map_err(|e| io_backend(format!("{e}")))
}

/// Read embedded album artwork as a UI thumbnail, if the file has any.
///
/// The cover is re-encoded to a JPEG of at most [`ARTWORK_UI_EDGE_PX`] on the
/// longest edge, so the UI (and the FRB boundary) never carries the user's
/// full-resolution artwork. `None` when the file has no cover or one we cannot
/// decode.
pub fn read_artwork(path: &Path) -> library_core::Result<Option<Vec<u8>>> {
    read_file_artwork_thumbnail(path, ARTWORK_UI_EDGE_PX)
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
        rating: tags.rating,
        comment: tags.comment,
        year: tags.year,
        track_number: tags.track_number,
        disc_number: tags.disc_number,
        color_label: tags.color.as_deref().and_then(|color| color.parse().ok()),
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
///
/// The pass-through is part of the contract, not an accident: anything that is
/// not a Camelot/Open Key code (`1A`..`12B`) is returned **unchanged**, so the
/// caller decides what to do with unknown input. `deck_data.rs` relies on this
/// — a value it accepted earlier survives here verbatim, which is why the edit
/// drawer validates keys against `camelotSlotForKey` before storing them.
pub(crate) fn normalize_key_notation(raw: &str) -> String {
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

    fn full_user_metadata() -> TrackMetadata {
        TrackMetadata {
            rating: Some(5),
            comment: Some("Keep me".into()),
            year: Some(1999),
            track_number: Some(7),
            disc_number: Some(2),
            color_label: Some(library_core::ColorLabel::Aqua),
            ..TrackMetadata::default()
        }
    }

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
            rating: Some(4),
            comment: Some("Peak time".into()),
            year: Some(2024),
            track_number: Some(3),
            disc_number: Some(2),
            color: Some("pink".into()),
            ..FileTags::default()
        };
        let meta = track_metadata_from_file_tags(Path::new("/music/My Song.flac"), tags);
        assert_eq!(meta.artist.as_deref(), Some("Artist"));
        assert_eq!(meta.key.as_deref(), Some("Am"));
        assert_eq!(meta.title.as_deref(), Some("My Song"));
        assert_eq!(meta.rating, Some(4));
        assert_eq!(meta.comment.as_deref(), Some("Peak time"));
        assert_eq!(meta.year, Some(2024));
        assert_eq!(meta.track_number, Some(3));
        assert_eq!(meta.disc_number, Some(2));
        assert_eq!(meta.color_label, Some(library_core::ColorLabel::Pink));

        let unknown_color = track_metadata_from_file_tags(
            Path::new("/music/Other.flac"),
            FileTags {
                color: Some("chartreuse".into()),
                ..FileTags::default()
            },
        );
        assert_eq!(unknown_color.color_label, None);
    }

    #[test]
    fn merge_user_metadata_preserves_stored_values_when_tags_are_missing() {
        let stored = full_user_metadata();
        let mut fresh = TrackMetadata::default();
        merge_user_metadata(&mut fresh, &stored, false);
        assert_eq!(fresh, stored);
    }

    #[test]
    fn app_managed_tags_maps_the_new_fields() {
        let metadata = TrackMetadata {
            bpm: Some(128.0),
            key: Some("Am".into()),
            ..full_user_metadata()
        };
        let tags = app_managed_tags(&metadata);
        assert_eq!(tags.bpm, Some(128.0));
        assert_eq!(tags.key.as_deref(), Some("Am"));
        assert_eq!(tags.rating, Some(5));
        assert_eq!(tags.comment.as_deref(), Some("Keep me"));
        assert_eq!(tags.year, Some(1999));
        assert_eq!(tags.track_number, Some(7));
        assert_eq!(tags.disc_number, Some(2));
        assert_eq!(tags.color.as_deref(), Some("Aqua"));

        let empty = app_managed_tags(&TrackMetadata::default());
        assert_eq!(empty.rating, None);
        assert_eq!(empty.comment, None);
        assert_eq!(empty.year, None);
        assert_eq!(empty.track_number, None);
        assert_eq!(empty.disc_number, None);
        assert_eq!(empty.color, None);
    }

    #[test]
    fn merge_user_metadata_treats_blank_tag_comment_as_missing() {
        let stored = full_user_metadata();
        let mut fresh = TrackMetadata {
            comment: Some("   ".into()),
            ..TrackMetadata::default()
        };
        merge_user_metadata(&mut fresh, &stored, false);
        assert_eq!(fresh.comment.as_deref(), Some("Keep me"));
    }

    #[test]
    fn merge_user_metadata_keeps_cleared_fields_cleared_after_edit() {
        let stored = TrackMetadata::default();
        let mut fresh = full_user_metadata();
        merge_user_metadata(&mut fresh, &stored, true);
        assert_eq!(fresh, stored);
    }

    #[test]
    fn merge_user_metadata_prefers_tag_values() {
        let stored = TrackMetadata {
            rating: Some(5),
            comment: Some("Old".into()),
            year: Some(1999),
            ..TrackMetadata::default()
        };
        let mut fresh = TrackMetadata {
            rating: Some(2),
            comment: Some("New".into()),
            year: Some(2024),
            ..TrackMetadata::default()
        };
        merge_user_metadata(&mut fresh, &stored, false);
        assert_eq!(fresh.rating, Some(2));
        assert_eq!(fresh.comment.as_deref(), Some("New"));
        assert_eq!(fresh.year, Some(2024));
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
