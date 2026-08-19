//! Shared library types.

use std::path::PathBuf;

use analyzer_core::AnalysisDurationMode;
use serde::{Deserialize, Serialize};

/// Stable track identifier within a library.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub String);

impl TrackId {
    /// Create a new track id.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the id as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TrackId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for TrackId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl std::fmt::Display for TrackId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Stable collection identifier within a library.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollectionId(pub String);

impl CollectionId {
    /// Create a new collection id.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the id as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for CollectionId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for CollectionId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl std::fmt::Display for CollectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// File-tag and basic audio metadata for a track.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TrackMetadata {
    /// Track title.
    pub title: Option<String>,
    /// Primary artist.
    pub artist: Option<String>,
    /// Album name.
    pub album: Option<String>,
    /// Genre.
    pub genre: Option<String>,
    /// Beats per minute.
    pub bpm: Option<f64>,
    /// Musical key (e.g. `"Am"`, `"F#m"`, `"Bb"`). Always musical notation — never Camelot / Open Key codes.
    pub key: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: Option<i32>,
    /// Sample rate in Hz.
    pub sample_rate: Option<u32>,
    /// Channel count.
    pub channels: Option<u16>,
    /// Bitrate in kbps, when known.
    pub bitrate_kbps: Option<u32>,
    /// ReplayGain track gain from tags (dB), when present (`REPLAYGAIN_TRACK_GAIN`).
    pub replaygain_track_gain_db: Option<f64>,
    /// Measured / effective loudness (LUFS) for normalization when known (e.g. analysis).
    pub loudness_lufs: Option<f64>,
}

/// Kind of collection in the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionType {
    /// Real disk directory (`fs_path` required).
    Folder,
    /// User-defined track list (`sortable` controls order).
    Playlist,
}

impl CollectionType {
    /// Wire format used in storage (`folder` / `playlist`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Playlist => "playlist",
        }
    }
}

impl std::str::FromStr for CollectionType {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "folder" => Ok(Self::Folder),
            "playlist" => Ok(Self::Playlist),
            _ => Err(()),
        }
    }
}

/// Type-specific collection settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CollectionConfig {
    /// Real disk directory.
    Folder {
        /// Absolute path to a real directory on disk.
        fs_path: PathBuf,
        /// When true, folder scans recurse into subdirectories.
        #[serde(default = "default_scan_folder_tree")]
        scan_folder_tree: bool,
    },
    /// User-defined track list.
    Playlist {
        /// Ordered vs set (crate-like).
        sortable: bool,
    },
}

fn default_scan_folder_tree() -> bool {
    true
}

impl CollectionConfig {
    /// Folder or playlist.
    pub fn collection_type(&self) -> CollectionType {
        match self {
            Self::Folder { .. } => CollectionType::Folder,
            Self::Playlist { .. } => CollectionType::Playlist,
        }
    }
}

/// A collection: either a disk folder or a playlist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    /// Library-stable identifier.
    pub id: CollectionId,
    /// Display name.
    pub name: String,
    /// Folder or playlist configuration.
    pub config: CollectionConfig,
}

impl Collection {
    /// Folder or playlist.
    pub fn collection_type(&self) -> CollectionType {
        self.config.collection_type()
    }

    /// Folder only: absolute path on disk.
    pub fn fs_path(&self) -> Option<&std::path::Path> {
        match &self.config {
            CollectionConfig::Folder { fs_path, .. } => Some(fs_path),
            CollectionConfig::Playlist { .. } => None,
        }
    }

    /// Playlist only: whether track order is meaningful.
    pub fn sortable(&self) -> bool {
        match &self.config {
            CollectionConfig::Folder { .. } => false,
            CollectionConfig::Playlist { sortable } => *sortable,
        }
    }
}

/// Many-to-many join: playlist ↔ track only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionTrack {
    /// Playlist collection id.
    pub collection_id: CollectionId,
    /// Track id.
    pub track_id: TrackId,
    /// Set when the playlist is `sortable`; `None` otherwise.
    pub position: Option<i32>,
}

/// Library manager configuration (reserved for library-wide options).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryConfig {}

/// Parameters for creating a collection (folder or playlist).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewCollection {
    /// Display name. When `None` for a folder, derived from `fs_path`.
    pub name: Option<String>,
    /// Folder or playlist configuration.
    pub config: CollectionConfig,
}

impl NewCollection {
    /// Create a disk-folder collection.
    pub fn folder(path: impl AsRef<std::path::Path>) -> Self {
        Self {
            name: None,
            config: CollectionConfig::Folder {
                fs_path: path.as_ref().to_path_buf(),
                scan_folder_tree: true,
            },
        }
    }

    /// Create a disk-folder collection with an explicit display name.
    pub fn folder_named(path: impl AsRef<std::path::Path>, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            config: CollectionConfig::Folder {
                fs_path: path.as_ref().to_path_buf(),
                scan_folder_tree: true,
            },
        }
    }

    /// Create a playlist collection.
    pub fn playlist(name: impl Into<String>, sortable: bool) -> Self {
        Self {
            name: Some(name.into()),
            config: CollectionConfig::Playlist { sortable },
        }
    }
}

/// Partial update for collection configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CollectionConfigUpdate {
    /// Playlist only: ordered vs set.
    Playlist { sortable: bool },
}

/// Partial update for an existing collection.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateCollection {
    /// New display name.
    pub name: Option<String>,
    /// Type-specific settings to change.
    pub config: Option<CollectionConfigUpdate>,
}

impl UpdateCollection {
    /// Update the display name.
    pub fn name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            config: None,
        }
    }

    /// Update whether a playlist is ordered.
    pub fn sortable(sortable: bool) -> Self {
        Self {
            name: None,
            config: Some(CollectionConfigUpdate::Playlist { sortable }),
        }
    }
}

/// Options for [`WritableLibrary::analyze_track`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeTrackOptions {
    /// When true, DSP analysis results replace BPM/key from file tags.
    /// When false, existing tag values are kept when present; analysis fills
    /// missing fields only.
    pub force: bool,
    /// How much of the track to analyze.
    pub analysis_duration: AnalysisDurationMode,
}

impl AnalyzeTrackOptions {
    /// Prefer DSP analysis over file tags for BPM/key (and beat grid when available).
    pub fn force() -> Self {
        Self {
            force: true,
            analysis_duration: AnalysisDurationMode::Complete,
        }
    }
}

/// Summary of a sync/scan operation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanReport {
    /// Newly imported tracks.
    pub added: usize,
    /// Existing tracks whose metadata was refreshed.
    pub updated: usize,
    /// Paths skipped (unsupported or unchanged policy).
    pub skipped: usize,
    /// Paths that failed to import.
    pub failed: usize,
    /// Per-path failure messages.
    pub errors: Vec<String>,
}
