//! Convert SeaORM models to library-core types.

use std::path::PathBuf;

use library_core::{
    AudioSource, Collection, CollectionConfig, CollectionId, CollectionType, FileAudioSource,
    LibraryError, Result, StreamAudioSource, StreamProvider, TrackId, TrackMetadata,
};

use crate::entity::{collections, tracks};

pub fn track_metadata(model: &tracks::Model) -> TrackMetadata {
    TrackMetadata {
        title: model.title.clone(),
        artist: model.artist.clone(),
        album: model.album.clone(),
        genre: model.genre.clone(),
        bpm: model.bpm,
        key: model.key.clone(),
        duration_ms: model.duration_ms,
        sample_rate: model.sample_rate.map(|v| v as u32),
        channels: model.channels.map(|v| v as u16),
        bitrate_kbps: model.bitrate_kbps.map(|v| v as u32),
        replaygain_track_gain_db: model.replaygain_track_gain_db,
        isrc: model.isrc.clone(),
        loudness_lufs: None,
    }
}

pub fn track_source(model: tracks::Model) -> Result<AudioSource> {
    let metadata = track_metadata(&model);
    let id = TrackId::new(model.id);

    match model.source_type.as_str() {
        "file" => Ok(AudioSource::File(FileAudioSource::new(
            id,
            PathBuf::from(model.source_ref),
            metadata,
        ))),
        "stream" => {
            let provider_raw = model.provider.clone();
            let provider = provider_raw
                .as_deref()
                .map(|s| s.parse::<StreamProvider>())
                .transpose()
                .map_err(|_| LibraryError::Backend {
                    backend: "library",
                    message: format!("unknown stream provider: {provider_raw:?}"),
                })?;
            Ok(AudioSource::Stream(StreamAudioSource::new(
                id,
                model.source_ref,
                metadata,
                provider,
            )))
        }
        other => Err(LibraryError::Backend {
            backend: "library",
            message: format!("unknown source_type: {other}"),
        }),
    }
}

pub fn collection(model: collections::Model) -> Result<Collection> {
    let collection_type = model
        .collection_type
        .parse::<CollectionType>()
        .map_err(|_| LibraryError::Backend {
            backend: "library",
            message: format!("unknown collection_type: {}", model.collection_type),
        })?;

    let config = match collection_type {
        CollectionType::Folder => {
            let Some(fs_path) = model.fs_path else {
                return Err(LibraryError::Backend {
                    backend: "library",
                    message: "folder collection missing fs_path".into(),
                });
            };
            CollectionConfig::Folder {
                fs_path: PathBuf::from(fs_path),
                scan_folder_tree: model.scan_folder_tree != 0,
            }
        }
        CollectionType::Playlist => CollectionConfig::Playlist {
            sortable: model.sortable != 0,
        },
    };

    Ok(Collection {
        id: CollectionId::new(model.id),
        name: model.name,
        config,
    })
}
