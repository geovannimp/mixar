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

#[cfg(feature = "analysis")]
mod analysis;
mod db;
mod deck_data;
mod entity;
mod model;
mod sampler_data;
mod store;
mod tags;
mod waveform;

#[cfg(feature = "analysis")]
use analyzer::{analyze_file, merge_track_metadata, AnalysisConfig, TagMetadata};
#[cfg(feature = "analysis")]
use analyzer_core::loudness_lufs_from_replaygain_track_gain_db;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use library_core::{
    is_supported_audio_extension, is_supported_audio_path, AnalyzeTrackOptions, AudioSource,
    Collection, CollectionConfig, CollectionConfigUpdate, CollectionId, CollectionTrack,
    CollectionType, FileAudioSource, Library, LibraryConfig, LibraryError, LoadableAudio,
    LoadedAudio, NewCollection, Result, ScanReport, StreamAudioSource, StreamProvider, TrackId,
    TrackMetadata, UpdateCollection, WritableLibrary,
};

pub use deck_data::{
    delete_hot_cue, delete_loop, list_hot_cues, list_loops, save_hot_cue, save_loop, HotCueRecord,
    LoopRecord,
};
pub use sampler_data::{
    assign_slot as assign_sampler_slot, clear_slot as clear_sampler_slot, create_bank, delete_bank,
    get_bank, get_track_last_sampler_bank_id, list_banks, list_slots as list_sampler_slots,
    set_track_last_sampler_bank_id, update_bank, SamplerBankRecord, SamplerPlayMode,
    SamplerSlotRecord, BANK_SIZE as SAMPLER_BANK_SIZE,
};
pub use tags::read_artwork;
pub use waveform::{BeatGridSnapshot, TrackWaveformOverview};

/// Library-owned playback handoff for engine/sampler consumers.
#[derive(Clone)]
pub struct PreparedTrackPlayback {
    pub track_id: TrackId,
    pub source: AudioSource,
    pub audio: Arc<LoadedAudio>,
    pub loudness_lufs: Option<f64>,
}

/// The user’s library manager (canonical writable store).
pub struct LibraryManager {
    db: db::Db,
    config: LibraryConfig,
    decode_cache: Mutex<HashMap<TrackId, Arc<LoadedAudio>>>,
}

impl LibraryManager {
    /// Open (or create) a library database at `db_path`.
    pub fn open(db_path: impl AsRef<Path>, config: LibraryConfig) -> Result<Self> {
        Ok(Self {
            db: db::open(db_path.as_ref())?,
            config,
            decode_cache: Mutex::new(HashMap::new()),
        })
    }

    /// Open an in-memory library (for tests).
    pub fn open_in_memory(config: LibraryConfig) -> Result<Self> {
        Ok(Self {
            db: db::open_in_memory()?,
            config,
            decode_cache: Mutex::new(HashMap::new()),
        })
    }

