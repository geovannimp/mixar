//! Library traits and shared types for rust-dj-engine.
//!
//! Backends implement [`Library`] and [`WritableLibrary`]. The canonical manager
//! lives in `library`; third-party adapters live in `library-adapters`.
//!
//! [`LibrarySource`] implements [`AudioSource`] so library entries load directly
//! into the engine: `engine.load_track(0, Arc::new(source.load()?), 0.0)?`.

mod audio_extensions;
mod audio_source;
mod error;
mod source;
mod traits;
mod types;

pub use analyzer_core::AnalysisDurationMode;
pub use audio_core::{AudioSource, LoadedAudio};
pub use audio_extensions::{
    is_supported_audio_extension, is_supported_audio_path, SUPPORTED_AUDIO_EXTENSIONS,
};
pub use error::{LibraryError, Result};
pub use source::{FileAudioSource, LibrarySource, StreamAudioSource, StreamProvider};
pub use traits::{Library, WritableLibrary};
pub use types::{
    AnalyzeTrackOptions, Collection, CollectionConfig, CollectionConfigUpdate, CollectionId,
    CollectionTrack, CollectionType, LibraryConfig, NewCollection, ScanReport, TrackId,
    TrackMetadata, UpdateCollection,
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
        assert_eq!(CollectionType::Folder.as_str(), "folder");
    }

    #[test]
    fn library_config_defaults() {
        let cfg = LibraryConfig::default();
        assert!(cfg.scan_folder_tree);
    }

    #[test]
    fn collection_id_display() {
        let id = CollectionId::new("folder:/music");
        assert_eq!(id.to_string(), "folder:/music");
        let _ = PathBuf::from("/music");
    }
}
