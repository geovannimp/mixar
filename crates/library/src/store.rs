//! SeaORM persistence for the library manager.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use library_core::{
    CollectionEntry, CollectionEntryId, CollectionId, Result, TrackId, TrackMetadata,
};
use sea_orm::sea_query::{Expr, OnConflict, Order};
#[cfg(test)]
use sea_orm::Statement;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, SqliteTransactionMode, TransactionOptions, TransactionTrait,
};

use crate::db::{self, Db};
use crate::entity::{
    collection_entries, collections, tracks, CollectionEntity, CollectionEntryEntity,
    TrackAnalysisEntity, TrackEntity,
};
use crate::model;
use uuid::Uuid;

/// SQLite caps host parameters per statement; bind large id sets in chunks.
const TRACK_ID_CHUNK: usize = 500;

fn track_id_strings(ids: &[TrackId]) -> Vec<String> {
    ids.iter().map(|id| id.as_str().to_string()).collect()
}

/// Assign the user-editable metadata columns on an insert/upsert model.
fn apply_metadata_columns(active: &mut tracks::ActiveModel, metadata: &TrackMetadata) {
    active.title = Set(metadata.title.clone());
    active.artist = Set(metadata.artist.clone());
    active.album = Set(metadata.album.clone());
    active.genre = Set(metadata.genre.clone());
    active.bpm = Set(metadata.bpm);
    active.key = Set(metadata.key.clone());
    active.duration_ms = Set(metadata.duration_ms);
    active.sample_rate = Set(metadata.sample_rate.map(|v| v as i32));
    active.channels = Set(metadata.channels.map(|v| v as i32));
    active.bitrate_kbps = Set(metadata.bitrate_kbps.map(|v| v as i32));
    active.replaygain_track_gain_db = Set(metadata.replaygain_track_gain_db);
    active.isrc = Set(metadata.isrc.clone());
    apply_user_columns(
        active,
        metadata.rating,
        metadata.comment.clone(),
        metadata.year,
        metadata.track_number,
        metadata.disc_number,
        metadata.color_label,
    );
}

/// Assign the fields the track editor owns. Shared by insert/upsert and the
/// full-replacement edit path so the two can never diverge.
fn apply_user_columns(
    active: &mut tracks::ActiveModel,
    rating: Option<u8>,
    comment: Option<String>,
    year: Option<u16>,
    track_number: Option<u16>,
    disc_number: Option<u16>,
    color_label: Option<library_core::ColorLabel>,
) {
    active.rating = Set(rating.map(i32::from));
    active.comment = Set(comment);
    active.year = Set(year.map(i32::from));
    active.track_number = Set(track_number.map(i32::from));
    active.disc_number = Set(disc_number.map(i32::from));
    active.color_label = Set(color_label.map(|c| c.as_str().to_string()));
}

