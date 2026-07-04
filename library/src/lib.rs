//! Library manager for rust-dj-engine.
//!
//! Implements [`Library`] and [`WritableLibrary`] from `library-core`.
//! Persistence is an implementation detail of this crate.
//! Third-party DJ software adapters live in `library-adapters`.
//!
//! # Example
//!
//! ```no_run
//! use library::{Library, LibraryConfig, LibraryManager, NewCollection, WritableLibrary};
//!
//! let mut lib = LibraryManager::open("library.db", LibraryConfig::default())?;
//! let folder = lib.add_collection(&NewCollection::folder("/music"))?;
//! lib.sync_collection(Some(&folder.id))?;
//! let tracks = lib.get_collection_tracks(&folder.id)?;
//! # Ok::<(), library::LibraryError>(())
//! ```

mod db;
mod tags;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

pub use library_core::{
    AudioSource, Collection, CollectionConfig, CollectionConfigUpdate, CollectionId,
    CollectionTrack, CollectionType, FileAudioSource, Library, LibraryConfig, LibraryError,
    LibrarySource, LoadedAudio, NewCollection, Result, ScanReport, StreamAudioSource,
    StreamProvider, TrackId, TrackMetadata, UpdateCollection, WritableLibrary,
    AnalyzeTrackOptions,
};

/// Audio file extensions recognized during scan/import.
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aiff", "aif", "ogg", "m4a", "aac", "opus", "wma", "alac",
];

/// The user’s library manager (canonical writable store).
pub struct LibraryManager {
    conn: Mutex<Connection>,
    config: LibraryConfig,
}

impl LibraryManager {
    /// Open (or create) a library database at `db_path`.
    pub fn open(db_path: impl AsRef<Path>, config: LibraryConfig) -> Result<Self> {
        let conn = db::open_connection(db_path.as_ref())?;
        Ok(Self {
            conn: Mutex::new(conn),
            config,
        })
    }

