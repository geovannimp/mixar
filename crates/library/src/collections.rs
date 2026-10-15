//! Collection sync/import and filesystem bookkeeping.
use super::*;

impl LibraryManager {
    /// Sync collections without holding the library mutex across filesystem walks or tag reads.
    pub fn sync_collection_off_mutex(
        library: &Mutex<Self>,
        collection_id: Option<&CollectionId>,
    ) -> Result<ScanReport> {
        let collections = {
            let lib = Self::lock_library(library)?;
            match collection_id {
                Some(id) => vec![lib.require_collection(id)?],
                None => lib.list_collections()?,
            }
        };

        if collections.is_empty() {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "no collections to sync".into(),
            });
        }

        let mut report = ScanReport::default();
        for collection in collections {
            let part = Self::sync_one_collection_off_mutex(library, &collection)?;
            report.added += part.added;
            report.updated += part.updated;
            report.skipped += part.skipped;
            report.failed += part.failed;
            report.errors.extend(part.errors);
        }
        Ok(report)
    }
    pub(crate) fn sync_one_collection_off_mutex(
        library: &Mutex<Self>,
        collection: &Collection,
    ) -> Result<ScanReport> {
        match collection.collection_type() {
            CollectionType::Folder => Self::sync_folder_off_mutex(library, collection),
            CollectionType::Playlist => Self::sync_playlist_off_mutex(library, collection),
        }
    }

    pub(crate) fn sync_folder_off_mutex(
        library: &Mutex<Self>,
        folder: &Collection,
    ) -> Result<ScanReport> {
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

        let CollectionConfig::Folder {
            scan_folder_tree, ..
        } = &folder.config
        else {
            return Err(LibraryError::Backend {
                backend: "library",
                message: format!("folder {} has non-folder config", folder.id),
            });
        };

        let files = collect_audio_files(fs_path, *scan_folder_tree)?;
        for file in files {
            match Self::import_file_off_mutex(library, &file) {
                Ok(FileImportOutcome::Added) => report.added += 1,
                Ok(FileImportOutcome::Updated) => report.updated += 1,
                Err(LibraryError::UnsupportedFile(_)) => report.skipped += 1,
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(format!("{}: {err}", file.display()));
                }
            }
        }
        Ok(report)
    }

    pub(crate) fn sync_playlist_off_mutex(
        library: &Mutex<Self>,
        playlist: &Collection,
    ) -> Result<ScanReport> {
        let paths = {
            let lib = Self::lock_library(library)?;
            lib.list_collection_tracks(&playlist.id)?
                .into_iter()
                .filter_map(|source| source.file().map(|file| file.path().to_path_buf()))
                .collect::<Vec<_>>()
        };

        let mut report = ScanReport::default();
        for path in paths {
            if !path.is_file() {
                report.skipped += 1;
                continue;
            }
            match Self::refresh_file_off_mutex(library, &path) {
                Ok(()) => report.updated += 1,
                Err(LibraryError::UnsupportedFile(_)) => report.skipped += 1,
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(format!("{}: {err}", path.display()));
                }
            }
        }
        Ok(report)
    }

    pub(crate) fn import_file_off_mutex(
        library: &Mutex<Self>,
        path: &Path,
    ) -> Result<FileImportOutcome> {
        let path = normalize_path(path)?;
        if !path.is_file() {
            return Err(LibraryError::PathNotFound(path_label(&path)));
        }
        if !is_audio_file(&path) {
            return Err(LibraryError::UnsupportedFile(path_label(&path)));
        }
        let metadata = tags::read_tags(&path)?;
        let id = Self::track_id_for(&path);
        let lib = Self::lock_library(library)?;
        let existed = lib.get_track(&id)?.is_some();
        lib.upsert_file_source(&path, &metadata)?;
        Ok(if existed {
            FileImportOutcome::Updated
        } else {
            FileImportOutcome::Added
        })
    }

    pub(crate) fn refresh_file_off_mutex(library: &Mutex<Self>, path: &Path) -> Result<()> {
        let path = normalize_path(path)?;
        if !path.is_file() {
            return Err(LibraryError::PathNotFound(path_label(&path)));
        }
        if !is_audio_file(&path) {
            return Err(LibraryError::UnsupportedFile(path_label(&path)));
        }
        let metadata = tags::read_tags(&path)?;
        let lib = Self::lock_library(library)?;
        lib.upsert_file_source(&path, &metadata)?;
        Ok(())
    }
}

