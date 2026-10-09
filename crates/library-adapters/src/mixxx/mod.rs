//! Read-only Mixxx library adapter.
//!
//! [`MixxxLibrary::open`] opens a Mixxx `mixxxdb.sqlite` read-only and loads the
//! track pool, playlists, crates, and watched directories into memory. The type
//! implements [`library_core::Library`] for browsing and
//! [`library_core::Migratable`] to copy the whole library into the user’s
//! canonical manager.

mod schema;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use library_core::{
    camelot_to_musical, path_under_folder, AudioSource, Collection, CollectionConfig,
    CollectionEntry, CollectionEntryId, CollectionId, CollectionType, FileAudioSource, Library,
    LibraryError, Migratable, MigrateOptions, MigrateReport, NewCollection, Result, TrackId,
    TrackMetadata, WritableLibrary,
};
use rusqlite::{Connection, OpenFlags};

use schema::{backend, LibrarySchema};

/// Name of the catch-all collection that holds every imported Mixxx track.
const CATCH_ALL_NAME: &str = "Mixxx";

/// A track row loaded from the Mixxx `library` table.
#[derive(Clone, Debug)]
pub(crate) struct MixxxTrack {
    pub library_id: i64,
    pub path: PathBuf,
    pub metadata: TrackMetadata,
    /// File was absent on disk when the library was opened. Computed here so the
    /// migration's write phase never touches the filesystem.
    pub missing: bool,
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
        self.tracks.iter().filter(|track| track.missing).count()
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
                .filter(|track| path_under_folder(&track.path, folder))
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

impl Migratable for MixxxLibrary {
    fn migrate(
        &self,
        target: &mut dyn WritableLibrary,
        options: &MigrateOptions,
    ) -> Result<MigrateReport> {
        let mut report = MigrateReport::default();

        // Snapshot the target once, before any mutation: folder ids detect
        // already-registered watched directories, and list names make re-imports
        // idempotent.
        let mut existing_folder_ids: HashSet<String> = HashSet::new();
        let mut existing_lists: HashSet<(String, bool)> = HashSet::new();
        let existing_collections = target.list_collections()?;
        for collection in &existing_collections {
            match collection.collection_type() {
                CollectionType::Folder => {
                    existing_folder_ids.insert(collection.id.as_str().to_string());
                }
                CollectionType::Playlist => {
                    existing_lists
                        .insert((collection.name.trim().to_string(), collection.sortable()));
                }
            }
        }

        // Import every loaded track once, remembering the target id per Mixxx id.
        let mut target_ids: HashMap<i64, TrackId> = HashMap::with_capacity(self.tracks.len());
        for track in &self.tracks {
            match target.import_track(&track.path, &track.metadata) {
                Ok(imported) => {
                    if imported.created {
                        report.tracks_added += 1;
                    } else {
                        report.tracks_updated += 1;
                    }
                    if track.missing {
                        report.tracks_missing_files += 1;
                    }
                    target_ids.insert(track.library_id, imported.source.id().clone());
                }
                Err(err) => {
                    report.failed += 1;
                    report
                        .errors
                        .push(format!("{}: {err}", track.path.display()));
                }
            }
        }

        if options.include_folders {
            for (_, path) in &self.folders {
                if !path.is_dir() {
                    report.failed += 1;
                    report
                        .errors
                        .push(format!("watched directory missing: {}", path.display()));
                    continue;
                }
                match target.add_collection(&NewCollection::folder(path)) {
                    Ok(created) => {
                        if existing_folder_ids.insert(created.id.as_str().to_string()) {
                            report.folders_imported += 1;
                        } else {
                            report.collections_skipped += 1;
                        }
                    }
                    Err(err) => {
                        report.failed += 1;
                        report.errors.push(format!("{}: {err}", path.display()));
                    }
                }
            }
        }

        // Tracks are only reachable through a collection, so every imported
        // track also goes into a catch-all "Mixxx" playlist (a set, so re-runs
        // merge instead of duplicating). This is created even when playlists,
        // crates, or folders are excluded, so nothing is lost.
        if !target_ids.is_empty() {
            let catch_all_id: Option<CollectionId> =
                match existing_collections.iter().find(|collection| {
                    collection.collection_type() == CollectionType::Playlist
                        && !collection.sortable()
                        && collection.name.trim() == CATCH_ALL_NAME
                }) {
                    Some(existing) => Some(existing.id.clone()),
                    None => match target
                        .add_collection(&NewCollection::playlist(CATCH_ALL_NAME, false))
                    {
                        Ok(created) => {
                            existing_lists.insert((CATCH_ALL_NAME.to_string(), false));
                            Some(created.id)
                        }
                        Err(err) => {
                            report.failed += 1;
                            report.errors.push(format!("{CATCH_ALL_NAME}: {err}"));
                            None
                        }
                    },
                };
            if let Some(catch_all_id) = catch_all_id {
                for track_id in target_ids.values() {
                    if let Err(err) = target.add_collection_entry(&catch_all_id, track_id, None) {
                        report.failed += 1;
                        report.errors.push(format!("{CATCH_ALL_NAME}: {err}"));
                    }
                }
            }
        }

        let mut lists: Vec<(&Collection, &Vec<i64>, bool)> = self
            .lists
            .iter()
            .filter_map(|(id, list)| {
                self.collections
                    .iter()
                    .find(|collection| collection.id.as_str() == id)
                    .map(|collection| (collection, &list.members, list.sortable))
            })
            .collect();
        lists.sort_by_key(|a| a.0.name.to_lowercase());

        for (collection, members, sortable) in lists {
            let include = if sortable {
                options.include_playlists
            } else {
                options.include_crates
            };
            if !include {
                continue;
            }

            let key = (collection.name.trim().to_string(), sortable);
            if options.skip_existing_lists && existing_lists.contains(&key) {
                report.collections_skipped += 1;
                continue;
            }

            let created = match target
                .add_collection(&NewCollection::playlist(collection.name.trim(), sortable))
            {
                Ok(created) => {
                    // Only remember the list once it exists, so a failed
                    // add_collection does not suppress a retry later in the run.
                    existing_lists.insert(key);
                    created
                }
                Err(err) => {
                    report.failed += 1;
                    report.errors.push(format!("{}: {err}", collection.name));
                    continue;
                }
            };

            for (position, library_id) in members.iter().enumerate() {
                let Some(track_id) = target_ids.get(library_id) else {
                    continue;
                };
                let position = sortable.then_some(position as i32);
                if let Err(err) = target.add_collection_entry(&created.id, track_id, position) {
                    report.failed += 1;
                    report.errors.push(format!("{}: {err}", collection.name));
                }
            }

            if sortable {
                report.playlists_imported += 1;
            } else {
                report.crates_imported += 1;
            }
        }

        Ok(report)
    }
}

/// Default Mixxx database location for the current OS, when it exists.
///
/// Mixxx stores `mixxxdb.sqlite` in a settings directory that varies by
/// install: native (`~/.mixxx`), Flatpak (`~/.var/app/org.mixxx.Mixxx/.mixxx`),
/// Snap (`~/snap/mixxx/...`), XDG (`$XDG_DATA_HOME/mixxx`), and the macOS /
/// Windows equivalents. The first existing candidate wins. `MIXXX_DATABASE`
/// (a file) and `MIXXX_SETTINGS_PATH` / `MIXXX_SETTINGS_DIR` (a directory)
/// override the search.
pub fn default_database_path() -> Option<PathBuf> {
    candidate_database_paths()
        .into_iter()
        .find(|path| path.is_file())
}

fn candidate_database_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit) = nonempty_env("MIXXX_DATABASE") {
        candidates.push(explicit);
    }
    for key in ["MIXXX_SETTINGS_PATH", "MIXXX_SETTINGS_DIR"] {
        if let Some(dir) = nonempty_env(key) {
            candidates.push(dir.join("mixxxdb.sqlite"));
        }
    }

    let home = nonempty_env("HOME");
    let xdg_data_home = nonempty_env("XDG_DATA_HOME");
    let local_app_data = nonempty_env("LOCALAPPDATA");
    for dir in settings_dirs(
        home.as_deref(),
        xdg_data_home.as_deref(),
        local_app_data.as_deref(),
    ) {
        candidates.push(dir.join("mixxxdb.sqlite"));
    }
    candidates
}

