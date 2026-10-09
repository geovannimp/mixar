//! Read-only Mixxx library adapter.
//!
//! [`MixxxLibrary::open`] opens a Mixxx `mixxxdb.sqlite` read-only and loads the
//! track pool, playlists, crates, and watched directories into memory. The type
//! implements [`library_core::Library`] for browsing and
//! [`library_core::Migratable`] to copy the whole library into the user’s
//! canonical manager.

mod schema;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use library_core::{
    camelot_to_musical, AudioSource, Collection, CollectionConfig, CollectionEntry,
    CollectionEntryId, CollectionId, FileAudioSource, Library, LibraryError, Result, TrackId,
    TrackMetadata,
};
use rusqlite::{Connection, OpenFlags};

use schema::{backend, LibrarySchema};

/// A track row loaded from the Mixxx `library` table.
#[derive(Clone, Debug)]
pub(crate) struct MixxxTrack {
    pub library_id: i64,
    pub path: PathBuf,
    pub metadata: TrackMetadata,
}

#[derive(Clone, Debug)]
struct MixxxList {
    members: Vec<i64>,
    sortable: bool,
}

/// Counts used to confirm an import before running it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MixxxPreview {
    /// Non-deleted tracks in the Mixxx library.
    pub track_count: usize,
    /// Tracks whose file is currently absent on disk.
    pub missing_file_count: usize,
    /// Visible user playlists.
    pub playlist_count: usize,
    /// Crates.
    pub crate_count: usize,
    /// Watched root directories.
    pub folder_count: usize,
}

/// A read-only view over a Mixxx `mixxxdb.sqlite`.
pub struct MixxxLibrary {
    tracks: Vec<MixxxTrack>,
    track_index: HashMap<i64, usize>,
    collections: Vec<Collection>,
    lists: HashMap<String, MixxxList>,
    folders: Vec<(CollectionId, PathBuf)>,
}

impl MixxxLibrary {
    /// Open a Mixxx database read-only and load its contents.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| backend(format!("open {}: {e}", path.display())))?;
        conn.execute_batch("PRAGMA query_only = 1;")
            .map_err(|e| backend(format!("set query_only: {e}")))?;

        let schema = schema::detect(&conn)?;

        let tracks = load_tracks(&conn, &schema)?;
        let mut track_index = HashMap::with_capacity(tracks.len());
        for (index, track) in tracks.iter().enumerate() {
            track_index.insert(track.library_id, index);
        }

        let folders = load_folders(&conn)?;
        let mut collections: Vec<Collection> = folders
            .iter()
            .map(|(id, path)| Collection {
                id: id.clone(),
                name: folder_name(path),
                config: CollectionConfig::Folder {
                    fs_path: path.clone(),
                    scan_folder_tree: true,
                },
            })
            .collect();

        let mut lists: HashMap<String, MixxxList> = HashMap::new();
        for (collection, members, sortable) in load_playlists(&conn)?
            .into_iter()
            .chain(load_crates(&conn)?)
        {
            lists.insert(
                collection.id.as_str().to_string(),
                MixxxList { members, sortable },
            );
            collections.push(collection);
        }

        collections.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });

        Ok(Self {
            tracks,
            track_index,
            collections,
            lists,
            folders,
        })
    }

    /// Summary counts for an import confirmation.
    pub fn preview(&self) -> MixxxPreview {
        MixxxPreview {
            track_count: self.tracks.len(),
            missing_file_count: self.missing_file_count(),
            playlist_count: self.playlist_count(),
            crate_count: self.crate_count(),
            folder_count: self.folders.len(),
        }
    }

    /// Non-deleted tracks loaded from the Mixxx library.
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Loaded tracks whose file is currently absent on disk.
    pub fn missing_file_count(&self) -> usize {
        self.tracks
            .iter()
            .filter(|track| !track.path.is_file())
            .count()
    }

    /// Visible user playlists.
    pub fn playlist_count(&self) -> usize {
        self.lists.values().filter(|list| list.sortable).count()
    }

    /// Crates.
    pub fn crate_count(&self) -> usize {
        self.lists.values().filter(|list| !list.sortable).count()
    }

    /// Watched root directories.
    pub fn folder_count(&self) -> usize {
        self.folders.len()
    }

    fn source(&self, track: &MixxxTrack) -> AudioSource {
        AudioSource::File(FileAudioSource::new(
            TrackId::new(format!("mixxx:track:{}", track.library_id)),
            track.path.clone(),
            track.metadata.clone(),
        ))
    }
}

