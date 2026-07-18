//! Library audio sources (file, stream, …).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::types::{TrackId, TrackMetadata};

/// Local file on disk.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileAudioSource {
    /// Library-stable identifier.
    pub id: TrackId,
    /// File-tag and basic audio metadata.
    pub metadata: TrackMetadata,
    path: PathBuf,
}

impl FileAudioSource {
    /// Create a library file source.
    pub fn new(id: TrackId, path: PathBuf, metadata: TrackMetadata) -> Self {
        Self { id, metadata, path }
    }

    /// Ad-hoc source from a path (id derived from path, empty metadata).
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        Self {
            id: TrackId::new(path.to_string_lossy()),
            metadata: TrackMetadata::default(),
            path,
        }
    }

    /// Filesystem path to the audio file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Known streaming providers (extensible).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamProvider {
    /// Generic HTTP(S) URL.
    Http,
}

impl StreamProvider {
    /// Wire format used in storage.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
        }
    }
}

impl std::str::FromStr for StreamProvider {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "http" => Ok(Self::Http),
            _ => Err(()),
        }
    }
}

/// Remote or service-backed audio.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StreamAudioSource {
    /// Library-stable identifier.
    pub id: TrackId,
    /// File-tag and basic audio metadata.
    pub metadata: TrackMetadata,
    uri: String,
    /// Optional hint for adapter-specific resolution.
    pub provider: Option<StreamProvider>,
}

impl StreamAudioSource {
    /// Create a library stream source.
    pub fn new(
        id: TrackId,
        uri: impl Into<String>,
        metadata: TrackMetadata,
        provider: Option<StreamProvider>,
    ) -> Self {
        Self {
            id,
            metadata,
            uri: uri.into(),
            provider,
        }
    }

    /// Playback or service identifier (URL, `beatport:track:123`, …).
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

/// A track in the library pool — file or stream.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source_type", rename_all = "snake_case")]
pub enum AudioSource {
    /// Local file on disk.
    File(FileAudioSource),
    /// Remote or service-backed audio.
    Stream(StreamAudioSource),
}

impl AudioSource {
    /// Stable identifier within the library.
    pub fn id(&self) -> &TrackId {
        match self {
            Self::File(s) => &s.id,
            Self::Stream(s) => &s.id,
        }
    }

    /// Associated metadata.
    pub fn metadata(&self) -> &TrackMetadata {
        match self {
            Self::File(s) => &s.metadata,
            Self::Stream(s) => &s.metadata,
        }
    }

    /// Mutable metadata (e.g. stamp analysis loudness before engine load).
    pub fn metadata_mut(&mut self) -> &mut TrackMetadata {
        match self {
            Self::File(s) => &mut s.metadata,
            Self::Stream(s) => &mut s.metadata,
        }
    }

    /// Borrow when this is a file source.
    pub fn file(&self) -> Option<&FileAudioSource> {
        match self {
            Self::File(s) => Some(s),
            Self::Stream(_) => None,
        }
    }

    /// Borrow when this is a stream source.
    pub fn stream(&self) -> Option<&StreamAudioSource> {
        match self {
            Self::File(_) => None,
            Self::Stream(s) => Some(s),
        }
    }

    /// Wire format for storage (`file` / `stream`).
    pub fn source_type(&self) -> &'static str {
        match self {
            Self::File(_) => "file",
            Self::Stream(_) => "stream",
        }
    }

    /// Path or URI reference for storage.
    pub fn source_ref(&self) -> String {
        match self {
            Self::File(s) => s.path().to_string_lossy().into_owned(),
            Self::Stream(s) => s.uri().to_string(),
        }
    }
}