    /// Open an in-memory library (for tests).
    pub fn open_in_memory(config: LibraryConfig) -> Result<Self> {
        let conn = db::open_in_memory()?;
        Ok(Self {
            conn: Mutex::new(conn),
            config,
        })
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| LibraryError::Backend {
            backend: "library",
            message: "library database lock poisoned".into(),
        })
    }

    /// Borrow the library configuration.
    pub fn config(&self) -> &LibraryConfig {
        &self.config
    }

    /// Replace the library configuration.
    pub fn set_config(&mut self, config: LibraryConfig) {
        self.config = config;
    }

    fn track_id_for(path: &Path) -> TrackId {
        TrackId::new(path.to_string_lossy())
    }

    fn folder_id_for(path: &Path) -> CollectionId {
        CollectionId::new(format!("folder:{}", path.to_string_lossy()))
    }

    fn new_playlist_id() -> CollectionId {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        CollectionId::new(format!("playlist:{nanos}"))
    }

    fn upsert_file_source(&self, path: &Path, metadata: &TrackMetadata) -> Result<LibrarySource> {
        let path = normalize_path(path)?;
        let id = Self::track_id_for(&path);
        let now = now_stamp();
        let conn = self.lock_conn()?;
        let source_ref = path.to_string_lossy();

        conn.execute(
            "
            INSERT INTO tracks (
                id, source_type, source_ref, provider, title, artist, album, genre, bpm, key,
                duration_secs, sample_rate, channels, bitrate_kbps,
                added_at, updated_at
            ) VALUES (
                ?1, 'file', ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12,
                ?13, ?13
            )
            ON CONFLICT(id) DO UPDATE SET
                source_type = excluded.source_type,
                source_ref = excluded.source_ref,
                provider = excluded.provider,
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                genre = excluded.genre,
                bpm = excluded.bpm,
                key = excluded.key,
                duration_secs = excluded.duration_secs,
                sample_rate = excluded.sample_rate,
                channels = excluded.channels,
                bitrate_kbps = excluded.bitrate_kbps,
                updated_at = excluded.updated_at
            ",
            params![
                id.as_str(),
                source_ref.as_ref(),
                metadata.title,
                metadata.artist,
                metadata.album,
                metadata.genre,
                metadata.bpm,
                metadata.key,
                metadata.duration_secs,
                metadata.sample_rate,
                metadata.channels.map(|c| c as i64),
                metadata.bitrate_kbps.map(|b| b as i64),
                now,
            ],
        )
        .map_err(db::db_err)?;

        Ok(LibrarySource::File(FileAudioSource::new(
            id,
            path,
            metadata.clone(),
        )))
    }

    fn upsert_stream_source(
        &self,
        uri: &str,
        metadata: &TrackMetadata,
        provider: Option<StreamProvider>,
    ) -> Result<LibrarySource> {
        let id = Self::stream_id_for(uri, provider);
        let now = now_stamp();
        let conn = self.lock_conn()?;
        let provider_str = provider.map(|p| p.as_str());

        conn.execute(
            "
            INSERT INTO tracks (
                id, source_type, source_ref, provider, title, artist, album, genre, bpm, key,
                duration_secs, sample_rate, channels, bitrate_kbps,
                added_at, updated_at
            ) VALUES (
                ?1, 'stream', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13,
                ?14, ?14
            )
            ON CONFLICT(id) DO UPDATE SET
                source_type = excluded.source_type,
                source_ref = excluded.source_ref,
                provider = excluded.provider,
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                genre = excluded.genre,
                bpm = excluded.bpm,
                key = excluded.key,
                duration_secs = excluded.duration_secs,
                sample_rate = excluded.sample_rate,
                channels = excluded.channels,
                bitrate_kbps = excluded.bitrate_kbps,
                updated_at = excluded.updated_at
            ",
            params![
                id.as_str(),
                uri,
                provider_str,
                metadata.title,
                metadata.artist,
                metadata.album,
                metadata.genre,
                metadata.bpm,
                metadata.key,
                metadata.duration_secs,
                metadata.sample_rate,
                metadata.channels.map(|c| c as i64),
                metadata.bitrate_kbps.map(|b| b as i64),
                now,
            ],
        )
        .map_err(db::db_err)?;

        Ok(LibrarySource::Stream(StreamAudioSource::new(
            id,
            uri,
            metadata.clone(),
            provider,
        )))
    }

    fn stream_id_for(uri: &str, provider: Option<StreamProvider>) -> TrackId {
        match provider {
            Some(p) => TrackId::new(format!("stream:{}:{uri}", p.as_str())),
            None => TrackId::new(format!("stream:{uri}")),
        }
    }

    fn row_metadata(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<TrackMetadata> {
        let channels: Option<i64> = row.get(offset + 8)?;
        let bitrate: Option<i64> = row.get(offset + 9)?;
        Ok(TrackMetadata {
            title: row.get(offset)?,
            artist: row.get(offset + 1)?,
            album: row.get(offset + 2)?,
            genre: row.get(offset + 3)?,
            bpm: row.get(offset + 4)?,
            key: row.get(offset + 5)?,
            duration_secs: row.get(offset + 6)?,
            sample_rate: row.get::<_, Option<i64>>(offset + 7)?.map(|v| v as u32),
            channels: channels.map(|v| v as u16),
            bitrate_kbps: bitrate.map(|v| v as u32),
        })
    }

    fn row_to_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibrarySource> {
        let id = TrackId::new(row.get::<_, String>(0)?);
        let source_type: String = row.get(1)?;
        let source_ref: String = row.get(2)?;
        let provider: Option<String> = row.get(3)?;
        let metadata = Self::row_metadata(row, 4)?;

        match source_type.as_str() {
            "file" => Ok(LibrarySource::File(FileAudioSource::new(
                id,
                PathBuf::from(source_ref),
                metadata,
            ))),
            "stream" => {
                let provider = provider
                    .as_deref()
                    .map(|s| s.parse::<StreamProvider>())
                    .transpose()
                    .map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("unknown stream provider: {:?}", provider),
                            )),
                        )
                    })?;
                Ok(LibrarySource::Stream(StreamAudioSource::new(
                    id,
                    source_ref,
                    metadata,
                    provider,
                )))
            }
            other => Err(rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown source_type: {other}"),
                )),
            )),
        }
    }

    fn row_to_collection(row: &rusqlite::Row<'_>) -> rusqlite::Result<Collection> {
        let type_str: String = row.get(2)?;
        let collection_type = type_str.parse::<CollectionType>().map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown collection_type: {type_str}"),
                )),
            )
        })?;
        let sortable: i64 = row.get(3)?;
        let fs_path: Option<String> = row.get(4)?;
        let config = match collection_type {
            CollectionType::Folder => {
                let Some(fs_path) = fs_path else {
                    return Err(rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "folder collection missing fs_path",
                        )),
                    ));
                };
                CollectionConfig::Folder {
                    fs_path: PathBuf::from(fs_path),
                }
            }
            CollectionType::Playlist => CollectionConfig::Playlist {
                sortable: sortable != 0,
            },
        };
        Ok(Collection {
            id: CollectionId::new(row.get::<_, String>(0)?),
            name: row.get(1)?,
            config,
        })
    }

    fn require_collection(&self, id: &CollectionId) -> Result<Collection> {
        self.get_collection(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))
    }

    fn require_playlist(&self, id: &CollectionId) -> Result<Collection> {
        let c = self.require_collection(id)?;
        if c.collection_type() != CollectionType::Playlist {
            return Err(LibraryError::WrongCollectionType {
                expected: "playlist",
                got: c.collection_type().as_str(),
            });
        }
        Ok(c)
    }

    fn sync_one_collection(&mut self, collection: &Collection) -> Result<ScanReport> {
        match collection.collection_type() {
            CollectionType::Folder => self.sync_folder(collection),
            CollectionType::Playlist => self.sync_playlist(collection),
        }
    }

    fn sync_folder(&mut self, folder: &Collection) -> Result<ScanReport> {
        let fs_path = folder.fs_path().ok_or_else(|| LibraryError::Backend {
            backend: "library",
            message: format!("folder {} has no fs_path", folder.id),
        })?;

        let mut report = ScanReport::default();
        if !fs_path.exists() {
            report.failed += 1;
            report
                .errors
                .push(format!("root not found: {}", fs_path.display()));
            return Ok(report);
        }

        let recursive = self.config.scan_folder_tree;
        let files = collect_audio_files(fs_path, recursive)?;
        for file in files {
            let existed = self.get_track(&Self::track_id_for(&file))?.is_some();
            match self.import_path(&file) {
                Ok(_) if existed => report.updated += 1,
                Ok(_) => report.added += 1,
                Err(LibraryError::UnsupportedFile(_)) => report.skipped += 1,
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(format!("{}: {err}", file.display()));
                }
            }
        }
        Ok(report)
    }

    fn sync_playlist(&mut self, playlist: &Collection) -> Result<ScanReport> {
        let sources = self.get_collection_tracks(&playlist.id)?;
        let mut report = ScanReport::default();
        for source in sources {
            let Some(file) = source.file() else {
                report.skipped += 1;
                continue;
            };
            if !file.path().is_file() {
                report.skipped += 1;
                continue;
            }
            match self.analyze_file_source(file.path(), AnalyzeTrackOptions::default()) {
                Ok(_) => report.updated += 1,
                Err(LibraryError::UnsupportedFile(_)) => report.skipped += 1,
                Err(err) => {
                    report.failed += 1;
                    report
                        .errors
                        .push(format!("{}: {err}", file.path().display()));
                }
            }
        }
        Ok(report)
    }

    fn add_folder_collection(&mut self, collection: &NewCollection) -> Result<Collection> {
        let CollectionConfig::Folder { fs_path: path } = &collection.config else {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "folder collection requires Folder config".into(),
            });
        };
        let path = normalize_path(path)?;
        if !path.is_dir() {
            return Err(LibraryError::NotADirectory(path));
        }

        let id = Self::folder_id_for(&path);
        if let Some(existing) = self.get_collection(&id)? {
            return Ok(existing);
        }

        let name = collection
            .name
            .clone()
            .or_else(|| {
                path.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| path.display().to_string());

        let conn = self.lock_conn()?;
        conn.execute(
            "
            INSERT INTO collections (id, name, collection_type, sortable, fs_path)
            VALUES (?1, ?2, 'folder', 0, ?3)
            ",
            params![id.as_str(), name, path.to_string_lossy().as_ref()],
        )
        .map_err(db::db_err)?;

        Ok(Collection {
            id,
            name,
            config: CollectionConfig::Folder { fs_path: path },
        })
    }

    fn add_playlist_collection(&mut self, collection: &NewCollection) -> Result<Collection> {
        let CollectionConfig::Playlist { sortable } = collection.config else {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "playlist collection requires Playlist config".into(),
            });
        };
        let name = collection.name.as_deref().ok_or_else(|| LibraryError::Backend {
            backend: "library",
            message: "playlist collection requires a name".into(),
        })?;

        let id = Self::new_playlist_id();
        let conn = self.lock_conn()?;
        conn.execute(
            "
            INSERT INTO collections (id, name, collection_type, sortable, fs_path)
            VALUES (?1, ?2, 'playlist', ?3, NULL)
            ",
            params![id.as_str(), name, i64::from(sortable)],
        )
        .map_err(db::db_err)?;

        Ok(Collection {
            id,
            name: name.to_string(),
            config: CollectionConfig::Playlist { sortable },
        })
    }

    fn playlist_track_ids(&self, playlist_id: &CollectionId) -> Result<Vec<TrackId>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "
                SELECT track_id FROM collection_tracks
                WHERE collection_id = ?1
                ORDER BY CASE WHEN position IS NULL THEN 1 ELSE 0 END, position ASC, track_id ASC
                ",
            )
            .map_err(db::db_err)?;
        let rows = stmt
            .query_map(params![playlist_id.as_str()], |row| {
                Ok(TrackId::new(row.get::<_, String>(0)?))
            })
            .map_err(db::db_err)?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(db::db_err)?);
        }
        Ok(ids)
    }

    fn file_sources_under(&self, root: &Path) -> Result<Vec<LibrarySource>> {
        let root_str = root.to_string_lossy().into_owned();
        let prefix = format!("{}/%", escape_like(&root_str));
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "
                SELECT id, source_type, source_ref, provider, title, artist, album, genre, bpm, key,
                       duration_secs, sample_rate, channels, bitrate_kbps
                FROM tracks
                WHERE source_type = 'file'
                  AND (source_ref = ?1 OR source_ref LIKE ?2 ESCAPE '\\')
                ORDER BY source_ref ASC
                ",
            )
            .map_err(db::db_err)?;
        let rows = stmt
            .query_map(params![root_str, prefix], Self::row_to_source)
            .map_err(db::db_err)?;
        let mut sources = Vec::new();
        for row in rows {
            sources.push(row.map_err(db::db_err)?);
        }
        Ok(sources)
    }

    pub(crate) fn import_path(&self, path: &Path) -> Result<LibrarySource> {
        self.analyze_file_source(path, AnalyzeTrackOptions::default())
    }

    pub(crate) fn import_stream(
        &self,
        uri: &str,
        metadata: &TrackMetadata,
        provider: Option<StreamProvider>,
    ) -> Result<LibrarySource> {
        if uri.is_empty() {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "stream uri must not be empty".into(),
            });
        }
        self.upsert_stream_source(uri, metadata, provider)
    }

    fn track_linked_to_collections(&self, track_id: &TrackId) -> Result<bool> {
        let in_playlist: i64 = {
            let conn = self.lock_conn()?;
            conn.query_row(
                "SELECT COUNT(*) FROM collection_tracks WHERE track_id = ?1",
                params![track_id.as_str()],
                |row| row.get(0),
            )
            .map_err(db::db_err)?
        };
        if in_playlist > 0 {
            return Ok(true);
        }

        let Some(source) = self.get_track(track_id)? else {
            return Ok(false);
        };
        let Some(file) = source.file() else {
            return Ok(false);
        };

        for collection in self.list_collections()? {
            if collection.collection_type() != CollectionType::Folder {
                continue;
            }
            let Some(fs_path) = collection.fs_path() else {
                continue;
            };
            if path_under_folder(file.path(), fs_path) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn remove_track_if_orphaned(&self, track_id: &TrackId) -> Result<()> {
        if self.track_linked_to_collections(track_id)? {
            return Ok(());
        }
        let conn = self.lock_conn()?;
        conn.execute("DELETE FROM tracks WHERE id = ?1", params![track_id.as_str()])
            .map_err(db::db_err)?;
        Ok(())
    }

    fn analyze_file_source(
        &self,
        path: &Path,
        options: AnalyzeTrackOptions,
    ) -> Result<LibrarySource> {
        let path = normalize_path(path)?;
        if !path.is_file() {
            return Err(LibraryError::PathNotFound(path));
        }
        if !is_audio_file(&path) {
            return Err(LibraryError::UnsupportedFile(path));
        }
        let tag_metadata = tags::read_tags(&path)?;
        let metadata = merge_analyzed_metadata(&tag_metadata, None, None, options.force);
        self.upsert_file_source(&path, &metadata)
    }
}

/// Merge file tags with optional DSP analysis results.
fn merge_analyzed_metadata(
    tags: &TrackMetadata,
    analysis_bpm: Option<f64>,
    analysis_key: Option<&str>,
    force: bool,
) -> TrackMetadata {
    let mut metadata = tags.clone();

    match (force, analysis_bpm) {
        (true, Some(bpm)) => metadata.bpm = Some(bpm),
        (false, Some(bpm)) if metadata.bpm.is_none() => metadata.bpm = Some(bpm),
        _ => {}
    }

    match (force, analysis_key) {
        (true, Some(key)) => metadata.key = Some(key.to_string()),
        (false, Some(key)) if metadata.key.is_none() => metadata.key = Some(key.to_string()),
        _ => {}
    }

    metadata
}

impl Library for LibraryManager {
    fn name(&self) -> &'static str {
        "library"
    }

    fn get_track(&self, id: &TrackId) -> Result<Option<LibrarySource>> {
        let conn = self.lock_conn()?;
        conn.query_row(
            "
            SELECT id, source_type, source_ref, provider, title, artist, album, genre, bpm, key,
                   duration_secs, sample_rate, channels, bitrate_kbps
            FROM tracks
            WHERE id = ?1
            ",
            params![id.as_str()],
            Self::row_to_source,
        )
        .optional()
        .map_err(db::db_err)
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "
                SELECT id, name, collection_type, sortable, fs_path
                FROM collections
                ORDER BY name ASC
                ",
            )
            .map_err(db::db_err)?;
        let rows = stmt
            .query_map([], Self::row_to_collection)
            .map_err(db::db_err)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(db::db_err)?);
        }
        Ok(out)
    }

    fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>> {
        let conn = self.lock_conn()?;
        conn.query_row(
            "
            SELECT id, name, collection_type, sortable, fs_path
            FROM collections
            WHERE id = ?1
            ",
            params![id.as_str()],
            Self::row_to_collection,
        )
        .optional()
        .map_err(db::db_err)
    }

    fn get_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<LibrarySource>> {
        let collection = self.require_collection(collection_id)?;
        match collection.collection_type() {
            CollectionType::Folder => {
                let fs_path = collection.fs_path().ok_or_else(|| LibraryError::Backend {
                    backend: "library",
                    message: format!("folder {collection_id} has no fs_path"),
                })?;
                self.file_sources_under(fs_path)
            }
            CollectionType::Playlist => {
                let ids = self.playlist_track_ids(collection_id)?;
                let mut sources = Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(source) = self.get_track(&id)? {
                        sources.push(source);
                    }
                }
                Ok(sources)
            }
        }
    }
}