impl Library for MixxxLibrary {
    fn name(&self) -> &'static str {
        "mixxx"
    }

    fn get_track(&self, id: &TrackId) -> Result<Option<AudioSource>> {
        let Some(library_id) = parse_track_id(id) else {
            return Ok(None);
        };
        Ok(self
            .track_index
            .get(&library_id)
            .map(|index| self.source(&self.tracks[*index])))
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        Ok(self.collections.clone())
    }

    fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>> {
        Ok(self
            .collections
            .iter()
            .find(|collection| collection.id == *id)
            .cloned())
    }

    fn list_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<AudioSource>> {
        if let Some((_, folder)) = self.folders.iter().find(|(id, _)| id == collection_id) {
            return Ok(self
                .tracks
                .iter()
                .filter(|track| track.path.starts_with(folder))
                .map(|track| self.source(track))
                .collect());
        }

        let Some(list) = self.lists.get(collection_id.as_str()) else {
            return Err(LibraryError::NotFound(collection_id.to_string()));
        };
        Ok(list
            .members
            .iter()
            .filter_map(|library_id| self.track_index.get(library_id))
            .map(|index| self.source(&self.tracks[*index]))
            .collect())
    }

    fn list_collection_entries(
        &self,
        collection_id: &CollectionId,
    ) -> Result<Vec<CollectionEntry>> {
        let Some(list) = self.lists.get(collection_id.as_str()) else {
            if self.folders.iter().any(|(id, _)| id == collection_id) {
                return Err(LibraryError::WrongCollectionType {
                    expected: "playlist",
                    got: "folder",
                });
            }
            return Err(LibraryError::NotFound(collection_id.to_string()));
        };
        Ok(list
            .members
            .iter()
            .enumerate()
            .map(|(index, library_id)| CollectionEntry {
                id: CollectionEntryId::new(format!("{}:entry:{index}", collection_id)),
                collection_id: collection_id.clone(),
                track_id: TrackId::new(format!("mixxx:track:{library_id}")),
                position: list.sortable.then_some(index as i32),
            })
            .collect())
    }
}

/// Default Mixxx database location for the current OS, when it exists.
pub fn default_database_path() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Mixxx"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join("Library/Application Support/Mixxx"))
    } else {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".mixxx"))
    }?;
    let database = base.join("mixxxdb.sqlite");
    database.is_file().then_some(database)
}

fn parse_track_id(id: &TrackId) -> Option<i64> {
    id.as_str().strip_prefix("mixxx:track:")?.parse().ok()
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn load_tracks(conn: &Connection, schema: &LibrarySchema) -> Result<Vec<MixxxTrack>> {
    let sql = format!(
        "SELECT {} FROM library l {} {}",
        schema.metadata_select(),
        schema.track_join(),
        schema.deleted_filter()
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| backend(format!("prepare track query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            let library_id: i64 = row.get("id")?;
            let path: Option<String> = row.get("path")?;
            let metadata = metadata_from_row(row)?;
            Ok((library_id, path, metadata))
        })
        .map_err(|e| backend(format!("query tracks: {e}")))?;

    let mut tracks = Vec::new();
    for row in rows {
        let (library_id, path, metadata) = row.map_err(|e| backend(format!("read track: {e}")))?;
        let Some(path) = path.filter(|path| !path.trim().is_empty()) else {
            continue;
        };
        tracks.push(MixxxTrack {
            library_id,
            path: PathBuf::from(path),
            metadata,
        });
    }
    Ok(tracks)
}

fn metadata_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrackMetadata> {
    let bpm: Option<f64> = row.get("bpm")?;
    let key: Option<String> = row.get("key")?;
    let duration: Option<f64> = row.get("duration")?;
    let sample_rate: Option<i64> = row.get("samplerate")?;
    let channels: Option<i64> = row.get("channels")?;
    let bitrate: Option<i64> = row.get("bitrate")?;
    let replaygain: Option<f64> = row.get("replaygain")?;

    Ok(TrackMetadata {
        title: clean(row.get("title")?),
        artist: clean(row.get("artist")?),
        album: clean(row.get("album")?),
        genre: clean(row.get("genre")?),
        bpm: bpm.filter(|value| *value > 0.0),
        key: clean(key).map(|key| camelot_to_musical(&key).unwrap_or(key)),
        duration_ms: duration
            .filter(|value| *value > 0.0)
            .map(|value| (value * 1000.0).round() as i32),
        sample_rate: sample_rate
            .filter(|value| *value > 0)
            .map(|value| value as u32),
        channels: channels
            .filter(|value| *value > 0)
            .map(|value| value as u16),
        bitrate_kbps: bitrate.filter(|value| *value > 0).map(|value| value as u32),
        replaygain_track_gain_db: replaygain
            .filter(|value| *value > 0.0)
            .map(|value| 20.0 * value.log10()),
        ..TrackMetadata::default()
    })
}

fn load_folders(conn: &Connection) -> Result<Vec<(CollectionId, PathBuf)>> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='directories'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| backend(format!("inspect directories: {e}")))?;
    if exists == 0 {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare("SELECT directory FROM directories")
        .map_err(|e| backend(format!("prepare directories query: {e}")))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, Option<String>>(0))
        .map_err(|e| backend(format!("query directories: {e}")))?;

    let mut folders = Vec::new();
    for row in rows {
        let directory = row.map_err(|e| backend(format!("read directory: {e}")))?;
        let Some(directory) = directory.filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        let path = PathBuf::from(directory);
        folders.push((
            CollectionId::new(format!("mixxx:folder:{}", path.display())),
            path,
        ));
    }
    Ok(folders)
}

