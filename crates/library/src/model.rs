//! Convert SeaORM models to library-core types.

use std::path::PathBuf;

use library_core::{
    AudioSource, Collection, CollectionConfig, CollectionId, CollectionType, FileAudioSource,
    LibraryError, Result, StreamAudioSource, StreamProvider, TrackId, TrackMetadata,
};

use crate::entity::{collections, tracks};

fn non_zero_u16(value: i32) -> Option<u16> {
    u16::try_from(value).ok().filter(|v| *v > 0)
}

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
        // Columns are `INTEGER`; a value outside the model's width is treated
        // as absent rather than clamped into a plausible-looking wrong value.
        rating: model
            .rating
            .and_then(|v| u8::try_from(v).ok())
            .filter(|v| (1..=5).contains(v)),
        comment: model.comment.clone(),
        year: model.year.and_then(non_zero_u16),
        track_number: model.track_number.and_then(non_zero_u16),
        disc_number: model.disc_number.and_then(non_zero_u16),
        color_label: model.color_label.as_deref().and_then(|s| s.parse().ok()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ActiveModelTrait, Set};

    #[test]
    fn track_metadata_normalizes_user_owned_columns() {
        let db = crate::db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();

        let invalid = tracks::ActiveModel {
            id: Set("bad".into()),
            source_type: Set("file".into()),
            source_ref: Set("/music/bad.wav".into()),
            rating: Set(Some(9)),
            year: Set(Some(0)),
            track_number: Set(Some(-1)),
            disc_number: Set(Some(70_000)),
            color_label: Set(Some("mauve".into())),
            added_at: Set("now".into()),
            updated_at: Set("now".into()),
            ..Default::default()
        }
        .insert(conn.as_connection())
        .unwrap();
        let metadata = track_metadata(&invalid);
        assert_eq!(metadata.rating, None);
        assert_eq!(metadata.year, None);
        assert_eq!(metadata.track_number, None);
        assert_eq!(metadata.disc_number, None);
        assert_eq!(metadata.color_label, None);

        let valid = tracks::ActiveModel {
            id: Set("good".into()),
            source_type: Set("file".into()),
            source_ref: Set("/music/good.wav".into()),
            rating: Set(Some(4)),
            year: Set(Some(2024)),
            track_number: Set(Some(3)),
            disc_number: Set(Some(2)),
            color_label: Set(Some("Aqua".into())),
            added_at: Set("now".into()),
            updated_at: Set("now".into()),
            ..Default::default()
        }
        .insert(conn.as_connection())
        .unwrap();
        let metadata = track_metadata(&valid);
        assert_eq!(metadata.rating, Some(4));
        assert_eq!(metadata.year, Some(2024));
        assert_eq!(metadata.track_number, Some(3));
        assert_eq!(metadata.disc_number, Some(2));
        assert_eq!(metadata.color_label, Some(library_core::ColorLabel::Aqua));
    }
}
