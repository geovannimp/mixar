//! Library manager for Mixar.
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
//! let tracks = lib.list_collection_tracks(&folder.id)?;
//! # Ok::<(), library::LibraryError>(())
//! ```

#[cfg(feature = "analysis")]
mod analysis;
mod bus;
mod collections;
mod db;
mod deck_data;
mod decode_cache;
mod entity;
mod export;
mod history;
mod model;
mod playback;
mod sampler_data;
mod session;
mod stems;
mod store;
mod tags;
mod waveform;
mod worker;

#[cfg(feature = "analysis")]
use crate::analysis::{compute_file_analysis, ComputedFileAnalysis};
#[cfg(feature = "analysis")]
#[cfg(feature = "analysis")]
use codec::decode_stem_file;
use codec::is_loadable_audio_path;

use decode_cache::DecodeCache;
use library_api::{EvtBody, Kind, Origin};
use library_core::AnalysisDurationMode;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub use library_core::{
    path_label, path_under_folder, AnalyzeTrackOptions, AudioSource, Collection, CollectionConfig,
    CollectionConfigUpdate, CollectionEntry, CollectionEntryId, CollectionId, CollectionType,
    FileAudioSource, ImportedTrack, Library, LibraryConfig, LibraryError, LoadableAudio,
    LoadedAudio, Migratable, MigrateOptions, MigrateReport, NewCollection, Result, ScanReport,
    StreamAudioSource, StreamProvider, TagWriteBack, TrackId, TrackMetadata, UpdateCollection,
    WritableLibrary,
};

pub use bus::{Evt, EvtReceiver, LibraryBus, LibraryBuses};
pub use deck_data::{
    delete_hot_cue, delete_loop, list_hot_cues, list_loops, save_hot_cue, save_loop, HotCueRecord,
    LoopRecord,
};
pub use export::ExportSession;
pub use history::{
    crossfader_gain, export_document, history_dir_for_db, DeckPlaySnapshot, HistoryDocument,
    HistoryEntry, HistoryExportFormat, HistoryRecorder, HistoryRestorePrompt, HistorySessionRow,
    HistorySettings,
};
pub use sampler_data::{
    assign_slot as assign_sampler_slot, clear_slot as clear_sampler_slot, create_bank, delete_bank,
    get_bank, get_track_last_sampler_bank_id, list_banks, list_slots as list_sampler_slots,
    set_track_last_sampler_bank_id, update_bank, SamplerBankRecord, SamplerPlayMode,
    SamplerSlotRecord, BANK_SIZE as SAMPLER_BANK_SIZE,
};
pub use session::LibrarySession;
pub use stems::{
    clear_all_track_stems, clear_model_cache, dir_size, ensure_track_stems,
    ensure_track_stems_with_progress, sqlite_db_bytes, stem_io_message, sync_stem_cache_with_db,
    StemProgressFn, TrackStemsInfo,
};
pub use store::TrackMetadataEdit;
pub use tags::{read_artwork, ARTWORK_UI_EDGE_PX};
pub use waveform::{
    clear_all_track_waveforms, waveform_cache_bytes, BeatGridSnapshot, TrackWaveformOverview,
};
pub use worker::{spawn_library_worker, LibraryWorker};

/// Library-owned playback handoff for engine/sampler consumers.
#[derive(Clone)]
pub struct PreparedTrackPlayback {
    pub track_id: TrackId,
    pub source: AudioSource,
    pub audio: Arc<LoadedAudio>,
    /// NI order (drums, bass, other, vocals) when a native Stem or valid cache was resolved.
    pub stems: Option<[Arc<LoadedAudio>; 4]>,
    pub loudness_lufs: Option<f64>,
}

enum FileImportOutcome {
    Added,
    Updated,
}

/// The user’s library manager (canonical writable store).
pub struct LibraryManager {
    db: db::Db,
    history: HistoryRecorder,
    config: LibraryConfig,
    decode_cache: Mutex<DecodeCache>,
    /// Serializes file tag writes, which run outside this manager's own lock.
    tag_write_gate: Arc<Mutex<()>>,
    /// Host-injected omnibus buses (None until [`Self::set_buses`]).
    buses: Option<LibraryBuses>,
}