impl LibraryManager {
    pub(crate) fn track_id_for(path: &Path) -> TrackId {
        TrackId::new(path.to_string_lossy())
    }

    pub(crate) fn folder_id_for(path: &Path) -> CollectionId {
        CollectionId::new(format!("folder:{}", path.to_string_lossy()))
    }

    pub(crate) fn new_playlist_id() -> CollectionId {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        CollectionId::new(format!("playlist:{nanos}"))
    }

    pub(crate) fn upsert_file_source(
        &self,
        path: &Path,
        metadata: &TrackMetadata,
    ) -> Result<AudioSource> {
        let path = normalize_path(path)?;
        self.upsert_file_source_normalized(&path, metadata)
    }

    /// Write a file track for a path that is already normalized.
    pub(crate) fn upsert_file_source_normalized(
        &self,
        path: &Path,
        metadata: &TrackMetadata,
    ) -> Result<AudioSource> {
        let id = Self::track_id_for(path);
        let now = unix_secs_string();
        self.store().upsert_file_track(&id, path, metadata, &now)?;
        Ok(AudioSource::File(FileAudioSource::new(
            id,
            path.to_path_buf(),
            metadata.clone(),
        )))
    }

    #[allow(dead_code)]
    fn upsert_stream_source(
        &self,
        uri: &str,
        metadata: &TrackMetadata,
        provider: Option<StreamProvider>,
    ) -> Result<AudioSource> {
        let id = Self::stream_id_for(uri, provider);
        let now = unix_secs_string();
        let provider_str = provider.map(|p| p.as_str());
        self.store()
            .upsert_stream_track(&id, uri, metadata, provider_str, &now)?;
        Ok(AudioSource::Stream(StreamAudioSource::new(
            id,
            uri,
            metadata.clone(),
            provider,
        )))
    }

    #[allow(dead_code)]
    pub(crate) fn stream_id_for(uri: &str, provider: Option<StreamProvider>) -> TrackId {
        match provider {
            Some(p) => TrackId::new(format!("stream:{}:{uri}", p.as_str())),
            None => TrackId::new(format!("stream:{uri}")),
        }
    }

