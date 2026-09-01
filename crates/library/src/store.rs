//! SeaORM persistence for the library manager.

use std::path::Path;

use library_core::{
    CollectionEntry, CollectionEntryId, CollectionId, Result, TrackId, TrackMetadata,
};
use sea_orm::sea_query::{Expr, OnConflict, Order};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set, SqliteTransactionMode, TransactionOptions, TransactionTrait,
};

use crate::db::{self, Db};
use crate::entity::{
    collection_entries, collections, tracks, CollectionEntity, CollectionEntryEntity,
    TrackAnalysisEntity, TrackEntity,
};
use crate::model;
use uuid::Uuid;

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
            duration_ms: Set(metadata.duration_ms),
            sample_rate: Set(metadata.sample_rate.map(|v| v as i32)),
            channels: Set(metadata.channels.map(|v| v as i32)),
            bitrate_kbps: Set(metadata.bitrate_kbps.map(|v| v as i32)),
            replaygain_track_gain_db: Set(metadata.replaygain_track_gain_db),
            isrc: Set(metadata.isrc.clone()),
            last_sampler_bank_id: Set(None),
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
                        tracks::Column::DurationMs,
                        tracks::Column::SampleRate,
                        tracks::Column::Channels,
                        tracks::Column::BitrateKbps,
                        tracks::Column::ReplaygainTrackGainDb,
                        tracks::Column::Isrc,
                        tracks::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.db.conn()?.as_connection())
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
            duration_ms: Set(metadata.duration_ms),
            sample_rate: Set(metadata.sample_rate.map(|v| v as i32)),
            channels: Set(metadata.channels.map(|v| v as i32)),
            bitrate_kbps: Set(metadata.bitrate_kbps.map(|v| v as i32)),
            replaygain_track_gain_db: Set(metadata.replaygain_track_gain_db),
            isrc: Set(metadata.isrc.clone()),
            last_sampler_bank_id: Set(None),
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
                        tracks::Column::DurationMs,
                        tracks::Column::SampleRate,
                        tracks::Column::Channels,
                        tracks::Column::BitrateKbps,
                        tracks::Column::ReplaygainTrackGainDb,
                        tracks::Column::Isrc,
                        tracks::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn track_exists(&self, id: &TrackId) -> Result<bool> {
        let count = TrackEntity::find_by_id(id.as_str())
            .count(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(count > 0)
    }

    pub fn get_track(&self, id: &TrackId) -> Result<Option<library_core::AudioSource>> {
        let row = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        row.map(model::track_source).transpose()
    }

    pub fn update_track_isrc(&self, id: &TrackId, isrc: Option<String>, now: &str) -> Result<()> {
        let Some(row) = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?
        else {
            return Ok(());
        };
        let mut active: tracks::ActiveModel = row.into();
        active.isrc = Set(isrc);
        active.updated_at = Set(now.to_string());
        active
            .update(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn find_file_track_by_source_ref(
        &self,
        source_ref: &str,
    ) -> Result<Option<library_core::AudioSource>> {
        let row = TrackEntity::find()
            .filter(tracks::Column::SourceType.eq("file"))
            .filter(tracks::Column::SourceRef.eq(source_ref))
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        row.map(model::track_source).transpose()
    }

    pub fn find_file_sources_under(
        &self,
        root: &str,
        prefix: &str,
    ) -> Result<Vec<library_core::AudioSource>> {
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
            .all(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        rows.into_iter().map(model::track_source).collect()
    }

    pub fn delete_track(&self, id: &TrackId) -> Result<()> {
        TrackEntity::delete_by_id(id.as_str())
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn count_playlist_links(&self, track_id: &TrackId) -> Result<u64> {
        CollectionEntryEntity::find()
            .filter(collection_entries::Column::TrackId.eq(track_id.as_str()))
            .count(self.db.conn()?.as_connection())
            .map_err(db::db_err)
    }

    pub fn insert_folder_collection(
        &self,
        id: &CollectionId,
        name: &str,
        fs_path: &str,
        scan_folder_tree: bool,
    ) -> Result<()> {
        let active = collections::ActiveModel {
            id: Set(id.as_str().to_string()),
            name: Set(name.to_string()),
            collection_type: Set(CollectionTypeWire::Folder.label().to_string()),
            sortable: Set(0),
            scan_folder_tree: Set(i32::from(scan_folder_tree)),
            fs_path: Set(Some(fs_path.to_string())),
        };
        CollectionEntity::insert(active)
            .exec(self.db.conn()?.as_connection())
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
            collection_type: Set(CollectionTypeWire::Playlist.label().to_string()),
            sortable: Set(i32::from(sortable)),
            scan_folder_tree: Set(1),
            fs_path: Set(None),
        };
        CollectionEntity::insert(active)
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn list_collections(&self) -> Result<Vec<library_core::Collection>> {
        let rows = CollectionEntity::find()
            .order_by_asc(collections::Column::Name)
            .all(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        rows.into_iter().map(model::collection).collect()
    }

    pub fn get_collection(&self, id: &CollectionId) -> Result<Option<library_core::Collection>> {
        let row = CollectionEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        row.map(model::collection).transpose()
    }

    pub fn update_collection_name(&self, id: &CollectionId, name: &str) -> Result<bool> {
        let result = CollectionEntity::update_many()
            .col_expr(collections::Column::Name, Expr::value(name))
            .filter(collections::Column::Id.eq(id.as_str()))
            .exec(self.db.conn()?.as_connection())
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
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn delete_collection(&self, id: &CollectionId) -> Result<bool> {
        let result = CollectionEntity::delete_by_id(id.as_str())
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(result.rows_affected > 0)
    }

    pub fn list_playlist_entries(
        &self,
        playlist_id: &CollectionId,
    ) -> Result<Vec<CollectionEntry>> {
        let rows = CollectionEntryEntity::find()
            .filter(collection_entries::Column::CollectionId.eq(playlist_id.as_str()))
            .order_by(
                Expr::cust("CASE WHEN position IS NULL THEN 1 ELSE 0 END"),
                Order::Asc,
            )
            .order_by_asc(collection_entries::Column::Position)
            .order_by_asc(collection_entries::Column::TrackId)
            .all(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(rows
            .into_iter()
            .map(|row| CollectionEntry {
                id: CollectionEntryId::new(row.id),
                collection_id: CollectionId::new(row.collection_id),
                track_id: TrackId::new(row.track_id),
                position: row.position,
            })
            .collect())
    }

    pub fn insert_collection_entry(
        &self,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: Option<i32>,
    ) -> Result<CollectionEntryId> {
        let id = self.insert_collection_entry_on(
            self.db.conn()?.as_connection(),
            collection_id,
            track_id,
            position,
        )?;
        Ok(CollectionEntryId::new(id))
    }

    fn insert_collection_entry_on(
        &self,
        connection: &impl ConnectionTrait,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: Option<i32>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let active = collection_entries::ActiveModel {
            id: Set(id.clone()),
            collection_id: Set(collection_id.as_str().to_string()),
            track_id: Set(track_id.as_str().to_string()),
            position: Set(position),
        };
        CollectionEntryEntity::insert(active)
            .exec(connection)
            .map_err(db::db_err)?;
        Ok(id)
    }

    pub fn upsert_collection_entry(
        &self,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: Option<i32>,
    ) -> Result<CollectionEntryId> {
        let conn = self.db.conn()?;
        // IMMEDIATE: crate membership is a set; serialize writers so two connections
        // cannot both observe absence and insert a second row for the same pair.
        let txn = conn
            .as_connection()
            .begin_with_options(TransactionOptions {
                sqlite_transaction_mode: Some(SqliteTransactionMode::Immediate),
                ..Default::default()
            })
            .map_err(db::db_err)?;
        let result = (|| -> Result<CollectionEntryId> {
            let existing = CollectionEntryEntity::find()
                .filter(collection_entries::Column::CollectionId.eq(collection_id.as_str()))
                .filter(collection_entries::Column::TrackId.eq(track_id.as_str()))
                .one(&txn)
                .map_err(db::db_err)?;
            if let Some(row) = existing {
                let entry_id = CollectionEntryId::new(row.id.clone());
                let mut active: collection_entries::ActiveModel = row.into();
                active.position = Set(position);
                active.update(&txn).map_err(db::db_err)?;
                return Ok(entry_id);
            }
            let id = self.insert_collection_entry_on(&txn, collection_id, track_id, position)?;
            Ok(CollectionEntryId::new(id))
        })();
        match result {
            Ok(entry_id) => {
                txn.commit().map_err(db::db_err)?;
                Ok(entry_id)
            }
            Err(error) => {
                let _ = txn.rollback();
                Err(error)
            }
        }
    }

    pub fn delete_collection_entry_by_id(
        &self,
        collection_id: &CollectionId,
        entry_id: &CollectionEntryId,
    ) -> Result<Option<TrackId>> {
        let row = CollectionEntryEntity::find()
            .filter(collection_entries::Column::CollectionId.eq(collection_id.as_str()))
            .filter(collection_entries::Column::Id.eq(entry_id.as_str()))
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let track_id = TrackId::new(row.track_id.clone());
        CollectionEntryEntity::delete_by_id(row.id)
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(Some(track_id))
    }

    pub fn clear_playlist_positions(&self, collection_id: &CollectionId) -> Result<()> {
        CollectionEntryEntity::update_many()
            .col_expr(
                collection_entries::Column::Position,
                Expr::value(None::<i32>),
            )
            .filter(collection_entries::Column::CollectionId.eq(collection_id.as_str()))
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    pub fn set_collection_entry_position_by_id(
        &self,
        collection_id: &CollectionId,
        entry_id: &CollectionEntryId,
        position: i32,
    ) -> Result<()> {
        CollectionEntryEntity::update_many()
            .col_expr(collection_entries::Column::Position, Expr::value(position))
            .filter(collection_entries::Column::CollectionId.eq(collection_id.as_str()))
            .filter(collection_entries::Column::Id.eq(entry_id.as_str()))
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    /// Keep one entry per track (first in display order).
    pub fn dedupe_playlist_entries(&self, playlist_id: &CollectionId) -> Result<()> {
        let rows = self.list_playlist_entries(playlist_id)?;
        let mut seen = std::collections::HashSet::new();
        let mut duplicate_ids = Vec::new();
        for row in rows {
            if !seen.insert(row.track_id.as_str().to_string()) {
                duplicate_ids.push(row.id.as_str().to_string());
            }
        }
        if duplicate_ids.is_empty() {
            return Ok(());
        }
        CollectionEntryEntity::delete_many()
            .filter(collection_entries::Column::Id.is_in(duplicate_ids))
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn count_track_analysis(&self, track_id: &TrackId) -> Result<u64> {
        TrackAnalysisEntity::find_by_id(track_id.as_str())
            .count(self.db.conn()?.as_connection())
            .map_err(db::db_err)
    }

    #[allow(dead_code)]
    pub fn track_analysis_loudness(&self, track_id: &TrackId) -> Result<Option<f64>> {
        Ok(TrackAnalysisEntity::find_by_id(track_id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?
            .and_then(|analysis| analysis.loudness_lufs))
    }

    /// Track fields needed to persist a manual beat grid.
    pub fn track_beat_grid_context(&self, track_id: &TrackId) -> Result<TrackBeatGridContext> {
        let track = TrackEntity::find_by_id(track_id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?
            .ok_or_else(|| library_core::LibraryError::Backend {
                backend: "library",
                message: format!("track not found: {}", track_id.as_str()),
            })?;
        Ok(TrackBeatGridContext {
            duration_ms: track.duration_ms.unwrap_or(0).max(0),
            key: track.key,
            sample_rate: track.sample_rate.unwrap_or(48_000),
        })
    }

    /// Upsert manual beat-grid analysis and mirror BPM onto the track row.
    pub fn upsert_manual_beat_grid(
        &self,
        track_id: &TrackId,
        bpm: f64,
        beat_grid_json: String,
        ctx: &TrackBeatGridContext,
        analyzed_at: &str,
    ) -> Result<()> {
        use crate::entity::track_analysis;

        let active = track_analysis::ActiveModel {
            track_id: Set(track_id.as_str().to_string()),
            backend: Set("manual".into()),
            backend_version: Set("1".into()),
            analyzed_at: Set(analyzed_at.to_string()),
            bpm: Set(Some(bpm)),
            bpm_confidence: Set(None),
            key: Set(ctx.key.clone()),
            key_confidence: Set(None),
            key_clarity: Set(None),
            grid_stability: Set(Some(1.0)),
            sample_rate: Set(ctx.sample_rate),
            duration_analyzed_ms: Set(ctx.duration_ms),
            loudness_lufs: Set(None),
            beat_grid_json: Set(Some(beat_grid_json)),
        };

        TrackAnalysisEntity::insert(active)
            .on_conflict(
                OnConflict::column(track_analysis::Column::TrackId)
                    .update_columns([
                        track_analysis::Column::Backend,
                        track_analysis::Column::BackendVersion,
                        track_analysis::Column::AnalyzedAt,
                        track_analysis::Column::Bpm,
                        track_analysis::Column::GridStability,
                        track_analysis::Column::BeatGridJson,
                    ])
                    .to_owned(),
            )
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;

        TrackEntity::update_many()
            .col_expr(tracks::Column::Bpm, Expr::value(bpm))
            .filter(tracks::Column::Id.eq(track_id.as_str()))
            .exec(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(())
    }
}

/// Snapshot of track fields used when writing a manual beat grid.
pub struct TrackBeatGridContext {
    pub duration_ms: i32,
    pub key: Option<String>,
    pub sample_rate: i32,
}

enum CollectionTypeWire {
    Folder,
    Playlist,
}

impl CollectionTypeWire {
    fn label(self) -> &'static str {
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
            duration_ms: Some(12_500),
            ..TrackMetadata::default()
        };
        store
            .upsert_file_track(&id, Path::new("/music/a.wav"), &meta, "1")
            .unwrap();
        let fetched = store.get_track(&id).unwrap().unwrap();
        assert_eq!(fetched.metadata().title.as_deref(), Some("a"));
        assert_eq!(fetched.metadata().duration_ms, Some(12_500));
    }
}
