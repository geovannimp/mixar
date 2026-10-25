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
mod error;
mod export;
mod key_format;
mod path;
mod smart;
mod source;
mod traits;
mod types;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{LoadableAudio, LoadedAudio, SpectralPeak};
pub use error::{path_label, LibraryError, Result};
pub use export::{
    ExportArtwork, ExportBeatGrid, ExportCue, ExportOptions, ExportReport, ExportSource,
    Exportable, TrackExportData, EXPORT_ARTWORK_EDGE_PX, EXPORT_WAVEFORM_ENTRIES_PER_SECOND,
};
pub use key_format::{
    camelot_code_to_musical, camelot_to_musical, canonical_key, format_key, harmonic_family,
    musical_to_camelot, KeyDisplayMode, MAJOR_KEYS, MINOR_KEYS,
};
pub use path::path_under_folder;
pub use smart::{
    DeckReference, SmartCondition, SmartField, SmartMatch, SmartOp, SmartPlaylistRules, SmartRule,
    SmartRuleGroup, SmartSort, SmartSortField, SmartTrackFacts, SMART_PLAYLIST_RULES_VERSION,
};
pub use source::{AudioSource, FileAudioSource, StreamAudioSource, StreamProvider};
pub use traits::{Library, Migratable, WritableLibrary};
pub use types::{
    AnalyzeTrackOptions, Collection, CollectionConfig, CollectionConfigUpdate, CollectionEntry,
    CollectionEntryId, CollectionId, CollectionType, ColorLabel, ImportedTrack, LibraryConfig,
    MigrateOptions, MigrateReport, NewCollection, ScanReport, TagWriteBack, TrackId, TrackMetadata,
    UpdateCollection,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
        assert_eq!(
            "smart_playlist".parse::<CollectionType>().unwrap(),
            CollectionType::SmartPlaylist
        );
        assert!("nonsense".parse::<CollectionType>().is_err());
        assert_eq!(CollectionType::Folder.as_str(), "folder");
        assert_eq!(CollectionType::Playlist.as_str(), "playlist");
        assert_eq!(CollectionType::SmartPlaylist.as_str(), "smart_playlist");
    }

    #[test]
    fn smart_playlist_collection_carries_its_rules() {
        let config = CollectionConfig::SmartPlaylist {
            rules_json: r#"{"version":1}"#.into(),
        };
        let collection = Collection {
            id: CollectionId::new("smart:1"),
            name: "Peak".into(),
            config: config.clone(),
        };
        assert_eq!(collection.collection_type(), CollectionType::SmartPlaylist);
        assert_eq!(collection.smart_rules_json(), Some(r#"{"version":1}"#));
        // A folder-only field and playlist-only field are both absent.
        assert!(collection.fs_path().is_none());
        assert!(!collection.sortable());

        // The document is stored verbatim and tagged like the other kinds.
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.starts_with(r#"{"kind":"smart_playlist""#), "{json}");
        assert_eq!(
            serde_json::from_str::<CollectionConfig>(&json).unwrap(),
            config
        );
    }

    #[test]
    fn collection_id_display() {
        let id = CollectionId::new("folder:/music");
        assert_eq!(id.to_string(), "folder:/music");
        let _ = PathBuf::from("/music");
    }
}