impl LibraryManager {
    /// Open (or create) a library database at `db_path`.
    pub fn open(db_path: impl AsRef<Path>, config: LibraryConfig) -> Result<Self> {
        let path = db_path.as_ref().to_path_buf();
        let db = db::open(&path)?;
        let mut history = HistoryRecorder::new(&path, HistorySettings::default())?;
        history.bootstrap(&db)?;
        Ok(Self {
            db,
            history,
            config,
            decode_cache: Mutex::new(DecodeCache::new()),
            tag_write_gate: Arc::new(Mutex::new(())),
            buses: None,
        })
    }

    /// Open an in-memory library (for tests).
    pub fn open_in_memory(config: LibraryConfig) -> Result<Self> {
        let db = db::open_in_memory()?;
        let path = PathBuf::from(":memory:");
        let mut history = HistoryRecorder::new(&path, HistorySettings::default())?;
        history.bootstrap(&db)?;
        Ok(Self {
            db,
            history,
            config,
            decode_cache: Mutex::new(DecodeCache::new()),
            tag_write_gate: Arc::new(Mutex::new(())),
            buses: None,
        })
    }

    fn store(&self) -> store::Store<'_> {
        store::Store::new(&self.db)
    }

    /// Attach host-owned omnibus buses (clones stored on the manager).
    pub fn set_buses(&mut self, buses: LibraryBuses) {
        self.buses = Some(buses);
    }

    /// Clone of attached buses, if any.
    pub fn buses(&self) -> Option<LibraryBuses> {
        self.buses.clone()
    }

    fn require_buses(&self) -> Result<&LibraryBuses> {
        self.buses.as_ref().ok_or_else(|| LibraryError::Backend {
            backend: "library",
            message: "library buses not attached; call set_buses first".into(),
        })
    }

    pub fn save_history_as_playlist(
        &mut self,
        session_id: &str,
        name: &str,
        sortable: bool,
    ) -> Result<Collection> {
        let entries = self.history.session_entries(&self.db, session_id)?;
        let playlist = self.add_collection(&NewCollection::playlist(name, sortable))?;
        for (position, entry) in entries.iter().enumerate() {
            let Some(track_id) = entry.track_id.as_ref() else {
                continue;
            };
            let track_id = TrackId::new(track_id);
            if self.get_track(&track_id)?.is_none() {
                continue;
            }
            self.add_collection_entry(&playlist.id, &track_id, Some(position as i32))?;
        }
        Ok(playlist)
    }

    /// Fire-and-forget publish of a command (nested `CmdBody` bytes in payload).
    pub fn publish_cmd(&self, origin: Origin, kind: Kind, body: impl AsRef<[u8]>) -> Result<()> {
        self.require_buses()?.publish_cmd(origin, kind, body)
    }

    /// Encode `EvtBody`, bump revision, publish on the evt bus.
    pub fn publish_evt(&self, origin: Origin, kind: Kind, body: EvtBody) -> Result<()> {
        self.require_buses()?.publish_evt(origin, kind, body)
    }

    /// Subscribe to all egress events (host bridge).
    pub fn subscribe_evt_all(&self) -> Result<EvtReceiver> {
        self.require_buses()?.subscribe_evt_all()
    }

    /// Subscribe to egress events for one track (`Origin::Track`).
    pub fn subscribe_evt_track(&self, track_id: impl Into<String>) -> Result<EvtReceiver> {
        self.require_buses()?.subscribe_evt_track(track_id)
    }

    /// Update default analysis duration used by AnalyzeTrack cmds.
    pub fn set_analysis_duration(&self, duration: AnalysisDurationMode) -> Result<()> {
        self.require_buses()?.set_analysis_duration(duration);
        Ok(())
    }

    /// Monotonic revision bumped when discrete library state changes.
    pub fn revision(&self) -> Result<u64> {
        Ok(self.require_buses()?.revision())
    }

    /// Borrow the library configuration.
    pub fn config(&self) -> &LibraryConfig {
        &self.config
    }

    /// Replace the library configuration.
    pub fn set_config(&mut self, config: LibraryConfig) {
        self.config = config;
    }

    /// Enable/disable mirroring app-managed BPM/key into file tags.
    pub fn set_tag_write_back(&mut self, mode: TagWriteBack) {
        self.config.tag_write_back = mode;
    }

    /// Gate that serializes tag writes executed after the manager lock is released.
    pub(crate) fn tag_write_gate(&self) -> Arc<Mutex<()>> {
        Arc::clone(&self.tag_write_gate)
    }

    fn lock_library(m: &Mutex<Self>) -> Result<std::sync::MutexGuard<'_, Self>> {
        m.lock().map_err(|_| LibraryError::Backend {
            backend: "library",
            message: "library lock poisoned".into(),
        })
    }

    pub(crate) fn lock_decode_cache(
        cache: &Mutex<DecodeCache>,
    ) -> Result<std::sync::MutexGuard<'_, DecodeCache>> {
        cache.lock().map_err(|_| LibraryError::Backend {
            backend: "library",
            message: "library decode cache lock poisoned".into(),
        })
    }

    /// Persist analysis results, handing back any tag write-back to run later.
    ///
    /// The plan is *not* executed here: callers hold the library lock, and the
    /// write reads and may rewrite the whole file. Callers that own the lock must
    /// drop it first (see `playback.rs`); the `WritableLibrary::analyze_track`
    /// path cannot, because its caller owns the lock, and executes inline.
    #[cfg(feature = "analysis")]
    fn persist_file_analysis(
        &self,
        path: &Path,
        computed: &ComputedFileAnalysis,
        persist_waveform: bool,
    ) -> Result<(AudioSource, Option<tags::TagWritePlan>)> {
        let path = normalize_path(path)?;
        // A file tag that carries no value for a user-owned field must not
        // clear what the editor stored. ID3 ratings are the common case: lofty
        // cannot read POPM, so a re-analysis would otherwise reset the rating.
        let mut metadata = computed.metadata.clone();
        if let Some((existing, edited)) = self
            .store()
            .file_track_metadata_and_edited(&path.to_string_lossy())?
        {
            tags::merge_user_metadata(&mut metadata, &existing, edited);
        }
        let source = self.upsert_file_source(&path, &metadata)?;
        analysis::upsert_track_analysis(&self.db, source.id(), &computed.analysis, computed.force)?;
        if persist_waveform {
            waveform::generate_and_store_overview(&self.db, source.id(), &path)?;
        }
        let plan = tags::TagWritePlan::new(
            self.config.tag_write_back,
            path,
            tags::app_managed_tags(&metadata),
            self.tag_write_gate(),
        );
        Ok((source, plan))
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

    fn list_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<AudioSource>> {
        let collection = self.require_collection(collection_id)?;
        match collection.collection_type() {
            CollectionType::Folder => {
                let fs_path = collection.fs_path().ok_or_else(|| LibraryError::Backend {
                    backend: "library",
                    message: format!("folder {collection_id} has no fs_path"),
                })?;
                let CollectionConfig::Folder {
                    scan_folder_tree, ..
                } = &collection.config
                else {
                    return Err(LibraryError::Backend {
                        backend: "library",
                        message: format!("folder {collection_id} has non-folder config"),
                    });
                };
                // The query decides membership: a non-recursive folder owns
                // only its direct files, even when nested tracks are indexed.
                self.file_sources_under(fs_path, *scan_folder_tree)
            }
            CollectionType::Playlist => {
                let rows = self.list_collection_entries(collection_id)?;
                let ids: Vec<TrackId> = rows.iter().map(|row| row.track_id.clone()).collect();
                let by_id = self.store().get_tracks_by_ids(&ids)?;
                Ok(rows
                    .into_iter()
                    .filter_map(|row| by_id.get(row.track_id.as_str()).cloned())
                    .collect())
            }
        }
    }

    fn list_collection_entries(
        &self,
        collection_id: &CollectionId,
    ) -> Result<Vec<CollectionEntry>> {
        let _ = self.require_playlist(collection_id)?;
        self.store().list_collection_entries(collection_id)
    }
}

pub(crate) fn normalize_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        path.canonicalize().map_err(LibraryError::from)
    } else {
        Ok(path.to_path_buf())
    }
}

pub(crate) fn is_audio_file(path: &Path) -> bool {
    is_loadable_audio_path(path)
}

pub(crate) fn collect_audio_files(root: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let canonical_root = normalize_path(root)?;
    let mut files = Vec::new();
    if canonical_root.is_file() {
        if is_audio_file(&canonical_root) && path_under_folder(&canonical_root, &canonical_root) {
            files.push(canonical_root);
        }
        return Ok(files);
    }

    let mut stack = vec![canonical_root.clone()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir)?;
        for entry in entries {
            let entry = entry?;
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                if recursive {
                    let normalized = normalize_path(&path)?;
                    if path_under_folder(&normalized, &canonical_root) {
                        stack.push(normalized);
                    }
                }
            } else if file_type.is_file() && is_audio_file(&path) {
                let normalized = normalize_path(&path)?;
                if path_under_folder(&normalized, &canonical_root) {
                    files.push(normalized);
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

pub(crate) fn unix_secs_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests;