    pub(crate) fn require_collection(&self, id: &CollectionId) -> Result<Collection> {
        self.get_collection(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))
    }

    pub(crate) fn require_playlist(&self, id: &CollectionId) -> Result<Collection> {
        let c = self.require_collection(id)?;
        if c.collection_type() != CollectionType::Playlist {
            return Err(LibraryError::WrongCollectionType {
                expected: "playlist",
                got: c.collection_type().as_str(),
            });
        }
        Ok(c)
    }

    /// Collection entry rows in order with resolved sources (`None` when the track row is missing).
    pub fn collection_entries_with_sources(
        &self,
        collection_id: &CollectionId,
    ) -> Result<Vec<(CollectionEntry, Option<AudioSource>)>> {
        let rows = self.list_collection_entries(collection_id)?;
        let ids: Vec<TrackId> = rows.iter().map(|row| row.track_id.clone()).collect();
        let by_id = self.store().get_tracks_by_ids(&ids)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let source = by_id.get(row.track_id.as_str()).cloned();
                (row, source)
            })
            .collect())
    }

    /// Track ids in a collection selected for bulk analysis.
    ///
    /// With `force == false` (the dialog default) this applies the
    /// [`TrackMetadata::needs_analysis`] rule — tracks that already carry BPM/key
    /// from tags or an earlier run, and tracks with a `track_analysis` row, are
    /// skipped so re-running a batch is cheap and never overwrites curated tags.
    /// With `force == true` every file track is returned. Stream tracks and
    /// playlist entries whose track row is missing are always skipped.
    pub fn collection_analyze_targets(
        &self,
        collection_id: &CollectionId,
        force: bool,
    ) -> Result<Vec<TrackId>> {
        let sources = self.list_collection_tracks(collection_id)?;
        let mut targets = Vec::new();
        for source in &sources {
            let file = match source.file() {
                Some(file) => file,
                None => continue,
            };
            if !force {
                let analyzed = self.store().count_track_analysis(source.id())? > 0;
                if !source.metadata().needs_analysis(analyzed) {
                    continue;
                }
            }
            targets.push(file.id.clone());
        }
        Ok(targets)
    }

    /// File track ids in a collection (streams and missing playlist rows skipped).
    pub fn collection_file_track_ids(&self, collection_id: &CollectionId) -> Result<Vec<TrackId>> {
        let sources = self.list_collection_tracks(collection_id)?;
        Ok(sources
            .iter()
            .filter_map(|source| source.file().map(|file| file.id.clone()))
            .collect())
    }

    pub(crate) fn sync_one_collection(&mut self, collection: &Collection) -> Result<ScanReport> {
        match collection.collection_type() {
            CollectionType::Folder => self.sync_folder(collection),
            CollectionType::Playlist => self.sync_playlist(collection),
        }
    }

    pub(crate) fn sync_folder(&mut self, folder: &Collection) -> Result<ScanReport> {
        let fs_path = folder.fs_path().ok_or_else(|| LibraryError::Backend {
            backend: "library",
            message: format!("folder {} has no fs_path", folder.id),
        })?;

        let mut report = ScanReport::default();
        if !fs_path.exists() {
            report.failed += 1;
            report
                .errors
                .push(format!("root not found: {}", path_label(fs_path)));
            return Ok(report);
        }

        let CollectionConfig::Folder {
            scan_folder_tree, ..
        } = &folder.config
        else {
            return Err(LibraryError::Backend {
                backend: "library",
                message: format!("folder {} has non-folder config", folder.id),
            });
        };
        let files = collect_audio_files(fs_path, *scan_folder_tree)?;
        for file in files {
            match self.refresh_file_source_with_outcome(&file) {
                Ok((_, true)) => report.updated += 1,
                Ok((_, false)) => report.added += 1,
                Err(LibraryError::UnsupportedFile(_)) => report.skipped += 1,
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(format!("{}: {err}", path_label(&file)));
                }
            }
        }
        Ok(report)
    }

    pub(crate) fn sync_playlist(&mut self, playlist: &Collection) -> Result<ScanReport> {
        let sources = self.list_collection_tracks(&playlist.id)?;
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
            match self.refresh_file_source(file.path()) {
                Ok(_) => report.updated += 1,
                Err(LibraryError::UnsupportedFile(_)) => report.skipped += 1,
                Err(err) => {
                    report.failed += 1;
                    report
                        .errors
                        .push(format!("{}: {err}", path_label(file.path())));
                }
            }
        }
        Ok(report)
    }

    pub(crate) fn add_folder_collection(
        &mut self,
        collection: &NewCollection,
    ) -> Result<Collection> {
        let CollectionConfig::Folder {
            fs_path: path,
            scan_folder_tree,
        } = &collection.config
        else {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "folder collection requires Folder config".into(),
            });
        };
        let path = normalize_path(path)?;
        if !path.is_dir() {
            return Err(LibraryError::NotADirectory(path_label(&path)));
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

        self.store().insert_folder_collection(
            &id,
            &name,
            &path.to_string_lossy(),
            *scan_folder_tree,
        )?;

        Ok(Collection {
            id,
            name,
            config: CollectionConfig::Folder {
                fs_path: path,
                scan_folder_tree: *scan_folder_tree,
            },
        })
    }

    pub(crate) fn add_playlist_collection(
        &mut self,
        collection: &NewCollection,
    ) -> Result<Collection> {
        let CollectionConfig::Playlist { sortable } = collection.config else {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "playlist collection requires Playlist config".into(),
            });
        };
        let name = collection
            .name
            .as_deref()
            .ok_or_else(|| LibraryError::Backend {
                backend: "library",
                message: "playlist collection requires a name".into(),
            })?;

        let id = Self::new_playlist_id();
        self.store()
            .insert_playlist_collection(&id, name, sortable)?;

        Ok(Collection {
            id,
            name: name.to_string(),
            config: CollectionConfig::Playlist { sortable },
        })
    }

    pub(crate) fn file_sources_under(&self, root: &Path) -> Result<Vec<AudioSource>> {
        let root_str = root.to_string_lossy().into_owned();
        let prefix = format!("{}/%", escape_like(&root_str));
        self.store().find_file_sources_under(&root_str, &prefix)
    }

    /// Import or refresh a file track at `path` and return the library source.
    pub fn import_file_path(&self, path: &Path) -> Result<AudioSource> {
        self.refresh_file_source(path)
    }

    /// Look up an indexed file track for `path`, if it exists in the library DB.
    pub fn lookup_file_track_at_path(&self, path: &Path) -> Result<Option<AudioSource>> {
        if let Ok(normalized) = normalize_path(path) {
            let id = Self::track_id_for(&normalized);
            if let Some(source) = self.get_track(&id)? {
                return Ok(Some(source));
            }

            let normalized_ref = normalized.to_string_lossy().into_owned();
            if let Some(source) = self
                .store()
                .find_file_track_by_source_ref(&normalized_ref)?
            {
                return Ok(Some(source));
            }
        }

        let raw_ref = path.to_string_lossy().into_owned();
        if let Some(source) = self.get_track(&TrackId::new(&raw_ref))? {
            return Ok(Some(source));
        }

        self.store().find_file_track_by_source_ref(&raw_ref)
    }

    /// Resolve any library tracks that match the given filesystem paths.
    pub fn lookup_file_tracks_at_paths(
        &self,
        paths: &[PathBuf],
    ) -> Result<Vec<(String, AudioSource)>> {
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for path in paths {
            let request_path = path.to_string_lossy().into_owned();
            if !seen.insert(request_path.clone()) {
                continue;
            }

            let Some(source) = self.lookup_file_track_at_path(path)? else {
                continue;
            };

            results.push((request_path, source));
        }

        Ok(results)
    }

    #[allow(dead_code)]
    pub(crate) fn import_stream(
        &self,
        uri: &str,
        metadata: &TrackMetadata,
        provider: Option<StreamProvider>,
    ) -> Result<AudioSource> {
        if uri.is_empty() {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "stream uri must not be empty".into(),
            });
        }
        self.upsert_stream_source(uri, metadata, provider)
    }

    pub(crate) fn track_linked_to_collections(&self, track_id: &TrackId) -> Result<bool> {
        if self.store().count_playlist_links(track_id)? > 0 {
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

    pub(crate) fn remove_track_if_orphaned(&self, track_id: &TrackId) -> Result<()> {
        if self.track_linked_to_collections(track_id)? {
            return Ok(());
        }
        self.store().delete_track(track_id)?;
        Ok(())
    }

    /// After off-mutex work, confirm [id] still exists and still maps to [path].
    pub(crate) fn ensure_track_still_at_path(&self, id: &TrackId, path: &Path) -> Result<()> {
        let Some(source) = self.get_track(id)? else {
            return Err(LibraryError::NotFound(id.to_string()));
        };
        let file = source.file().ok_or(LibraryError::Unsupported(
            "stream track analysis not implemented",
        ))?;
        let current = normalize_path(file.path())?;
        let expected = normalize_path(path)?;
        if current != expected {
            return Err(LibraryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    pub(crate) fn refresh_file_source(&self, path: &Path) -> Result<AudioSource> {
        self.refresh_file_source_with_outcome(path)
            .map(|(source, _)| source)
    }

    fn refresh_file_source_with_outcome(&self, path: &Path) -> Result<(AudioSource, bool)> {
        let path = normalize_path(path)?;
        if !path.is_file() {
            return Err(LibraryError::PathNotFound(path_label(&path)));
        }
        if !is_audio_file(&path) {
            return Err(LibraryError::UnsupportedFile(path_label(&path)));
        }
        let id = Self::track_id_for(&path);
        let existed = self.store().track_exists(&id)?;
        let metadata = tags::read_tags(&path)?;
        let source = self.upsert_file_source(&path, &metadata)?;
        Ok((source, existed))
    }

    #[cfg(feature = "analysis")]
    pub(crate) fn analyze_file_source(
        &self,
        path: &Path,
        options: AnalyzeTrackOptions,
    ) -> Result<AudioSource> {
        let computed = compute_file_analysis(path, options)?;
        self.persist_file_analysis(path, &computed, true)
    }
}

impl WritableLibrary for LibraryManager {
    fn import_track(&mut self, path: &Path, metadata: &TrackMetadata) -> Result<ImportedTrack> {
        let path = normalize_path(path)?;
        let id = Self::track_id_for(&path);
        let created = !self.store().track_exists(&id)?;
        let source = self.upsert_file_source_normalized(&path, metadata)?;
        Ok(ImportedTrack { source, created })
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

    fn analyze_track(&mut self, id: &TrackId, options: AnalyzeTrackOptions) -> Result<AudioSource> {
        let source = self
            .get_track(id)?
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;
        match source {
            AudioSource::File(file) => self.analyze_file_source(file.path(), options),
            AudioSource::Stream(_) => Err(LibraryError::Unsupported(
                "stream track analysis not implemented",
            )),
        }
    }

    fn update_collection(&mut self, id: &CollectionId, update: &UpdateCollection) -> Result<()> {
        if update.name.is_none() && update.config.is_none() {
            return Ok(());
        }

        if let Some(name) = &update.name {
            if !self.store().update_collection_name(id, name)? {
                return Err(LibraryError::NotFound(id.to_string()));
            }
        }

        if let Some(CollectionConfigUpdate::Playlist { sortable }) = update.config {
            let playlist = self.require_playlist(id)?;
            if playlist.sortable() != sortable {
                self.store().update_collection_sortable(id, sortable)?;

                if sortable {
                    let rows = self.store().list_collection_entries(id)?;
                    for (pos, row) in rows.iter().enumerate() {
                        self.store()
                            .set_collection_entry_position_by_id(id, &row.id, pos as i32)?;
                    }
                } else {
                    self.store().dedupe_playlist_entries(id)?;
                    self.store().clear_playlist_positions(id)?;
                }
            }
        }

        Ok(())
    }

    fn delete_collection(&mut self, id: &CollectionId) -> Result<()> {
        if !self.store().delete_collection(id)? {
            return Err(LibraryError::NotFound(id.to_string()));
        }
        Ok(())
    }

    fn add_collection_entry(
        &mut self,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: Option<i32>,
    ) -> Result<CollectionEntryId> {
        let playlist = self.require_playlist(collection_id)?;
        if self.get_track(track_id)?.is_none() {
            return Err(LibraryError::NotFound(track_id.to_string()));
        }

        let position = if playlist.sortable() {
            Some(position.unwrap_or_else(|| {
                self.list_collection_entries(collection_id)
                    .map(|rows| rows.len() as i32)
                    .unwrap_or(0)
            }))
        } else {
            None
        };

        // Sortable playlists may list the same track more than once; crates are a set.
        if playlist.sortable() {
            self.store()
                .insert_collection_entry(collection_id, track_id, position)
        } else {
            self.store()
                .upsert_collection_entry(collection_id, track_id, position)
        }
    }

    fn remove_collection_entry(
        &mut self,
        collection_id: &CollectionId,
        entry_id: &CollectionEntryId,
    ) -> Result<()> {
        let _ = self.require_playlist(collection_id)?;
        let Some(track_id) = self
            .store()
            .delete_collection_entry_by_id(collection_id, entry_id)?
        else {
            return Err(LibraryError::NotFound(format!(
                "{collection_id}/{entry_id}"
            )));
        };
        self.remove_track_if_orphaned(&track_id)?;
        Ok(())
    }

    fn update_collection_entries(
        &mut self,
        collection_id: &CollectionId,
        entry_ids: &[CollectionEntryId],
    ) -> Result<()> {
        let playlist = self.require_playlist(collection_id)?;
        if !playlist.sortable() {
            return Err(LibraryError::Unsupported(
                "reorder requires a sortable playlist",
            ));
        }

        let existing = self.list_collection_entries(collection_id)?;
        if existing.len() != entry_ids.len() {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "update_collection_entries must include exactly the playlist membership"
                    .into(),
            });
        }
        let existing_ids: std::collections::HashSet<_> =
            existing.iter().map(|row| row.id.as_str()).collect();
        let mut seen = std::collections::HashSet::new();
        if entry_ids.len() != existing_ids.len()
            || entry_ids
                .iter()
                .any(|id| !seen.insert(id.as_str()) || !existing_ids.contains(id.as_str()))
        {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "update_collection_entries must include exactly the playlist membership"
                    .into(),
            });
        }

        for (pos, entry_id) in entry_ids.iter().enumerate() {
            self.store().set_collection_entry_position_by_id(
                collection_id,
                entry_id,
                pos as i32,
            )?;
        }
        Ok(())
    }
}
