//! Library traits and shared types for Mixar.
//!
//! Backends implement [`Library`] and [`WritableLibrary`]. The canonical manager
//! lives in `library`; third-party adapters live in `library-adapters`.
//!
//! [`Exportable`] formats write the manager's collections out to a foreign
//! layout; they read through [`ExportSource`] (the `library` crate's manager
//! implements it) so format code never touches the canonical store.
//!
//! [`AudioSource`] implements [`LoadableAudio`] so library entries load directly
//! into the engine: `engine.load_track(0, AudioSource::File(FileAudioSource::from_path(...)))?`.

mod audio_source;
mod beat_grid;
mod error;
mod export;
mod key_format;
mod path;
mod source;
mod traits;
mod types;

pub use analyzer_core::{AnalysisDurationMode, DEFAULT_CUE_THRESHOLD_DB};

/// Bounds for the cue-detection silence threshold, in dBFS (Settings → Analysis).
pub const MIN_CUE_THRESHOLD_DB: f32 = -90.0;
pub const MAX_CUE_THRESHOLD_DB: f32 = -30.0;
pub use audio_core::{LoadableAudio, LoadedAudio, SpectralPeak};
pub use beat_grid::{
    align_segmented_bpm, from_analysis_json, from_constant, from_legacy_json, is_family_mismatch,
    BeatSegment, CanonicalBeatGrid, GridError, SOURCE_ANALYSIS, SOURCE_IMPORT, SOURCE_MANUAL,
};
pub use error::{path_label, LibraryError, Result};
pub use export::{
    ExportArtwork, ExportBeatGrid, ExportCue, ExportOptions, ExportReport, ExportSource,
    Exportable, TrackExportData, EXPORT_ARTWORK_EDGE_PX, EXPORT_WAVEFORM_ENTRIES_PER_SECOND,
};
pub use key_format::{
    camelot_code_to_musical, camelot_to_musical, format_key, musical_to_camelot,
    normalize_key_notation, KeyDisplayMode, MAJOR_KEYS, MINOR_KEYS,
};
pub use path::path_under_folder;
pub use source::{
    AudioSource, FileAudioSource, StreamAudioSource, StreamProvider, SOURCE_TYPE_FILE,
    SOURCE_TYPE_STREAM,
};
pub use traits::{Library, Migratable, WritableLibrary};
pub use types::{
    AnalyzeTrackOptions, Collection, CollectionConfig, CollectionConfigUpdate, CollectionEntry,
    CollectionEntryId, CollectionId, CollectionType, ColorLabel, CueSource, HotCueImport,
    ImportedTrack, LibraryConfig, LoopImport, MigrateOptions, MigrateReport, NewCollection,
    ScanReport, TagWriteBack, TrackCue, TrackId, TrackMetadata, UpdateCollection,
};

/// Seconds since the Unix epoch, as the decimal string the database stores.
pub fn unix_secs_string() -> String {
    unix_now().as_secs().to_string()
}

/// Milliseconds since the Unix epoch, as the decimal string the database stores.
pub fn unix_millis_string() -> String {
    unix_now().as_millis().to_string()
}

/// Time since the Unix epoch, or zero if the clock is before it.
pub fn unix_now() -> std::time::Duration {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn timestamp_helpers_are_decimal_strings() {
        let secs: u64 = unix_secs_string().parse().expect("seconds parse");
        let millis: u64 = unix_millis_string().parse().expect("millis parse");
        assert!(secs > 1_700_000_000, "seconds since the epoch");
        assert!(millis >= secs * 1000, "millis are not smaller than seconds");
    }

    #[test]
    fn track_id_display_and_from() {
        let id = TrackId::from("track-1");
        assert_eq!(id.as_str(), "track-1");
        assert_eq!(id.to_string(), "track-1");
    }

    #[test]
    fn collection_type_round_trip() {
        assert_eq!(
            "folder".parse::<CollectionType>().unwrap(),
            CollectionType::Folder
        );
        assert_eq!(
            "playlist".parse::<CollectionType>().unwrap(),
            CollectionType::Playlist
        );
        assert_eq!(CollectionType::Folder.as_str(), "folder");
    }

    #[test]
    fn collection_id_display() {
        let id = CollectionId::new("folder:/music");
        assert_eq!(id.to_string(), "folder:/music");
        let _ = PathBuf::from("/music");
    }
}