impl WritableLibrary for LibraryManager {
    fn analyze_track(
        &mut self,
        id: &TrackId,
        options: AnalyzeTrackOptions,
    ) -> Result<LibrarySource> {
        let source = self
            .get_track(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        match source {
            LibrarySource::File(file) => self.analyze_file_source(file.path(), options),
            LibrarySource::Stream(_) => Err(LibraryError::Unsupported(
                "stream track analysis not implemented",
            )),
        }
    }

    fn add_collection(&mut self, collection: &NewCollection) -> Result<Collection> {
        match collection.config.collection_type() {
            CollectionType::Folder => self.add_folder_collection(collection),
            CollectionType::Playlist => self.add_playlist_collection(collection),
        }
    }

    fn sync_collection(&mut self, collection_id: Option<&CollectionId>) -> Result<ScanReport> {
        let collections: Vec<Collection> = match collection_id {
            Some(id) => vec![self.require_collection(id)?],
            None => self.list_collections()?,
        };

        if collections.is_empty() {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "no collections to sync".into(),
            });
        }

        let mut report = ScanReport::default();
        for collection in &collections {
            let part = self.sync_one_collection(collection)?;
            report.added += part.added;
            report.updated += part.updated;
            report.skipped += part.skipped;
            report.failed += part.failed;
            report.errors.extend(part.errors);
        }
        Ok(report)
    }

    fn update_collection(
        &mut self,
        id: &CollectionId,
        update: &UpdateCollection,
    ) -> Result<()> {
        if update.name.is_none() && update.config.is_none() {
            return Ok(());
        }

        if let Some(name) = &update.name {
            let conn = self.lock_conn()?;
            let changed = conn
                .execute(
                    "UPDATE collections SET name = ?1 WHERE id = ?2",
                    params![name, id.as_str()],
                )
                .map_err(db::db_err)?;
            if changed == 0 {
                return Err(LibraryError::NotFound(id.to_string()));
            }
        }

        if let Some(CollectionConfigUpdate::Playlist { sortable }) = update.config {
            let playlist = self.require_playlist(id)?;
            if playlist.sortable() != sortable {
                let conn = self.lock_conn()?;
                conn.execute(
                    "UPDATE collections SET sortable = ?1 WHERE id = ?2",
                    params![i64::from(sortable), id.as_str()],
                )
                .map_err(db::db_err)?;

                if sortable {
                    let mut stmt = conn
                        .prepare(
                            "
                            SELECT track_id FROM collection_tracks
                            WHERE collection_id = ?1
                            ORDER BY track_id ASC
                            ",
                        )
                        .map_err(db::db_err)?;
                    let ids: Vec<String> = stmt
                        .query_map(params![id.as_str()], |row| row.get(0))
                        .map_err(db::db_err)?
                        .collect::<std::result::Result<_, _>>()
                        .map_err(db::db_err)?;
                    drop(stmt);

                    for (pos, track_id) in ids.iter().enumerate() {
                        conn.execute(
                            "
                            UPDATE collection_tracks SET position = ?1
                            WHERE collection_id = ?2 AND track_id = ?3
                            ",
                            params![pos as i32, id.as_str(), track_id],
                        )
                        .map_err(db::db_err)?;
                    }
                } else {
                    conn.execute(
                        "UPDATE collection_tracks SET position = NULL WHERE collection_id = ?1",
                        params![id.as_str()],
                    )
                    .map_err(db::db_err)?;
                }
            }
        }

        Ok(())
    }

    fn delete_collection(&mut self, id: &CollectionId) -> Result<()> {
        let conn = self.lock_conn()?;
        let changed = conn
            .execute("DELETE FROM collections WHERE id = ?1", params![id.as_str()])
            .map_err(db::db_err)?;
        if changed == 0 {
            return Err(LibraryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    fn add_collection_track(
        &mut self,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: Option<i32>,
    ) -> Result<()> {
        let playlist = self.require_playlist(collection_id)?;
        if self.get_track(track_id)?.is_none() {
            return Err(LibraryError::NotFound(track_id.to_string()));
        }

        let position = if playlist.sortable() {
            Some(position.unwrap_or_else(|| {
                self.playlist_track_ids(collection_id)
                    .map(|ids| ids.len() as i32)
                    .unwrap_or(0)
            }))
        } else {
            None
        };

        let conn = self.lock_conn()?;
        conn.execute(
            "
            INSERT INTO collection_tracks (collection_id, track_id, position)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(collection_id, track_id) DO UPDATE SET
                position = excluded.position
            ",
            params![collection_id.as_str(), track_id.as_str(), position],
        )
        .map_err(db::db_err)?;
        Ok(())
    }

    fn remove_collection_track(
        &mut self,
        collection_id: &CollectionId,
        track_id: &TrackId,
    ) -> Result<()> {
        let _ = self.require_playlist(collection_id)?;
        let changed = {
            let conn = self.lock_conn()?;
            conn.execute(
                "
                DELETE FROM collection_tracks
                WHERE collection_id = ?1 AND track_id = ?2
                ",
                params![collection_id.as_str(), track_id.as_str()],
            )
            .map_err(db::db_err)?
        };
        if changed == 0 {
            return Err(LibraryError::NotFound(format!(
                "{collection_id}/{track_id}"
            )));
        }
        self.remove_track_if_orphaned(track_id)?;
        Ok(())
    }

    fn update_collection_track(
        &mut self,
        collection_id: &CollectionId,
        track_ids: &[TrackId],
    ) -> Result<()> {
        let playlist = self.require_playlist(collection_id)?;
        if !playlist.sortable() {
            return Err(LibraryError::Unsupported(
                "reorder requires a sortable playlist",
            ));
        }

        let existing = self.playlist_track_ids(collection_id)?;
        if existing.len() != track_ids.len()
            || track_ids.iter().any(|id| !existing.contains(id))
        {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "update_collection_track must include exactly the playlist membership"
                    .into(),
            });
        }

        let conn = self.lock_conn()?;
        for (pos, track_id) in track_ids.iter().enumerate() {
            conn.execute(
                "
                UPDATE collection_tracks SET position = ?1
                WHERE collection_id = ?2 AND track_id = ?3
                ",
                params![pos as i32, collection_id.as_str(), track_id.as_str()],
            )
            .map_err(db::db_err)?;
        }
        Ok(())
    }
}