fn load_playlists(conn: &Connection) -> Result<Vec<(Collection, Vec<i64>, bool)>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name FROM Playlists WHERE COALESCE(hidden, 0) = 0 ORDER BY position, id",
        )
        .map_err(|e| backend(format!("prepare playlists query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| backend(format!("query playlists: {e}")))?;

    let mut lists = Vec::new();
    for row in rows {
        let (id, name) = row.map_err(|e| backend(format!("read playlist: {e}")))?;
        let collection = Collection {
            id: CollectionId::new(format!("mixxx:playlist:{id}")),
            name: name.unwrap_or_default(),
            config: CollectionConfig::Playlist { sortable: true },
        };
        lists.push((collection, playlist_members(conn, id)?, true));
    }
    Ok(lists)
}

fn playlist_members(conn: &Connection, playlist_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn
        .prepare("SELECT track_id FROM PlaylistTracks WHERE playlist_id = ?1 ORDER BY position, id")
        .map_err(|e| backend(format!("prepare playlist tracks query: {e}")))?;
    let rows = stmt
        .query_map([playlist_id], |row| row.get::<_, i64>(0))
        .map_err(|e| backend(format!("query playlist tracks: {e}")))?;
    let mut members = Vec::new();
    for row in rows {
        members.push(row.map_err(|e| backend(format!("read playlist track: {e}")))?);
    }
    Ok(members)
}

fn load_crates(conn: &Connection) -> Result<Vec<(Collection, Vec<i64>, bool)>> {
    let mut stmt = conn
        .prepare("SELECT id, name FROM crates ORDER BY name, id")
        .map_err(|e| backend(format!("prepare crates query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| backend(format!("query crates: {e}")))?;

    let mut lists = Vec::new();
    for row in rows {
        let (id, name) = row.map_err(|e| backend(format!("read crate: {e}")))?;
        let collection = Collection {
            id: CollectionId::new(format!("mixxx:crate:{id}")),
            name: name.unwrap_or_default(),
            config: CollectionConfig::Playlist { sortable: false },
        };
        lists.push((collection, crate_members(conn, id)?, false));
    }
    Ok(lists)
}

fn crate_members(conn: &Connection, crate_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn
        .prepare("SELECT track_id FROM crate_tracks WHERE crate_id = ?1 ORDER BY rowid")
        .map_err(|e| backend(format!("prepare crate tracks query: {e}")))?;
    let rows = stmt
        .query_map([crate_id], |row| row.get::<_, i64>(0))
        .map_err(|e| backend(format!("query crate tracks: {e}")))?;
    let mut members = Vec::new();
    for row in rows {
        members.push(row.map_err(|e| backend(format!("read crate track: {e}")))?);
    }
    Ok(members)
}