    fn store(&self) -> store::Store<'_> {
        store::Store::new(&self.db)
    }

    /// Borrow the library configuration.
    pub fn config(&self) -> &LibraryConfig {
        &self.config
    }

    /// Replace the library configuration.
    pub fn set_config(&mut self, config: LibraryConfig) {
        self.config = config;
    }

    /// Load hot cues and saved loops for a track.
    pub fn list_track_hot_cues(&self, id: &TrackId) -> Result<Vec<HotCueRecord>> {
        deck_data::list_hot_cues(&self.db, id)
    }

    /// Read embedded album artwork bytes for a library track, if present.
    pub fn get_track_artwork(&self, id: &TrackId) -> Result<Option<Vec<u8>>> {
        let Some(source) = self.get_track(id)? else {
            return Ok(None);
        };
        let Some(file) = source.file() else {
            return Ok(None);
        };
        tags::read_artwork(file.path())
    }

    pub fn list_track_loops(&self, id: &TrackId) -> Result<Vec<LoopRecord>> {
        deck_data::list_loops(&self.db, id)
    }

    pub fn save_track_hot_cue(
        &self,
        id: &TrackId,
        slot_index: u8,
        position_ms: i32,
        loop_length_beats: Option<i32>,
        color: Option<String>,
        label: Option<String>,
    ) -> Result<()> {
        deck_data::save_hot_cue(
            &self.db,
            id,
            slot_index,
            position_ms,
            loop_length_beats,
            color,
            label,
        )
    }

    pub fn delete_track_hot_cue(&self, id: &TrackId, slot_index: u8) -> Result<()> {
        deck_data::delete_hot_cue(&self.db, id, slot_index)
    }

    pub fn save_track_loop(
        &self,
        id: &TrackId,
        slot_index: u8,
        in_ms: i32,
        out_ms: i32,
        label: Option<String>,
        color: Option<String>,
    ) -> Result<()> {
        deck_data::save_loop(&self.db, id, slot_index, in_ms, out_ms, label, color)
    }

    pub fn delete_track_loop(&self, id: &TrackId, slot_index: u8) -> Result<()> {
        deck_data::delete_loop(&self.db, id, slot_index)
    }

    pub fn list_sampler_banks(&self) -> Result<Vec<SamplerBankRecord>> {
        sampler_data::list_banks(&self.db)
    }

    pub fn get_sampler_bank(&self, bank_id: &str) -> Result<Option<SamplerBankRecord>> {
        sampler_data::get_bank(&self.db, bank_id)
    }

    pub fn create_sampler_bank(
        &self,
        name: &str,
        play_mode: Option<SamplerPlayMode>,
    ) -> Result<SamplerBankRecord> {
        sampler_data::create_bank(&self.db, name, play_mode)
    }

    pub fn update_sampler_bank(
        &self,
        bank_id: &str,
        name: &str,
        play_mode: Option<SamplerPlayMode>,
    ) -> Result<()> {
        sampler_data::update_bank(&self.db, bank_id, name, play_mode)
    }

    pub fn delete_sampler_bank(&self, bank_id: &str) -> Result<()> {
        sampler_data::delete_bank(&self.db, bank_id)
    }

    pub fn list_sampler_bank_slots(&self, bank_id: &str) -> Result<Vec<SamplerSlotRecord>> {
        sampler_data::list_slots(&self.db, bank_id)
    }

    pub fn assign_sampler_bank_slot(
        &self,
        bank_id: &str,
        slot_index: u8,
        track_id: Option<String>,
        path: Option<String>,
        label: Option<String>,
    ) -> Result<()> {
        sampler_data::assign_slot(&self.db, bank_id, slot_index, track_id, path, label)
    }

    pub fn clear_sampler_bank_slot(&self, bank_id: &str, slot_index: u8) -> Result<()> {
        sampler_data::clear_slot(&self.db, bank_id, slot_index)
    }

    /// Bank last used with this track (set when a sampler pad is triggered while the track is loaded).
    pub fn get_track_last_sampler_bank_id(&self, id: &TrackId) -> Result<Option<String>> {
        sampler_data::get_track_last_sampler_bank_id(&self.db, id)
    }

    /// Remember which sampler bank was active when a pad was triggered for this track.
    pub fn set_track_last_sampler_bank_id(
        &self,
        id: &TrackId,
        bank_id: Option<&str>,
    ) -> Result<()> {
        sampler_data::set_track_last_sampler_bank_id(&self.db, id, bank_id)
    }

    /// Load the stored L0 waveform overview for a track, if present.
    pub fn get_track_waveform_overview(
        &self,
        id: &TrackId,
    ) -> Result<Option<waveform::TrackWaveformOverview>> {
        waveform::get_track_waveform_row(&self.db, id)
    }

    /// Generate and persist the overview when missing (e.g. first deck load).
    ///
    /// Takes `&Mutex<Self>` so overview generation does not hold the library lock.
    pub fn ensure_track_waveform(library: &Mutex<Self>, id: &TrackId) -> Result<()> {
        let path = {
            let lib = library.lock().expect("library lock");
            if waveform::has_track_waveform(&lib.db, id)? {
                return Ok(());
            }
            lib.get_track(id)?
                .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
                .file()
                .ok_or(LibraryError::Unsupported("stream tracks have no waveform"))?
                .path()
                .to_path_buf()
        };

        let peaks = waveform::generate_overview_from_path(&path)?;

        let lib = library.lock().expect("library lock");
        if waveform::has_track_waveform(&lib.db, id)? {
            return Ok(());
        }
        waveform::store_overview(&lib.db, id, &peaks)
    }

    /// Beat grid overlay data when the track has been analyzed.
    pub fn get_track_beat_grid(&self, id: &TrackId) -> Result<Option<waveform::BeatGridSnapshot>> {
        waveform::get_track_beat_grid(&self.db, id)
    }

    /// Read the stored integrated loudness for a track, if it has been analyzed.
    pub fn track_loudness_lufs(&self, id: &TrackId) -> Result<Option<f64>> {
        self.store().track_analysis_loudness(id)
    }

    /// Import/refresh a file, ensure library-managed playback metadata, and return a cached decode.
    ///
    /// Takes `&Mutex<Self>` so decode / waveform work does not hold the library lock.
    pub fn prepare_file_path_for_playback(
        library: &Mutex<Self>,
        path: &Path,
    ) -> Result<PreparedTrackPlayback> {
        let source = {
            let lib = library.lock().expect("library lock");
            lib.import_file_path(path)?
        };
        Self::prepare_source_for_playback(library, source)
    }

    /// Resolve a library track, ensure library-managed playback metadata, and return a cached decode.
    ///
    /// Takes `&Mutex<Self>` so decode / waveform work does not hold the library lock.
    pub fn prepare_track_for_playback(
        library: &Mutex<Self>,
        id: &TrackId,
    ) -> Result<PreparedTrackPlayback> {
        let source = {
            let lib = library.lock().expect("library lock");
            lib.get_track(id)?
                .ok_or_else(|| LibraryError::NotFound(id.to_string()))?
        };
        Self::prepare_source_for_playback(library, source)
    }

    fn prepare_source_for_playback(
        library: &Mutex<Self>,
        mut source: AudioSource,
    ) -> Result<PreparedTrackPlayback> {
        let track_id = source.id().clone();
        if source.file().is_some() {
            Self::ensure_track_waveform(library, &track_id)?;
        }
        let loudness_lufs = {
            let lib = library.lock().expect("library lock");
            lib.track_loudness_lufs(&track_id)?
        };
        source.metadata_mut().loudness_lufs = loudness_lufs;

        let cached = {
            let lib = library.lock().expect("library lock");
            let cache = lib.decode_cache.lock().expect("library decode cache lock");
            cache.get(&track_id).map(Arc::clone)
        };

        let audio = if let Some(cached) = cached {
            cached
        } else {
            // Decode without holding LibraryManager so artwork/DB reads can proceed.
            let loaded = Arc::new(source.load().map_err(|e| LibraryError::Backend {
                backend: "library",
                message: format!("failed to decode track for playback: {e}"),
            })?);
            let lib = library.lock().expect("library lock");
            let mut cache = lib.decode_cache.lock().expect("library decode cache lock");
            if let Some(existing) = cache.get(&track_id) {
                Arc::clone(existing)
            } else {
                cache.insert(track_id.clone(), Arc::clone(&loaded));
                loaded
            }
        };

        Ok(PreparedTrackPlayback {
            track_id,
            source,
            audio,
            loudness_lufs,
        })
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

    fn upsert_file_source(&self, path: &Path, metadata: &TrackMetadata) -> Result<AudioSource> {
        let path = normalize_path(path)?;
        let id = Self::track_id_for(&path);
        let now = now_stamp();
        self.store().upsert_file_track(&id, &path, metadata, &now)?;
        Ok(AudioSource::File(FileAudioSource::new(
            id,
            path,
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
        let now = now_stamp();
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
    fn stream_id_for(uri: &str, provider: Option<StreamProvider>) -> TrackId {
        match provider {
            Some(p) => TrackId::new(format!("stream:{}:{uri}", p.as_str())),
            None => TrackId::new(format!("stream:{uri}")),
        }
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
            match self.refresh_file_source(file.path()) {
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

        self.store()
            .insert_folder_collection(&id, &name, &path.to_string_lossy())?;

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

    fn playlist_track_ids(&self, playlist_id: &CollectionId) -> Result<Vec<TrackId>> {
        self.store().playlist_track_ids(playlist_id)
    }

    fn file_sources_under(&self, root: &Path) -> Result<Vec<AudioSource>> {
        let root_str = root.to_string_lossy().into_owned();
        let prefix = format!("{}/%", escape_like(&root_str));
        self.store().find_file_sources_under(&root_str, &prefix)
    }

    pub(crate) fn import_path(&self, path: &Path) -> Result<AudioSource> {
        self.refresh_file_source(path)
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

    fn track_linked_to_collections(&self, track_id: &TrackId) -> Result<bool> {
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

    fn remove_track_if_orphaned(&self, track_id: &TrackId) -> Result<()> {
        if self.track_linked_to_collections(track_id)? {
            return Ok(());
        }
        self.store().delete_track(track_id)?;
        Ok(())
    }

    fn refresh_file_source(&self, path: &Path) -> Result<AudioSource> {
        let path = normalize_path(path)?;
        if !path.is_file() {
            return Err(LibraryError::PathNotFound(path));
        }
        if !is_audio_file(&path) {
            return Err(LibraryError::UnsupportedFile(path));
        }
        let metadata = tags::read_tags(&path)?;
        self.upsert_file_source(&path, &metadata)
    }

    #[cfg(feature = "analysis")]
    fn analyze_file_source(
        &self,
        path: &Path,
        options: AnalyzeTrackOptions,
    ) -> Result<AudioSource> {
        let path = normalize_path(path)?;
        if !path.is_file() {
            return Err(LibraryError::PathNotFound(path));
        }
        if !is_audio_file(&path) {
            return Err(LibraryError::UnsupportedFile(path));
        }

        let mut config = AnalysisConfig::default();
        let tag_metadata = tags::read_tags(&path)?;
        config.max_duration_secs = options
            .analysis_duration
            .resolve_max_duration_secs(tag_metadata.duration_ms.map(audio_core::ms_to_secs));
        let mut analysis = analyze_file(&path, &config).map_err(analysis::analyzer_error)?;
        let replaygain_track_gain_db = tags::read_replaygain_track_gain_db(&path)?;
        analysis.loudness_lufs =
            preferred_loudness_lufs(analysis.loudness_lufs, replaygain_track_gain_db);

        let tag_side = TagMetadata {
            bpm: tag_metadata.bpm,
            key: tag_metadata.key.clone(),
        };
        let merged = merge_track_metadata(
            &tag_side,
            &analysis,
            options.force,
            config.min_bpm_confidence,
            config.min_key_confidence,
        );

        let mut metadata = tag_metadata;
        metadata.bpm = merged.bpm;
        metadata.key = merged.key;

        let source = self.upsert_file_source(&path, &metadata)?;
        analysis::upsert_track_analysis(&self.db, source.id(), &analysis)?;
        waveform::generate_and_store_overview(&self.db, source.id(), &path)?;
        Ok(source)
    }

    #[cfg(not(feature = "analysis"))]
    fn analyze_file_source(
        &self,
        path: &Path,
        _options: AnalyzeTrackOptions,
    ) -> Result<AudioSource> {
        self.refresh_file_source(path)
    }
}

#[cfg(feature = "analysis")]
fn preferred_loudness_lufs(measured_lufs: Option<f64>, replaygain_db: Option<f64>) -> Option<f64> {
    replaygain_db
        .map(loudness_lufs_from_replaygain_track_gain_db)
        .or(measured_lufs)
}

impl Library for LibraryManager {
    fn name(&self) -> &'static str {
        "library"
    }

    fn get_track(&self, id: &TrackId) -> Result<Option<AudioSource>> {
        self.store().get_track(id)
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        self.store().list_collections()
    }

    fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>> {
        self.store().get_collection(id)
    }

    fn get_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<AudioSource>> {
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
                    let ids = self.store().playlist_track_ids_by_track_id(id)?;
                    for (pos, track_id) in ids.iter().enumerate() {
                        self.store()
                            .set_collection_track_position(id, track_id, pos as i32)?;
                    }
                } else {
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

        self.store()
            .upsert_collection_track(collection_id, track_id, position)?;
        Ok(())
    }

    fn remove_collection_track(
        &mut self,
        collection_id: &CollectionId,
        track_id: &TrackId,
    ) -> Result<()> {
        let _ = self.require_playlist(collection_id)?;
        if !self
            .store()
            .delete_collection_track(collection_id, track_id)?
        {
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
        if existing.len() != track_ids.len() || track_ids.iter().any(|id| !existing.contains(id)) {
            return Err(LibraryError::Backend {
                backend: "library",
                message: "update_collection_track must include exactly the playlist membership"
                    .into(),
            });
        }

        for (pos, track_id) in track_ids.iter().enumerate() {
            self.store()
                .set_collection_track_position(collection_id, track_id, pos as i32)?;
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
    is_supported_audio_path(path)
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

    /// Mono 16-bit PCM sine (~3s @ 48 kHz) so EBU R128 loudness stays finite.
    #[cfg(feature = "analysis")]
    fn write_analysis_wav(path: &Path) {
        let sample_rate = 48_000u32;
        let duration_secs = 3u32;
        let sample_count = (sample_rate * duration_secs) as usize;
        let mut pcm = Vec::with_capacity(sample_count * 2);
        for index in 0..sample_count {
            let time = index as f32 / sample_rate as f32;
            let sample =
                (0.25 * (2.0 * std::f32::consts::PI * 440.0 * time).sin() * i16::MAX as f32) as i16;
            pcm.extend_from_slice(&sample.to_le_bytes());
        }
        let data_size = pcm.len() as u32;
        let file_size = 36 + data_size;
        let byte_rate = sample_rate * 2;
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&file_size.to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
        file.write_all(&1u16.to_le_bytes()).unwrap(); // mono
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&byte_rate.to_le_bytes()).unwrap();
        file.write_all(&2u16.to_le_bytes()).unwrap(); // block align
        file.write_all(&16u16.to_le_bytes()).unwrap(); // bits
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();
        file.write_all(&pcm).unwrap();
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
        assert_eq!(fetched.file().unwrap().path(), track.file().unwrap().path());
    }

    #[test]
    fn prepare_track_for_playback_reuses_cached_decode() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("track.wav");
        write_minimal_wav(&wav);

        let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
        let track_id = {
            let lib = library.lock().unwrap();
            lib.import_path(&wav).unwrap().id().clone()
        };

        let first = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
        let second = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();

        assert_eq!(first.track_id, track_id);
        assert_eq!(second.track_id, track_id);
        assert!(std::sync::Arc::ptr_eq(&first.audio, &second.audio));
    }

    #[test]
    fn prepare_track_for_playback_stores_waveform_without_holding_lock_across_generate() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("track.wav");
        write_minimal_wav(&wav);

        let library = Mutex::new(LibraryManager::open_in_memory(LibraryConfig::default()).unwrap());
        let track_id = {
            let lib = library.lock().unwrap();
            lib.import_path(&wav).unwrap().id().clone()
        };

        assert!(library
            .lock()
            .unwrap()
            .get_track_waveform_overview(&track_id)
            .unwrap()
            .is_none());

        let prepared = LibraryManager::prepare_track_for_playback(&library, &track_id).unwrap();
        assert_eq!(prepared.track_id, track_id);
        assert!(library
            .lock()
            .unwrap()
            .get_track_waveform_overview(&track_id)
            .unwrap()
            .is_some());
    }

    #[test]
    fn lookup_file_track_at_path_finds_imported_track() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("track.wav");
        write_minimal_wav(&wav);

        let lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let track = lib.import_path(&wav).unwrap();

        let found = lib.lookup_file_track_at_path(&wav).unwrap().unwrap();
        assert_eq!(found.id(), track.id());
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
        let folder = lib
            .add_collection(&NewCollection::folder_named(dir.path(), "Music"))
            .unwrap();
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

        let pl = lib
            .add_collection(&NewCollection::playlist("Warmup", true))
            .unwrap();
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
        let pl = lib
            .add_collection(&NewCollection::playlist("Crate", false))
            .unwrap();
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
        let pl = lib
            .add_collection(&NewCollection::playlist("Set", false))
            .unwrap();
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
        let pl = lib
            .add_collection(&NewCollection::playlist("Temp", true))
            .unwrap();
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
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let track_id = lib.get_collection_tracks(&folder.id).unwrap()[0]
            .id()
            .clone();

        lib.delete_collection(&folder.id).unwrap();
        assert!(lib.get_track(&track_id).unwrap().is_some());
    }

    #[test]
    #[cfg(feature = "analysis")]
    fn replaygain_loudness_replaces_measured_loudness() {
        let loudness = preferred_loudness_lufs(Some(-12.0), Some(3.2));
        assert_eq!(loudness, Some(-21.2));
        assert_eq!(preferred_loudness_lufs(Some(-12.0), None), Some(-12.0));
    }

    #[test]
    #[cfg(feature = "analysis")]
    fn analyze_track_persists_track_analysis() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("song.wav");
        write_analysis_wav(&wav);

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let track = lib.import_path(&wav).unwrap();
        lib.analyze_track(track.id(), AnalyzeTrackOptions::default())
            .unwrap();

        let count = lib.store().count_track_analysis(track.id()).unwrap();
        assert_eq!(count, 1);
        assert!(lib.track_loudness_lufs(track.id()).unwrap().is_some());
    }

    #[test]
    #[cfg(feature = "analysis")]
    fn analyze_track_persists_waveform_overview() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("song.wav");
        write_analysis_wav(&wav);

        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let track = lib.import_path(&wav).unwrap();
        assert!(lib
            .get_track_waveform_overview(track.id())
            .unwrap()
            .is_none());

        lib.analyze_track(track.id(), AnalyzeTrackOptions::default())
            .unwrap();

        let overview = lib
            .get_track_waveform_overview(track.id())
            .unwrap()
            .expect("waveform overview should exist after analyze");
        assert_eq!(overview.overview_count, audio_core::OVERVIEW_SAMPLE_COUNT);
        assert_eq!(overview.peaks.len(), audio_core::OVERVIEW_SAMPLE_COUNT);

        // Idempotent: ensure after analyze must not fail or clear the overview.
        let track_id = track.id().clone();
        let library = Mutex::new(lib);
        LibraryManager::ensure_track_waveform(&library, &track_id).unwrap();
        assert!(library
            .lock()
            .unwrap()
            .get_track_waveform_overview(&track_id)
            .unwrap()
            .is_some());
    }

    #[test]
    fn analyze_track_refreshes_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("song.wav");
        write_analysis_wav(&wav);

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
        let meta = TrackMetadata {
            title: Some("Remote Track".into()),
            ..Default::default()
        };
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
        let folder = lib
            .add_collection(&NewCollection::folder(dir.path()))
            .unwrap();
        lib.sync_collection(Some(&folder.id)).unwrap();
        let track_id = lib.get_collection_tracks(&folder.id).unwrap()[0]
            .id()
            .clone();

        let pl = lib
            .add_collection(&NewCollection::playlist("Also", true))
            .unwrap();
        lib.add_collection_track(&pl.id, &track_id, None).unwrap();
        lib.remove_collection_track(&pl.id, &track_id).unwrap();

        assert!(lib.get_track(&track_id).unwrap().is_some());
    }

    #[test]
    fn import_stream_round_trip() {
        let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
        let meta = TrackMetadata {
            title: Some("Remote Track".into()),
            ..Default::default()
        };
        let source = lib
            .import_stream(
                "https://example.com/track.mp3",
                &meta,
                Some(StreamProvider::Http),
            )
            .unwrap();

        let AudioSource::Stream(stream) = &source else {
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
            let pl = lib
                .add_collection(&NewCollection::playlist("All", true))
                .unwrap();
            let tracks = lib.get_collection_tracks(&folder.id).unwrap();
            lib.add_collection_track(&pl.id, tracks[0].id(), None)
                .unwrap();
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