fn path_under_folder(path: &Path, folder: &Path) -> bool {
    path == folder || path.starts_with(folder.join(""))
}

fn normalize_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        path.canonicalize().map_err(LibraryError::from)
    } else {
        Ok(path.to_path_buf())
    }
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

fn collect_audio_files(root: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if root.is_file() {
        if is_audio_file(root) {
            files.push(normalize_path(root)?);
        }
        return Ok(files);
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    stack.push(path);
                }
            } else if is_audio_file(&path) {
                files.push(normalize_path(&path)?);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn now_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_minimal_wav(path: &Path) {
        let mut file = std::fs::File::create(path).unwrap();
        let data_size: u32 = 16;
        let file_size: u32 = 36 + data_size;
        file.write_all(b"RIFF").unwrap();
        file.write_all(&file_size.to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&8000u32.to_le_bytes()).unwrap();
        file.write_all(&8000u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&8u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();
        file.write_all(&[0u8; 16]).unwrap();
    }

    #[test]
    fn import_and_get_track_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("track.wav");
        write_minimal_wav(&wav);

        let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let track = lib.import_path(&wav).unwrap();

        assert_eq!(track.metadata().title.as_deref(), Some("track"));
        let fetched = lib.get_track(track.id()).unwrap().unwrap();
        assert_eq!(
            fetched.file().unwrap().path(),
            track.file().unwrap().path()
        );
    }

    #[test]
    fn add_collection_and_sync() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("house");
        std::fs::create_dir_all(&nested).unwrap();
        write_minimal_wav(&dir.path().join("a.wav"));
        write_minimal_wav(&nested.join("b.wav"));
        std::fs::write(dir.path().join("readme.txt"), b"nope").unwrap();

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let folder = lib.add_collection(&NewCollection::folder_named(dir.path(), "Music")).unwrap();
        assert_eq!(folder.collection_type(), CollectionType::Folder);
        assert_eq!(folder.name, "Music");

        let report = lib.sync_collection(Some(&folder.id)).unwrap();
        assert_eq!(report.added, 2);

        let tracks = lib.get_collection_tracks(&folder.id).unwrap();
        assert_eq!(tracks.len(), 2);
    }

    #[test]
    fn playlist_membership_and_order() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        write_minimal_wav(&a);
        write_minimal_wav(&b);

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let ta = lib.import_path(&a).unwrap();
        let tb = lib.import_path(&b).unwrap();

        let pl = lib.add_collection(&NewCollection::playlist("Warmup", true)).unwrap();
        lib.add_collection_track(&pl.id, ta.id(), None).unwrap();
        lib.add_collection_track(&pl.id, tb.id(), None).unwrap();

        let tracks = lib.get_collection_tracks(&pl.id).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].id(), ta.id());
        assert_eq!(tracks[1].id(), tb.id());

        lib.update_collection_track(&pl.id, &[tb.id().clone(), ta.id().clone()])
            .unwrap();
        let tracks = lib.get_collection_tracks(&pl.id).unwrap();
        assert_eq!(tracks[0].id(), tb.id());
        assert_eq!(tracks[1].id(), ta.id());

        lib.remove_collection_track(&pl.id, ta.id()).unwrap();
        assert_eq!(lib.get_collection_tracks(&pl.id).unwrap().len(), 1);
    }

    #[test]
    fn unsortable_playlist_rejects_reorder() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("x.wav");
        write_minimal_wav(&wav);

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let track = lib.import_path(&wav).unwrap();
        let pl = lib.add_collection(&NewCollection::playlist("Crate", false)).unwrap();
        lib.add_collection_track(&pl.id, track.id(), None).unwrap();

        let err = lib
            .update_collection_track(&pl.id, &[track.id().clone()])
            .unwrap_err();
        assert!(matches!(err, LibraryError::Unsupported(_)));
    }

    #[test]
    fn update_collection_sortable() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        write_minimal_wav(&a);
        write_minimal_wav(&b);

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let ta = lib.import_path(&a).unwrap();
        let tb = lib.import_path(&b).unwrap();
        let pl = lib.add_collection(&NewCollection::playlist("Set", false)).unwrap();
        lib.add_collection_track(&pl.id, ta.id(), None).unwrap();
        lib.add_collection_track(&pl.id, tb.id(), None).unwrap();

        lib.update_collection(&pl.id, &UpdateCollection::sortable(true))
            .unwrap();
        let pl = lib.get_collection(&pl.id).unwrap().unwrap();
        assert!(pl.sortable());
        lib.update_collection_track(&pl.id, &[tb.id().clone(), ta.id().clone()])
            .unwrap();

        lib.update_collection(&pl.id, &UpdateCollection::sortable(false))
            .unwrap();
        let err = lib
            .update_collection_track(&pl.id, &[tb.id().clone(), ta.id().clone()])
            .unwrap_err();
        assert!(matches!(err, LibraryError::Unsupported(_)));
    }

    #[test]
    fn delete_playlist_keeps_tracks() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("keep.wav");
        write_minimal_wav(&wav);

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let track = lib.import_path(&wav).unwrap();
        let pl = lib.add_collection(&NewCollection::playlist("Temp", true)).unwrap();
        lib.add_collection_track(&pl.id, track.id(), None).unwrap();
        lib.delete_collection(&pl.id).unwrap();

        assert!(lib.get_collection(&pl.id).unwrap().is_none());
        assert!(lib.get_track(track.id()).unwrap().is_some());
    }

    #[test]
    fn delete_folder_keeps_tracks() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_wav(&dir.path().join("t.wav"));

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let folder = lib.add_collection(&NewCollection::folder(dir.path())).unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let track_id = lib.get_collection_tracks(&folder.id).unwrap()[0].id().clone();

        lib.delete_collection(&folder.id).unwrap();
        assert!(lib.get_track(&track_id).unwrap().is_some());
    }

    #[test]
    fn merge_analyzed_metadata_force_overrides_tags() {
        let tags = TrackMetadata {
            bpm: Some(120.0),
            key: Some("Am".into()),
            ..TrackMetadata::default()
        };
        let merged = merge_analyzed_metadata(&tags, Some(128.0), Some("F#m"), true);
        assert_eq!(merged.bpm, Some(128.0));
        assert_eq!(merged.key.as_deref(), Some("F#m"));
    }

    #[test]
    fn merge_analyzed_metadata_keeps_tags_when_not_forced() {
        let tags = TrackMetadata {
            bpm: Some(120.0),
            key: Some("Am".into()),
            ..TrackMetadata::default()
        };
        let merged = merge_analyzed_metadata(&tags, Some(128.0), Some("F#m"), false);
        assert_eq!(merged.bpm, Some(120.0));
        assert_eq!(merged.key.as_deref(), Some("Am"));
    }

    #[test]
    fn merge_analyzed_metadata_fills_missing_when_not_forced() {
        let tags = TrackMetadata::default();
        let merged = merge_analyzed_metadata(&tags, Some(128.0), Some("F#m"), false);
        assert_eq!(merged.bpm, Some(128.0));
        assert_eq!(merged.key.as_deref(), Some("F#m"));
    }

    #[test]
    fn analyze_track_refreshes_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("song.wav");
        write_minimal_wav(&wav);

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let track = lib.import_path(&wav).unwrap();
        let analyzed = lib
            .analyze_track(track.id(), AnalyzeTrackOptions::default())
            .unwrap();

        assert_eq!(analyzed.metadata().title.as_deref(), Some("song"));
        assert_eq!(
            analyzed.metadata().sample_rate,
            track.metadata().sample_rate
        );
    }

    #[test]
    fn analyze_track_rejects_stream() {
        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let source = lib
            .import_stream(
                "https://example.com/track.mp3",
                &TrackMetadata::default(),
                Some(StreamProvider::Http),
            )
            .unwrap();
        let err = lib
            .analyze_track(source.id(), AnalyzeTrackOptions::default())
            .unwrap_err();
        assert!(matches!(err, LibraryError::Unsupported(_)));
    }

    #[test]
    fn remove_collection_track_drops_orphaned_stream() {
        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let mut meta = TrackMetadata::default();
        meta.title = Some("Remote Track".into());
        let source = lib
            .import_stream(
                "https://example.com/track.mp3",
                &meta,
                Some(StreamProvider::Http),
            )
            .unwrap();
        let pl = lib
            .add_collection(&NewCollection::playlist("Streaming", true))
            .unwrap();
        lib.add_collection_track(&pl.id, source.id(), None).unwrap();

        lib.remove_collection_track(&pl.id, source.id()).unwrap();
        assert!(lib.get_track(source.id()).unwrap().is_none());
    }

    #[test]
    fn remove_collection_track_keeps_folder_linked_file() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_wav(&dir.path().join("keep.wav"));

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let folder = lib.add_collection(&NewCollection::folder(dir.path())).unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let track_id = lib.get_collection_tracks(&folder.id).unwrap()[0].id().clone();

        let pl = lib.add_collection(&NewCollection::playlist("Also", true)).unwrap();
        lib.add_collection_track(&pl.id, &track_id, None).unwrap();
        lib.remove_collection_track(&pl.id, &track_id).unwrap();

        assert!(lib.get_track(&track_id).unwrap().is_some());
    }

    #[test]
    fn import_stream_round_trip() {
        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let mut meta = TrackMetadata::default();
        meta.title = Some("Remote Track".into());
        let source = lib
            .import_stream(
                "https://example.com/track.mp3",
                &meta,
                Some(StreamProvider::Http),
            )
            .unwrap();

        let LibrarySource::Stream(stream) = &source else {
            panic!("expected stream source");
        };
        assert_eq!(stream.uri(), "https://example.com/track.mp3");
        assert_eq!(source.metadata().title.as_deref(), Some("Remote Track"));

        let pl = lib
            .add_collection(&NewCollection::playlist("Streaming", true))
            .unwrap();
        lib.add_collection_track(&pl.id, source.id(), None).unwrap();
        assert_eq!(lib.get_collection_tracks(&pl.id).unwrap().len(), 1);
    }

    #[test]
    fn open_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("lib.db");
        let music = dir.path().join("music");
        std::fs::create_dir_all(&music).unwrap();
        write_minimal_wav(&music.join("p.wav"));

        {
            let mut lib = LibraryManager::open(&db, LibraryConfig::default()).unwrap();
            let folder = lib.add_collection(&NewCollection::folder(&music)).unwrap();
            lib.sync_collection(Some(&folder.id)).unwrap();
            let pl = lib.add_collection(&NewCollection::playlist("All", true)).unwrap();
            let tracks = lib.get_collection_tracks(&folder.id).unwrap();
            lib.add_collection_track(&pl.id, tracks[0].id(), None).unwrap();
        }

        let lib = LibraryManager::open(&db, LibraryConfig::default()).unwrap();
        assert_eq!(lib.list_collections().unwrap().len(), 2);
        let folder = lib
            .list_collections()
            .unwrap()
            .into_iter()
            .find(|c| c.collection_type() == CollectionType::Folder)
            .unwrap();
        assert_eq!(lib.get_collection_tracks(&folder.id).unwrap().len(), 1);
    }
}