/// Columns refreshed when an existing track is upserted.
fn metadata_update_columns() -> Vec<tracks::Column> {
    let mut columns = vec![
        tracks::Column::SourceType,
        tracks::Column::SourceRef,
        tracks::Column::Provider,
    ];
    columns.extend([
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
        tracks::Column::Rating,
        tracks::Column::Comment,
        tracks::Column::Year,
        tracks::Column::TrackNumber,
        tracks::Column::DiscNumber,
        tracks::Column::ColorLabel,
    ]);
    columns.push(tracks::Column::UpdatedAt);
    columns
}

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
        let mut active = tracks::ActiveModel {
            id: Set(id.as_str().to_string()),
            source_type: Set("file".to_string()),
            source_ref: Set(source_ref),
            provider: Set(None),
            last_sampler_bank_id: Set(None),
            added_at: Set(now.to_string()),
            updated_at: Set(now.to_string()),
            ..Default::default()
        };
        apply_metadata_columns(&mut active, metadata);

        TrackEntity::insert(active)
            .on_conflict(
                OnConflict::column(tracks::Column::Id)
                    .update_columns(metadata_update_columns())
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
        let mut active = tracks::ActiveModel {
            id: Set(id.as_str().to_string()),
            source_type: Set("stream".to_string()),
            source_ref: Set(uri.to_string()),
            provider: Set(provider.map(str::to_string)),
            last_sampler_bank_id: Set(None),
            added_at: Set(now.to_string()),
            updated_at: Set(now.to_string()),
            ..Default::default()
        };
        apply_metadata_columns(&mut active, metadata);

        TrackEntity::insert(active)
            .on_conflict(
                OnConflict::column(tracks::Column::Id)
                    .update_columns(metadata_update_columns())
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

    pub fn get_tracks_by_ids(
        &self,
        ids: &[TrackId],
    ) -> Result<HashMap<String, library_core::AudioSource>> {
        let mut by_id = HashMap::new();
        if ids.is_empty() {
            return Ok(by_id);
        }
        let conn = self.db.conn()?;
        for chunk in ids.chunks(TRACK_ID_CHUNK) {
            let rows = TrackEntity::find()
                .filter(tracks::Column::Id.is_in(track_id_strings(chunk)))
                .all(conn.as_connection())
                .map_err(db::db_err)?;
            by_id.extend(rows.into_iter().filter_map(|row| {
                model::track_source(row)
                    .ok()
                    .map(|source| (source.id().as_str().to_string(), source))
            }));
        }
        Ok(by_id)
    }

    pub fn get_track(&self, id: &TrackId) -> Result<Option<library_core::AudioSource>> {
        let row = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        row.map(model::track_source).transpose()
    }

    /// Overwrite only the user-editable metadata columns; the technical fields
    /// (`duration_ms`, `sample_rate`, `channels`, `bitrate_kbps`,
    /// `replaygain_track_gain_db`, `last_sampler_bank_id`) and `added_at` are
    /// left untouched. `updated_at` is stamped with `now` on every call.
    /// Unknown id is a no-op (no phantom row).
    ///
    /// **Full replacement:** every `None` column is written as `NULL`, so
    /// blanking a field clears the stored value. Callers must send the
    /// complete field set — there is no partial update.
    pub fn update_track_metadata(
        &self,
        id: &TrackId,
        edit: &TrackMetadataEdit,
        now: &str,
    ) -> Result<()> {
        let Some(row) = TrackEntity::find_by_id(id.as_str())
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?
        else {
            return Ok(());
        };
        // This write path may not introduce an invalid BPM; re-persisting the
        // value already stored (the untouched field of a full-replacement
        // write) passes, so an unrelated edit never has to clear a column to
        // satisfy the rule. Compare numerically rather than by bits: `==`
        // treats `-0.0` and `0.0` as equal (both `<= 0`, so both take this
        // branch) while NaN needs its own arm to round-trip at all.
        if let Some(v) = edit.bpm {
            let invalid = !v.is_finite() || v <= 0.0;
            let unchanged = row
                .bpm
                .is_some_and(|old| old == v || (old.is_nan() && v.is_nan()));
            if invalid && !unchanged {
                return Err(library_core::LibraryError::Backend {
                    backend: "library",
                    message: format!("invalid BPM: {v}"),
                });
            }
        }
        let mut active: tracks::ActiveModel = row.into();
        active.title = Set(edit.title.clone());
        active.artist = Set(edit.artist.clone());
        active.album = Set(edit.album.clone());
        active.genre = Set(edit.genre.clone());
        active.bpm = Set(edit.bpm);
        active.key = Set(edit.key.clone());
        active.isrc = Set(edit.isrc.clone());
        apply_user_columns(
            &mut active,
            edit.rating,
            edit.comment.clone(),
            edit.year,
            edit.track_number,
            edit.disc_number,
            edit.color_label,
        );
        active.metadata_edited = Set(Some(true));
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

    /// Stored metadata plus the editor marker for a file path, in one query.
    pub fn file_track_metadata_and_edited(
        &self,
        source_ref: &str,
    ) -> Result<Option<(TrackMetadata, bool)>> {
        let row = TrackEntity::find()
            .filter(tracks::Column::SourceType.eq("file"))
            .filter(tracks::Column::SourceRef.eq(source_ref))
            .one(self.db.conn()?.as_connection())
            .map_err(db::db_err)?;
        Ok(row.map(|row| {
            let edited = row.metadata_edited.unwrap_or(false);
            (model::track_metadata(&row), edited)
        }))
    }

    /// File sources under `root`.
    ///
    /// `prefix` is the escaped `root/%` LIKE pattern. A non-recursive query
    /// also excludes anything nested below a direct child, so the caller never
    /// has to filter the result set in Rust.
    pub fn find_file_sources_under(
        &self,
        root: &str,
        prefix: &str,
        recursive: bool,
    ) -> Result<Vec<library_core::AudioSource>> {
        // One grouped predicate: `ESCAPE` and the NOT LIKE have to apply to
        // the LIKE branch only, so the whole membership rule stays in SQL.
        let (sql, values): (&str, Vec<sea_orm::Value>) = if recursive {
            (
                "(source_ref = ? OR source_ref LIKE ? ESCAPE '\\')",
                vec![
                    sea_orm::Value::from(root.to_string()),
                    sea_orm::Value::from(prefix.to_string()),
                ],
            )
        } else {
            (
                "(source_ref = ? OR (source_ref LIKE ? ESCAPE '\\' AND source_ref NOT LIKE ? ESCAPE '\\'))",
                vec![
                    sea_orm::Value::from(root.to_string()),
                    sea_orm::Value::from(prefix.to_string()),
                    // `prefix` ends in a wildcard, so this pattern only matches
                    // paths with a second separator after the root.
                    sea_orm::Value::from(format!("{prefix}/%")),
                ],
            )
        };
        let rows = TrackEntity::find()
            .filter(tracks::Column::SourceType.eq("file"))
            .filter(Expr::cust_with_values(sql, values))
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

    /// Distinct track ids among `ids` that still have a playlist entry.
    pub fn playlist_linked_track_ids(&self, ids: &[TrackId]) -> Result<HashSet<String>> {
        let mut linked = HashSet::new();
        if ids.is_empty() {
            return Ok(linked);
        }
        let conn = self.db.conn()?;
        for chunk in ids.chunks(TRACK_ID_CHUNK) {
            let rows = CollectionEntryEntity::find()
                .filter(collection_entries::Column::TrackId.is_in(track_id_strings(chunk)))
                .all(conn.as_connection())
                .map_err(db::db_err)?;
            linked.extend(rows.into_iter().map(|row| row.track_id));
        }
        Ok(linked)
    }

    /// Delete the given track rows in one transaction (related rows cascade).
    pub fn delete_tracks(&self, ids: &[TrackId]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.db.conn()?;
        let txn = conn
            .as_connection()
            .begin_with_options(TransactionOptions {
                sqlite_transaction_mode: Some(SqliteTransactionMode::Immediate),
                ..Default::default()
            })
            .map_err(db::db_err)?;
        let result = (|| -> Result<()> {
            for chunk in ids.chunks(TRACK_ID_CHUNK) {
                TrackEntity::delete_many()
                    .filter(tracks::Column::Id.is_in(track_id_strings(chunk)))
                    .exec(&txn)
                    .map_err(db::db_err)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                txn.commit().map_err(db::db_err)?;
                Ok(())
            }
            Err(error) => {
                let _ = txn.rollback();
                Err(error)
            }
        }
    }

    /// Test helper: remove a track row but leave playlist entries pointing at it.
    #[cfg(test)]
    pub fn delete_track_row_only(&self, id: &TrackId) -> Result<()> {
        let guard = self.db.conn()?;
        let conn = guard.as_connection();
        conn.execute_raw(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "PRAGMA foreign_keys = OFF".to_string(),
        ))
        .map_err(db::db_err)?;
        TrackEntity::delete_by_id(id.as_str())
            .exec(conn)
            .map_err(db::db_err)?;
        conn.execute_raw(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            "PRAGMA foreign_keys = ON".to_string(),
        ))
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

    pub fn update_collection_scan_folder_tree(
        &self,
        id: &CollectionId,
        scan_folder_tree: bool,
    ) -> Result<()> {
        CollectionEntity::update_many()
            .col_expr(
                collections::Column::ScanFolderTree,
                Expr::value(i32::from(scan_folder_tree)),
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

    pub fn list_collection_entries(
        &self,
        collection_id: &CollectionId,
    ) -> Result<Vec<CollectionEntry>> {
        let rows = CollectionEntryEntity::find()
            .filter(collection_entries::Column::CollectionId.eq(collection_id.as_str()))
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
        let rows = self.list_collection_entries(playlist_id)?;
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

/// The user-editable metadata columns.
///
/// Grouped so a caller names the field it is setting instead of passing a run
/// of `Option`s positionally — many same-typed parameters would let a
/// transposition (`artist`/`album`, `genre`/`key`) compile silently.
///
/// The derived [`Default`] is "every column `None`", which under the
/// full-replacement write means **clear every column** — never a
/// leave-unchanged value. Build it field by field instead of reaching for
/// `..Default::default()` in code that only edits one column.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackMetadataEdit {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub isrc: Option<String>,
    /// Star rating 1–5; `None` clears it.
    pub rating: Option<u8>,
    pub comment: Option<String>,
    pub year: Option<u16>,
    pub track_number: Option<u16>,
    pub disc_number: Option<u16>,
    pub color_label: Option<library_core::ColorLabel>,
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

    #[test]
    fn update_track_metadata_replaces_editable_columns_only() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/edit.wav");
        let meta = TrackMetadata {
            title: Some("Old".into()),
            artist: Some("OldA".into()),
            album: Some("OldAl".into()),
            genre: Some("OldG".into()),
            bpm: Some(100.0),
            key: Some("C".into()),
            isrc: Some("OLD1".into()),
            duration_ms: Some(1000),
            sample_rate: Some(44_100),
            ..TrackMetadata::default()
        };
        store
            .upsert_file_track(&id, Path::new("/music/edit.wav"), &meta, "1")
            .unwrap();

        store
            .update_track_metadata(
                &id,
                &TrackMetadataEdit {
                    title: Some("New".into()),
                    artist: None,
                    album: Some("NewAl".into()),
                    genre: None,
                    bpm: Some(128.0),
                    key: Some("Am".into()),
                    isrc: Some("NEW1".into()),
                    ..TrackMetadataEdit::default()
                },
                "2",
            )
            .unwrap();

        let fetched = store.get_track(&id).unwrap().unwrap();
        let metadata = fetched.metadata();
        assert_eq!(metadata.title.as_deref(), Some("New"));
        assert_eq!(metadata.artist, None, "blank artist clears the column");
        assert_eq!(metadata.album.as_deref(), Some("NewAl"));
        assert_eq!(metadata.genre, None, "blank genre clears the column");
        assert_eq!(metadata.bpm, Some(128.0));
        assert_eq!(metadata.key.as_deref(), Some("Am"));
        assert_eq!(metadata.isrc.as_deref(), Some("NEW1"));
        assert_eq!(metadata.duration_ms, Some(1000), "technical fields stay");
        assert_eq!(metadata.sample_rate, Some(44_100));
    }

    #[test]
    fn update_track_metadata_round_trips_rating_year_numbers_and_color() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/detailed.wav");
        let meta = TrackMetadata {
            rating: Some(5),
            comment: Some("Peak time".into()),
            year: Some(1999),
            track_number: Some(3),
            disc_number: Some(1),
            color_label: Some(library_core::ColorLabel::Pink),
            ..TrackMetadata::default()
        };
        store
            .upsert_file_track(&id, Path::new("/music/detailed.wav"), &meta, "1")
            .unwrap();
        let (_, edited) = store
            .file_track_metadata_and_edited("/music/detailed.wav")
            .unwrap()
            .unwrap();
        assert!(!edited, "a fresh file row is not editor-owned");

        store
            .update_track_metadata(
                &id,
                &TrackMetadataEdit {
                    rating: Some(2),
                    comment: Some("Warm up".into()),
                    year: Some(2024),
                    track_number: Some(7),
                    disc_number: Some(2),
                    color_label: Some(library_core::ColorLabel::Blue),
                    ..TrackMetadataEdit::default()
                },
                "2",
            )
            .unwrap();

        let metadata = store.get_track(&id).unwrap().unwrap().metadata().clone();
        assert_eq!(metadata.rating, Some(2));
        assert_eq!(metadata.comment.as_deref(), Some("Warm up"));
        assert_eq!(metadata.year, Some(2024));
        assert_eq!(metadata.track_number, Some(7));
        assert_eq!(metadata.disc_number, Some(2));
        assert_eq!(metadata.color_label, Some(library_core::ColorLabel::Blue));
        let (_, edited) = store
            .file_track_metadata_and_edited("/music/detailed.wav")
            .unwrap()
            .unwrap();
        assert!(edited, "editor save marks the row authoritative");
        assert!(store
            .file_track_metadata_and_edited("/music/absent.wav")
            .unwrap()
            .is_none());

        store
            .update_track_metadata(&id, &TrackMetadataEdit::default(), "3")
            .unwrap();
        let cleared = store.get_track(&id).unwrap().unwrap().metadata().clone();
        assert_eq!(cleared.rating, None);
        assert_eq!(cleared.comment, None);
        assert_eq!(cleared.year, None);
        assert_eq!(cleared.track_number, None);
        assert_eq!(cleared.disc_number, None);
        assert_eq!(cleared.color_label, None);
    }

    #[test]
    fn update_track_metadata_unknown_id_is_a_noop() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/absent.wav");

        store
            .update_track_metadata(
                &id,
                &TrackMetadataEdit {
                    title: Some("New".into()),
                    artist: None,
                    album: None,
                    genre: None,
                    bpm: None,
                    key: None,
                    isrc: None,
                    ..TrackMetadataEdit::default()
                },
                "2",
            )
            .unwrap();

        assert!(store.get_track(&id).unwrap().is_none(), "no phantom row");
    }

    #[test]
    fn update_track_metadata_round_trips_stored_invalid_bpm_but_rejects_new() {
        let db = db::open_in_memory().unwrap();
        let store = Store::new(&db);
        let id = TrackId::new("/music/corrupt.wav");
        // A malformed file tag can have put a non-positive BPM in the column.
        let meta = TrackMetadata {
            bpm: Some(-4.0),
            ..TrackMetadata::default()
        };
        store
            .upsert_file_track(&id, Path::new("/music/corrupt.wav"), &meta, "1")
            .unwrap();

        // Re-persisting the value that is already stored passes, so editing an
        // unrelated field neither fails nor clears the untouched column.
        store
            .update_track_metadata(
                &id,
                &TrackMetadataEdit {
                    title: Some("Edited".into()),
                    artist: None,
                    album: None,
                    genre: None,
                    bpm: Some(-4.0),
                    key: None,
                    isrc: None,
                    ..TrackMetadataEdit::default()
                },
                "2",
            )
            .unwrap();
        let fetched = store.get_track(&id).unwrap().unwrap();
        assert_eq!(
            fetched.metadata().bpm,
            Some(-4.0),
            "unchanged corrupt value round-trips instead of being cleared"
        );
        assert_eq!(fetched.metadata().title.as_deref(), Some("Edited"));

        // `-0.0` is `<= 0` (so it may not be *introduced*) yet compares equal
        // to the stored value, so an untouched round-trip must not reject it.
        let zero_id = TrackId::new("/music/negative-zero.wav");
        let zero_meta = TrackMetadata {
            bpm: Some(-0.0),
            ..TrackMetadata::default()
        };
        store
            .upsert_file_track(
                &zero_id,
                Path::new("/music/negative-zero.wav"),
                &zero_meta,
                "1",
            )
            .unwrap();
        store
            .update_track_metadata(
                &zero_id,
                &TrackMetadataEdit {
                    title: Some("Edited".into()),
                    artist: None,
                    album: None,
                    genre: None,
                    bpm: Some(-0.0),
                    key: None,
                    isrc: None,
                    ..TrackMetadataEdit::default()
                },
                "2",
            )
            .unwrap();
        let zero = store.get_track(&zero_id).unwrap().unwrap();
        assert_eq!(zero.metadata().title.as_deref(), Some("Edited"));
        assert_eq!(zero.metadata().bpm, Some(-0.0), "still zero, not rejected");

        // Introducing an invalid value is rejected by the column owner.
        let err = store
            .update_track_metadata(
                &id,
                &TrackMetadataEdit {
                    title: None,
                    artist: None,
                    album: None,
                    genre: None,
                    bpm: Some(0.0),
                    key: None,
                    isrc: None,
                    ..TrackMetadataEdit::default()
                },
                "3",
            )
            .expect_err("a newly supplied invalid BPM must be rejected");
        assert!(
            format!("{err}").contains("invalid BPM"),
            "unexpected error: {err}"
        );
    }
}