fn settings_dirs(
    home: Option<&Path>,
    xdg_data_home: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(target_os = "windows") {
        if let Some(local) = local_app_data {
            dirs.push(local.join("Mixxx"));
        }
    } else if cfg!(target_os = "macos") {
        if let Some(home) = home {
            dirs.push(home.join("Library/Application Support/Mixxx"));
        }
    } else {
        if let Some(home) = home {
            // Native install, then Flatpak and Snap sandboxes.
            dirs.push(home.join(".mixxx"));
            dirs.push(home.join(".var/app/org.mixxx.Mixxx/.mixxx"));
            dirs.push(home.join("snap/mixxx/current/.mixxx"));
            dirs.push(home.join("snap/mixxx/common/.mixxx"));
            dirs.push(home.join(".local/share/mixxx"));
        }
        if let Some(data) = xdg_data_home {
            dirs.push(data.join("mixxx"));
        }
    }
    dirs
}

fn nonempty_env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
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
        let path = PathBuf::from(path);
        let missing = !path.is_file();
        tracks.push(MixxxTrack {
            library_id,
            path,
            metadata,
            missing,
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
    if !schema::table_exists(conn, "directories")? {
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
    if !schema::table_exists(conn, "Playlists")? {
        return Ok(Vec::new());
    }

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
    if !schema::table_exists(conn, "crates")? {
        return Ok(Vec::new());
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn settings_dirs_cover_native_flatpak_and_snap() {
        let home = Path::new("/home/user");
        let dirs = settings_dirs(Some(home), Some(Path::new("/home/user/.local/share")), None);
        assert!(dirs.contains(&home.join(".mixxx")));
        assert!(dirs.contains(&home.join(".var/app/org.mixxx.Mixxx/.mixxx")));
        assert!(dirs.contains(&home.join("snap/mixxx/current/.mixxx")));
        assert!(dirs.contains(&PathBuf::from("/home/user/.local/share/mixxx")));
    }
}
