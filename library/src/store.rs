//! SeaORM persistence for the library manager.

use std::path::Path;

use library_core::{CollectionId, Result, TrackId, TrackMetadata};
use sea_orm::sea_query::{Expr, OnConflict, Order};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};

use crate::db::{self, Db};
use crate::entity::{
    collection_tracks, collections, tracks, CollectionEntity, CollectionTrackEntity,
    TrackAnalysisEntity, TrackEntity,
};
use crate::model;

pub struct Store<'a> {
    db: &'a Db,
}

impl<'a> Store<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    pub fn upsert_file_track(
        &self,
        id: &TrackId,
        path: &Path,
        metadata: &TrackMetadata,
        now: &str,
    ) -> Result<()> {
        let source_ref = path.to_string_lossy().into_owned();
        let active = tracks::ActiveModel {
            id: Set(id.as_str().to_string()),
            source_type: Set("file".to_string()),
            source_ref: Set(source_ref),
            provider: Set(None),
            title: Set(metadata.title.clone()),
            artist: Set(metadata.artist.clone()),
            album: Set(metadata.album.clone()),
            genre: Set(metadata.genre.clone()),
            bpm: Set(metadata.bpm),
            key: Set(metadata.key.clone()),
            duration_secs: Set(metadata.duration_secs),
            sample_rate: Set(metadata.sample_rate.map(|v| v as i32)),
            channels: Set(metadata.channels.map(|v| v as i32)),
            bitrate_kbps: Set(metadata.bitrate_kbps.map(|v| v as i32)),
            added_at: Set(now.to_string()),
            updated_at: Set(now.to_string()),
        };

        TrackEntity::insert(active)
            .on_conflict(
                OnConflict::column(tracks::Column::Id)
                    .update_columns([
                        tracks::Column::SourceType,
                        tracks::Column::SourceRef,
                        tracks::Column::Provider,
                        tracks::Column::Title,
                        tracks::Column::Artist,
                        tracks::Column::Album,
                        tracks::Column::Genre,
                        tracks::Column::Bpm,
                        tracks::Column::Key,
                        tracks::Column::DurationSecs,
                        tracks::Column::SampleRate,
                        tracks::Column::Channels,
                        tracks::Column::BitrateKbps,
                        tracks::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn upsert_stream_track(
        &self,
        id: &TrackId,
        uri: &str,
        metadata: &TrackMetadata,
        provider: Option<&str>,
        now: &str,
    ) -> Result<()> {
        let active = tracks::ActiveModel {
            id: Set(id.as_str().to_string()),
            source_type: Set("stream".to_string()),
            source_ref: Set(uri.to_string()),
            provider: Set(provider.map(str::to_string)),
            title: Set(metadata.title.clone()),
            artist: Set(metadata.artist.clone()),
            album: Set(metadata.album.clone()),
            genre: Set(metadata.genre.clone()),
            bpm: Set(metadata.bpm),
            key: Set(metadata.key.clone()),
            duration_secs: Set(metadata.duration_secs),
            sample_rate: Set(metadata.sample_rate.map(|v| v as i32)),
            channels: Set(metadata.channels.map(|v| v as i32)),
            bitrate_kbps: Set(metadata.bitrate_kbps.map(|v| v as i32)),
            added_at: Set(now.to_string()),
            updated_at: Set(now.to_string()),
        };

        TrackEntity::insert(active)
            .on_conflict(
                OnConflict::column(tracks::Column::Id)
                    .update_columns([
                        tracks::Column::SourceType,
                        tracks::Column::SourceRef,
                        tracks::Column::Provider,
                        tracks::Column::Title,
                        tracks::Column::Artist,
                        tracks::Column::Album,
                        tracks::Column::Genre,
                        tracks::Column::Bpm,
                        tracks::Column::Key,
                        tracks::Column::DurationSecs,
                        tracks::Column::SampleRate,
                        tracks::Column::Channels,
                        tracks::Column::BitrateKbps,
                        tracks::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn get_track(&self, id: &TrackId) -> Result<Option<library_core::LibrarySource>> {
        let row = TrackEntity::find_by_id(id.as_str())
            .one(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        row.map(model::track_source).transpose()
    }

    pub fn find_file_track_by_source_ref(
        &self,
        source_ref: &str,
    ) -> Result<Option<library_core::LibrarySource>> {
        let row = TrackEntity::find()
            .filter(tracks::Column::SourceType.eq("file"))
            .filter(tracks::Column::SourceRef.eq(source_ref))
            .one(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        row.map(model::track_source).transpose()
    }

    pub fn find_file_sources_under(
        &self,
        root: &str,
        prefix: &str,
    ) -> Result<Vec<library_core::LibrarySource>> {
        let rows = TrackEntity::find()
            .filter(tracks::Column::SourceType.eq("file"))
            .filter(
                Condition::any()
                    .add(tracks::Column::SourceRef.eq(root))
                    .add(Expr::cust_with_values(
                        "source_ref LIKE ? ESCAPE '\\'",
                        [sea_orm::Value::from(prefix.to_string())],
                    )),
            )
            .order_by_asc(tracks::Column::SourceRef)
            .all(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        rows.into_iter().map(model::track_source).collect()
    }

    pub fn delete_track(&self, id: &TrackId) -> Result<()> {
        TrackEntity::delete_by_id(id.as_str())
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn count_playlist_links(&self, track_id: &TrackId) -> Result<u64> {
        CollectionTrackEntity::find()
            .filter(collection_tracks::Column::TrackId.eq(track_id.as_str()))
            .count(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)
    }

    pub fn insert_folder_collection(
        &self,
        id: &CollectionId,
        name: &str,
        fs_path: &str,
    ) -> Result<()> {
        let active = collections::ActiveModel {
            id: Set(id.as_str().to_string()),
            name: Set(name.to_string()),
            collection_type: Set(CollectionTypeWire::Folder.as_str().to_string()),
            sortable: Set(0),
            fs_path: Set(Some(fs_path.to_string())),
        };
        CollectionEntity::insert(active)
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn insert_playlist_collection(
        &self,
        id: &CollectionId,
        name: &str,
        sortable: bool,
    ) -> Result<()> {
        let active = collections::ActiveModel {
            id: Set(id.as_str().to_string()),
            name: Set(name.to_string()),
            collection_type: Set(CollectionTypeWire::Playlist.as_str().to_string()),
            sortable: Set(i32::from(sortable)),
            fs_path: Set(None),
        };
        CollectionEntity::insert(active)
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn list_collections(&self) -> Result<Vec<library_core::Collection>> {
        let rows = CollectionEntity::find()
            .order_by_asc(collections::Column::Name)
            .all(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        rows.into_iter().map(model::collection).collect()
    }

    pub fn get_collection(&self, id: &CollectionId) -> Result<Option<library_core::Collection>> {
        let row = CollectionEntity::find_by_id(id.as_str())
            .one(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        row.map(model::collection).transpose()
    }

    pub fn update_collection_name(&self, id: &CollectionId, name: &str) -> Result<bool> {
        let result = CollectionEntity::update_many()
            .col_expr(collections::Column::Name, Expr::value(name))
            .filter(collections::Column::Id.eq(id.as_str()))
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(result.rows_affected > 0)
    }

    pub fn update_collection_sortable(&self, id: &CollectionId, sortable: bool) -> Result<()> {
        CollectionEntity::update_many()
            .col_expr(
                collections::Column::Sortable,
                Expr::value(i32::from(sortable)),
            )
            .filter(collections::Column::Id.eq(id.as_str()))
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn delete_collection(&self, id: &CollectionId) -> Result<bool> {
        let result = CollectionEntity::delete_by_id(id.as_str())
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(result.rows_affected > 0)
    }

    pub fn playlist_track_ids(&self, playlist_id: &CollectionId) -> Result<Vec<TrackId>> {
        let rows = CollectionTrackEntity::find()
            .filter(collection_tracks::Column::CollectionId.eq(playlist_id.as_str()))
            .order_by(
                Expr::cust("CASE WHEN position IS NULL THEN 1 ELSE 0 END"),
                Order::Asc,
            )
            .order_by_asc(collection_tracks::Column::Position)
            .order_by_asc(collection_tracks::Column::TrackId)
            .all(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(rows
            .into_iter()
            .map(|row| TrackId::new(row.track_id))
            .collect())
    }

    pub fn upsert_collection_track(
        &self,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: Option<i32>,
    ) -> Result<()> {
        let active = collection_tracks::ActiveModel {
            collection_id: Set(collection_id.as_str().to_string()),
            track_id: Set(track_id.as_str().to_string()),
            position: Set(position),
        };
        CollectionTrackEntity::insert(active)
            .on_conflict(
                OnConflict::columns([
                    collection_tracks::Column::CollectionId,
                    collection_tracks::Column::TrackId,
                ])
                .update_columns([collection_tracks::Column::Position])
                .to_owned(),
            )
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn delete_collection_track(
        &self,
        collection_id: &CollectionId,
        track_id: &TrackId,
    ) -> Result<bool> {
        let result = CollectionTrackEntity::delete_many()
            .filter(collection_tracks::Column::CollectionId.eq(collection_id.as_str()))
            .filter(collection_tracks::Column::TrackId.eq(track_id.as_str()))
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(result.rows_affected > 0)
    }

    pub fn clear_playlist_positions(&self, collection_id: &CollectionId) -> Result<()> {
        CollectionTrackEntity::update_many()
            .col_expr(
                collection_tracks::Column::Position,
                Expr::value(None::<i32>),
            )
            .filter(collection_tracks::Column::CollectionId.eq(collection_id.as_str()))
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn set_collection_track_position(
        &self,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: i32,
    ) -> Result<()> {
        CollectionTrackEntity::update_many()
            .col_expr(collection_tracks::Column::Position, Expr::value(position))
            .filter(collection_tracks::Column::CollectionId.eq(collection_id.as_str()))
            .filter(collection_tracks::Column::TrackId.eq(track_id.as_str()))
            .exec(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn count_track_analysis(&self, track_id: &TrackId) -> Result<u64> {
        TrackAnalysisEntity::find_by_id(track_id.as_str())
            .count(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)
    }

    pub fn playlist_track_ids_by_track_id(
        &self,
        playlist_id: &CollectionId,
    ) -> Result<Vec<TrackId>> {
        let rows = CollectionTrackEntity::find()
            .filter(collection_tracks::Column::CollectionId.eq(playlist_id.as_str()))
            .order_by_asc(collection_tracks::Column::TrackId)
            .all(&*self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(rows
            .into_iter()
            .map(|row| TrackId::new(row.track_id))
            .collect())
    }
}

enum CollectionTypeWire {
    Folder,
    Playlist,
}

impl CollectionTypeWire {
    fn as_str(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::Playlist => "playlist",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn upsert_and_get_file_track() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/a.wav");
        let meta = TrackMetadata {
            title: Some("a".into()),
            ..TrackMetadata::default()
        };
        store
            .upsert_file_track(&id, Path::new("/music/a.wav"), &meta, "1")
            .unwrap();
        let fetched = store.get_track(&id).unwrap().unwrap();
        assert_eq!(fetched.metadata().title.as_deref(), Some("a"));
    }
}
