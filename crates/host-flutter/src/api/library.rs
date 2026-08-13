//! FRB `LibraryTransport`: browse collections + tracks (shared Tauri `library.db`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use library::{
    read_artwork, spawn_library_worker, LibraryBuses, LibraryConfig, LibraryManager, LibraryWorker,
    NewCollection, TrackId, WritableLibrary,
};
use library_core::{AudioSource, Collection, CollectionId, Library};

/// Collection row for the Flutter collections pane (mirrors Tauri `CollectionSummary`).
#[derive(Clone, Debug)]
pub struct LibraryCollectionSummary {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub path: Option<String>,
    pub track_count: u32,
}

/// Track row for the Flutter track table (mirrors Tauri / `library_api::TrackSummary`).
#[derive(Clone, Debug)]
pub struct LibraryTrackSummary {
    pub id: String,
    pub display_name: String,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub duration_ms: Option<i32>,
    pub path: String,
}

/// Result of adding a folder collection and syncing it.
#[derive(Clone, Debug)]
pub struct AddFolderCollectionResult {
    pub collection: LibraryCollectionSummary,
    pub added: u32,
    pub updated: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// Path lookup hit: original request path + resolved track summary.
#[derive(Clone, Debug)]
pub struct ResolvedLibraryTrack {
    pub request_path: String,
    pub track: LibraryTrackSummary,
}

/// Host-owned library handle exposed to Dart via FRB methods.
#[flutter_rust_bridge::frb(opaque)]
pub struct LibraryTransport {
    // ponytail: Mutex serializes browse calls per transport. Upgrade to a
    // read-capable manager / connection pool if concurrent queries matter.
    library: Arc<Mutex<LibraryManager>>,
    /// Kept for Task 3 (analyze/refresh stream); unused by browse RPCs.
    #[allow(dead_code)]
    buses: LibraryBuses,
    /// Drop joins the worker; must not live inside the manager mutex.
    #[allow(dead_code)]
    worker: LibraryWorker,
}

impl LibraryTransport {
    /// Open (or create) a SQLite library at `db_path`.
    pub fn open(db_path: String) -> Result<Self, String> {
        let manager =
            LibraryManager::open(db_path, LibraryConfig::default()).map_err(|e| e.to_string())?;
        Self::from_manager(manager)
    }

    /// In-memory library for tests.
    pub fn open_in_memory() -> Result<Self, String> {
        let manager =
            LibraryManager::open_in_memory(LibraryConfig::default()).map_err(|e| e.to_string())?;
        Self::from_manager(manager)
    }

    fn from_manager(mut manager: LibraryManager) -> Result<Self, String> {
        let buses = LibraryBuses::new();
        manager.set_buses(buses.clone());
        let library = Arc::new(Mutex::new(manager));
        let worker = spawn_library_worker(Arc::clone(&library)).map_err(|e| e.to_string())?;
        Ok(Self {
            library,
            buses,
            worker,
        })
    }

    /// List all collections with track counts.
    pub fn list_collections(&self) -> Result<Vec<LibraryCollectionSummary>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let collections = lib.list_collections().map_err(|e| e.to_string())?;
        collections
            .into_iter()
            .map(|c| collection_summary(&lib, c))
            .collect()
    }

    /// List tracks in a collection.
    pub fn list_collection_tracks(
        &self,
        collection_id: String,
    ) -> Result<Vec<LibraryTrackSummary>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let tracks = lib
            .get_collection_tracks(&CollectionId::new(collection_id))
            .map_err(|e| e.to_string())?;
        Ok(tracks.iter().filter_map(track_summary).collect())
    }

    /// Add a folder as a collection and sync its tracks.
    pub fn add_folder_collection(
        &self,
        folder_path: String,
    ) -> Result<AddFolderCollectionResult, String> {
        let mut lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let collection = lib
            .add_collection(&NewCollection::folder(folder_path))
            .map_err(|e| e.to_string())?;
        let scan = lib
            .sync_collection(Some(&collection.id))
            .map_err(|e| e.to_string())?;
        let summary = collection_summary(&lib, collection)?;
        Ok(AddFolderCollectionResult {
            collection: summary,
            added: scan.added as u32,
            updated: scan.updated as u32,
            skipped: scan.skipped as u32,
            failed: scan.failed as u32,
        })
    }

    /// Resolve library tracks for the given filesystem paths.
    pub fn resolve_tracks_for_paths(
        &self,
        paths: Vec<String>,
    ) -> Result<Vec<ResolvedLibraryTrack>, String> {
        let lib = self
            .library
            .lock()
            .map_err(|_| "library lock poisoned".to_string())?;
        let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        let resolved = lib
            .lookup_file_tracks_at_paths(&path_bufs)
            .map_err(|e| e.to_string())?;
        Ok(resolved
            .into_iter()
            .filter_map(|(request_path, source)| {
                track_summary(&source).map(|track| ResolvedLibraryTrack {
                    request_path,
                    track,
                })
            })
            .collect())
    }

    /// Embedded artwork bytes for a track id and/or file path.
    ///
    /// Returns `Ok(None)` when the file has no artwork (e.g. minimal WAV).
    pub fn get_track_artwork(
        &self,
        track_id: Option<String>,
        path: Option<String>,
    ) -> Result<Option<Vec<u8>>, String> {
        let file_path = if let Some(path) = path {
            path
        } else if let Some(track_id) = track_id {
            let lib = self
                .library
                .lock()
                .map_err(|_| "library lock poisoned".to_string())?;
            let source = lib
                .get_track(&TrackId::new(track_id))
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Track not found in library.".to_string())?;
            source
                .file()
                .ok_or_else(|| "Only file tracks have artwork.".to_string())?
                .path()
                .to_string_lossy()
                .into_owned()
        } else {
            return Err("track_id or path is required.".to_string());
        };

        read_artwork(Path::new(&file_path)).map_err(|e| e.to_string())
    }
}

fn collection_summary(
    library: &LibraryManager,
    collection: Collection,
) -> Result<LibraryCollectionSummary, String> {
    let track_count = library
        .get_collection_tracks(&collection.id)
        .map_err(|e| e.to_string())?
        .len() as u32;
    Ok(LibraryCollectionSummary {
        id: collection.id.as_str().to_string(),
        name: collection.name.clone(),
        kind: collection.collection_type().as_str().to_string(),
        path: collection
            .fs_path()
            .map(|path| path.to_string_lossy().into_owned()),
        track_count,
    })
}

fn track_summary(source: &AudioSource) -> Option<LibraryTrackSummary> {
    let file = source.file()?;
    let metadata = source.metadata();
    Some(LibraryTrackSummary {
        id: source.id().as_str().to_string(),
        display_name: track_display_name(source),
        artist: metadata.artist.clone(),
        title: metadata.title.clone(),
        album: metadata.album.clone(),
        genre: metadata.genre.clone(),
        bpm: metadata.bpm,
        key: metadata.key.clone(),
        duration_ms: metadata.duration_ms,
        path: file.path().to_string_lossy().into_owned(),
    })
}

fn track_display_name(source: &AudioSource) -> String {
    let metadata = source.metadata();
    if let Some(title) = metadata.title.as_ref().filter(|t| !t.is_empty()) {
        return title.clone();
    }
    source
        .file()
        .and_then(|f| {
            f.path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| source.id().as_str().to_string())
}
