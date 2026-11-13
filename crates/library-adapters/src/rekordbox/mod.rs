//! Read-only Rekordbox 6/7 library adapter.
//!
//! [`RekordboxLibrary::open`] copies the SQLCipher-encrypted `master.db` (and
//! its `-wal` when present) into a private temporary directory, unlocks the
//! copy, and loads tracks, playlists, hot cues and saved loops into memory. The
//! original database is never opened for writing.
//!
//! The type implements [`library_core::Library`] for browsing and
//! [`library_core::Migratable`] to copy the whole library into the user's
//! canonical manager through [`library_core::WritableLibrary`].
//!
//! # What is dropped
//!
//! Documented lossiness, mirroring the Mixxx adapter's approach: only the
//! earliest memory cue survives (it becomes the track cue; a memory cue with an
//! out point is additionally stored as a saved loop), playlist-folder nesting
//! (folders become a name prefix, see [`list_name`]), beat grids and waveforms
//! (they live in ANLZ analysis files, not the database), and My Tag / history.

mod crypto;
mod schema;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use library_core::{
    camelot_to_musical, path_label, AudioSource, Collection, CollectionConfig, CollectionEntry,
    CollectionEntryId, CollectionId, ColorLabel, FileAudioSource, HotCueImport, Library,
    LibraryError, LoopImport, Migratable, MigrateOptions, MigrateReport, NewCollection, Result,
    TrackId, TrackMetadata, WritableLibrary,
};
use rusqlite::Connection;

use crypto::DatabaseCopy;
use schema::{backend, Schema};

pub use crypto::{DATABASE_KEY, KEY_ENV};

/// Name of the catch-all playlist holding every imported track.
const CATCH_ALL_NAME: &str = "Rekordbox";

/// `djmdCue.Kind` of a memory cue; a larger value is a hot cue whose
/// zero-based slot is `Kind - 1` (cue A is `Kind = 1`).
const MEMORY_CUE_KIND: i64 = 0;

/// `djmdPlaylist.Attribute`: 0 playlist, 1 folder, 4 smart playlist.
const ATTRIBUTE_FOLDER: i64 = 1;
const ATTRIBUTE_SMART: i64 = 4;

/// Separator between folder names when a nested playlist is flattened.
const FOLDER_SEPARATOR: &str = " / ";

/// Rekordbox's hot-cue colour palette, indexed by `djmdCue.Color - 1`.
///
/// The values are the ones Rekordbox writes into XML exports and ANLZ cue
/// entries (rose, red, orange, lemon, green, turquoise, blue, violet).
const CUE_COLORS: [&str; 8] = [
    "#FF007F", "#FF0000", "#FFA500", "#FFFF00", "#00FF00", "#25FDE9", "#0000FF", "#660099",
];

/// A track row loaded from `djmdContent`.
#[derive(Clone, Debug)]
struct RekordboxTrack {
    content_id: String,
    path: PathBuf,
    metadata: TrackMetadata,
    /// The file was absent on disk when the library was opened. Computed here
    /// so the migration's write phase never touches the filesystem.
    missing: bool,
}

/// One row of `djmdPlaylist`.
#[derive(Clone, Debug)]
struct PlaylistRow {
    id: String,
    name: String,
    attribute: i64,
    parent: Option<String>,
}

impl PlaylistRow {
    fn is_folder(&self) -> bool {
        self.attribute == ATTRIBUTE_FOLDER
    }

    fn is_smart(&self) -> bool {
        self.attribute == ATTRIBUTE_SMART
    }
}

/// Hot cues and saved loops of one track, in slot order.
#[derive(Clone, Debug, Default)]
pub struct CueSet {
    /// Hot cues, ordered by slot.
    pub hot_cues: Vec<HotCueImport>,
    /// Saved loops, ordered by slot.
    pub loops: Vec<LoopImport>,
    /// Position (ms) of the earliest memory cue: the track's main cue.
    pub main_cue_ms: Option<i32>,
}

/// Counts used to confirm an import before running it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RekordboxPreview {
    /// Rows in `djmdContent`.
    pub track_count: usize,
    /// Tracks whose file is currently absent on disk.
    pub missing_file_count: usize,
    /// User playlists (`Attribute = 0`).
    pub playlist_count: usize,
    /// Smart playlists, imported as static snapshots (`Attribute = 4`).
    pub smart_playlist_count: usize,
    /// Playlist folders (`Attribute = 1`); they only contribute name prefixes.
    pub folder_count: usize,
    /// Hot cues found in `djmdCue`.
    pub hot_cue_count: usize,
    /// Saved loops found in `djmdCue`.
    pub loop_count: usize,
}

/// A read-only view over a Rekordbox 6/7 `master.db`.
#[derive(Debug)]
pub struct RekordboxLibrary {
    tracks: Vec<RekordboxTrack>,
    track_index: HashMap<String, usize>,
    cues: HashMap<String, CueSet>,
    collections: Vec<Collection>,
    collection_index: HashMap<String, usize>,
    /// Collection id → member content ids, in list order.
    members: HashMap<String, Vec<String>>,
    playlists: Vec<PlaylistRow>,
}

impl RekordboxLibrary {
    /// Open the Rekordbox library at `path` read-only.
    ///
    /// The passphrase comes from [`KEY_ENV`] when set, else from the
    /// community-verified constant in [`crypto`].
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_passphrase(path.as_ref(), &crypto::default_passphrase())
    }

    /// Open with an explicit SQLCipher passphrase (rotated keys, tests).
    pub fn open_keyed(path: impl AsRef<Path>, passphrase: &str) -> Result<Self> {
        Self::open_with_passphrase(path.as_ref(), passphrase)
    }

    fn open_with_passphrase(path: &Path, passphrase: &str) -> Result<Self> {
        if is_rekordbox5_file(path) {
            return Err(schema::unsupported(
                "Rekordbox 5 datafile.edb is not supported; import needs a Rekordbox 6/7 master.db",
            ));
        }

        let copy = DatabaseCopy::create(path)?;
        let conn = crypto::open_library(copy.path(), &path_label(path), passphrase)?;

        let schema = schema::detect(&conn)?;
        let lookups = load_lookups(&conn, &schema)?;
        let tracks = load_tracks(&conn, &lookups)?;
        let cues = load_cues(&conn, &schema, &tracks)?;
        let playlists = load_playlists(&conn)?;
        let members = load_memberships(&conn)?;

        let mut track_index = HashMap::with_capacity(tracks.len());
        for (index, track) in tracks.iter().enumerate() {
            track_index.insert(track.content_id.clone(), index);
        }

        let playlist_by_id: HashMap<&str, &PlaylistRow> =
            playlists.iter().map(|row| (row.id.as_str(), row)).collect();
        let mut collections = Vec::with_capacity(playlists.len());
        let mut list_members = HashMap::with_capacity(playlists.len());
        for playlist in playlists.iter().filter(|row| !row.is_folder()) {
            let id = CollectionId::new(format!("rekordbox:playlist:{}", playlist.id));
            collections.push(Collection {
                id: id.clone(),
                name: list_name(playlist, &playlist_by_id),
                // Rekordbox keeps an explicit order for every list, and a smart
                // playlist snapshot keeps the order it was computed in.
                config: CollectionConfig::Playlist { sortable: true },
            });
            let member_ids = members
                .get(&playlist.id)
                .map(|ids| {
                    ids.iter()
                        .filter(|id| track_index.contains_key(id.as_str()))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            list_members.insert(id.as_str().to_string(), member_ids);
        }

        collections.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        let collection_index = collections
            .iter()
            .enumerate()
            .map(|(index, collection)| (collection.id.as_str().to_string(), index))
            .collect();

        Ok(Self {
            tracks,
            track_index,
            cues,
            collections,
            collection_index,
            members: list_members,
            playlists,
        })
    }

    /// Summary counts for an import confirmation.
    pub fn preview(&self) -> RekordboxPreview {
        RekordboxPreview {
            track_count: self.tracks.len(),
            missing_file_count: self.missing_file_count(),
            playlist_count: self
                .playlists
                .iter()
                .filter(|row| !row.is_folder() && !row.is_smart())
                .count(),
            smart_playlist_count: self.playlists.iter().filter(|row| row.is_smart()).count(),
            folder_count: self.playlists.iter().filter(|row| row.is_folder()).count(),
            hot_cue_count: self.cues.values().map(|set| set.hot_cues.len()).sum(),
            loop_count: self.cues.values().map(|set| set.loops.len()).sum(),
        }
    }

    /// Tracks loaded from the Rekordbox collection.
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Loaded tracks whose file is currently absent on disk.
    pub fn missing_file_count(&self) -> usize {
        self.tracks.iter().filter(|track| track.missing).count()
    }

    /// Hot cues and saved loops of one track (empty when it has none).
    pub fn cues_for(&self, id: &TrackId) -> CueSet {
        parse_track_id(id)
            .and_then(|content_id| self.cues.get(&content_id).cloned())
            .unwrap_or_default()
    }

    fn source(&self, track: &RekordboxTrack) -> AudioSource {
        AudioSource::File(FileAudioSource::new(
            TrackId::new(format!("rekordbox:track:{}", track.content_id)),
            track.path.clone(),
            track.metadata.clone(),
        ))
    }
}

impl Library for RekordboxLibrary {
    fn name(&self) -> &'static str {
        "rekordbox"
    }

    fn get_track(&self, id: &TrackId) -> Result<Option<AudioSource>> {
        let Some(content_id) = parse_track_id(id) else {
            return Ok(None);
        };
        Ok(self
            .track_index
            .get(&content_id)
            .map(|index| self.source(&self.tracks[*index])))
    }

    fn list_collections(&self) -> Result<Vec<Collection>> {
        Ok(self.collections.clone())
    }

    fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>> {
        Ok(self
            .collection_index
            .get(id.as_str())
            .map(|index| self.collections[*index].clone()))
    }

    fn list_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<AudioSource>> {
        let Some(members) = self.members.get(collection_id.as_str()) else {
            return Err(LibraryError::NotFound(collection_id.to_string()));
        };
        Ok(members
            .iter()
            .filter_map(|content_id| self.track_index.get(content_id))
            .map(|index| self.source(&self.tracks[*index]))
            .collect())
    }

    fn list_collection_entries(
        &self,
        collection_id: &CollectionId,
    ) -> Result<Vec<CollectionEntry>> {
        let Some(members) = self.members.get(collection_id.as_str()) else {
            return Err(LibraryError::NotFound(collection_id.to_string()));
        };
        Ok(members
            .iter()
            .enumerate()
            .map(|(index, content_id)| CollectionEntry {
                id: CollectionEntryId::new(format!("{}:entry:{index}", collection_id)),
                collection_id: collection_id.clone(),
                track_id: TrackId::new(format!("rekordbox:track:{content_id}")),
                position: Some(index as i32),
            })
            .collect())
    }
}

impl Migratable for RekordboxLibrary {
    fn migrate(
        &self,
        target: &mut dyn WritableLibrary,
        options: &MigrateOptions,
    ) -> Result<MigrateReport> {
        let mut report = MigrateReport::default();

        // Snapshot the target once: re-imports must skip lists that already
        // exist instead of duplicating them.
        let existing_collections = target.list_collections()?;
        let mut existing_lists: HashSet<(String, bool)> = HashSet::new();
        for collection in &existing_collections {
            if collection.collection_type() == library_core::CollectionType::Playlist {
                existing_lists.insert((collection.name.trim().to_string(), collection.sortable()));
            }
        }

        // Import every loaded track once, remembering the target id per
        // Rekordbox content id so lists and cues can reference it.
        let mut target_ids: HashMap<&str, TrackId> = HashMap::with_capacity(self.tracks.len());
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
                    target_ids.insert(track.content_id.as_str(), imported.source.id().clone());
                }
                Err(err) => {
                    report.failed += 1;
                    report
                        .errors
                        .push(format!("{}: {err}", path_label(&track.path)));
                }
            }
        }

        // Cues belong to a track, so they land right after its import and only
        // when the track made it into the target.
        if options.include_cues {
            for (content_id, cues) in &self.cues {
                let Some(track_id) = target_ids.get(content_id.as_str()) else {
                    continue;
                };
                for cue in &cues.hot_cues {
                    match target.import_hot_cue(track_id, cue) {
                        Ok(()) => report.hot_cues_imported += 1,
                        Err(err) => {
                            report.failed += 1;
                            report
                                .errors
                                .push(format!("{} cue {}: {err}", content_id, cue.slot));
                        }
                    }
                }
                for saved in &cues.loops {
                    match target.import_loop(track_id, saved) {
                        Ok(()) => report.loops_imported += 1,
                        Err(err) => {
                            report.failed += 1;
                            report
                                .errors
                                .push(format!("{} loop {}: {err}", content_id, saved.slot));
                        }
                    }
                }
                if let Some(cue_ms) = cues.main_cue_ms {
                    if let Err(err) = target.import_track_cue(track_id, cue_ms) {
                        report.failed += 1;
                        report.errors.push(format!("{content_id} cue: {err}"));
                    }
                }
            }
        }

        // Tracks are only reachable through a collection, so every imported
        // track also goes into a catch-all "Rekordbox" playlist (a set, so
        // re-runs merge instead of duplicating).
        if !target_ids.is_empty() {
            let catch_all_id = match existing_collections.iter().find(|collection| {
                collection.collection_type() == library_core::CollectionType::Playlist
                    && !collection.sortable()
                    && collection.name.trim() == CATCH_ALL_NAME
            }) {
                Some(existing) => Some(existing.id.clone()),
                None => {
                    match target.add_collection(&NewCollection::playlist(CATCH_ALL_NAME, false)) {
                        Ok(created) => {
                            existing_lists.insert((CATCH_ALL_NAME.to_string(), false));
                            Some(created.id)
                        }
                        Err(err) => {
                            report.failed += 1;
                            report.errors.push(format!("{CATCH_ALL_NAME}: {err}"));
                            None
                        }
                    }
                }
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

        if options.include_playlists {
            let mut lists: Vec<&Collection> = self.collections.iter().collect();
            lists.sort_by_key(|collection| collection.name.to_lowercase());

            for collection in lists {
                let name = collection.name.trim().to_string();
                // A blank name is not an identity: unnamed lists always import.
                let dedupe = options.skip_existing_lists && !name.is_empty();
                if dedupe && existing_lists.contains(&(name.clone(), true)) {
                    report.collections_skipped += 1;
                    continue;
                }

                let created = match target.add_collection(&NewCollection::playlist(&name, true)) {
                    Ok(created) => {
                        if !name.is_empty() {
                            existing_lists.insert((name.clone(), true));
                        }
                        created
                    }
                    Err(err) => {
                        report.failed += 1;
                        report.errors.push(format!("{}: {err}", collection.name));
                        continue;
                    }
                };

                let members = self
                    .members
                    .get(collection.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                for (position, content_id) in members.iter().enumerate() {
                    let Some(track_id) = target_ids.get(content_id.as_str()) else {
                        continue;
                    };
                    if let Err(err) =
                        target.add_collection_entry(&created.id, track_id, Some(position as i32))
                    {
                        report.failed += 1;
                        report.errors.push(format!("{}: {err}", collection.name));
                    }
                }
                report.playlists_imported += 1;
            }
        }

        Ok(report)
    }
}

/// Default Rekordbox `master.db` location for this machine, when it exists.
///
/// Rekordbox records the database directory in `<app data>/Pioneer/rekordbox6/
/// rekordbox3.settings` (`<VALUE name="masterDbDirectory" val="…"/>`). The
/// search tries the settings-derived directory first, then the conventional
/// locations, so an install with an unusual settings file is still found.
/// `REKORDBOX_DATABASE` (a file) and `REKORDBOX_SETTINGS_DIR` (a directory)
/// override the search; tests use the same hooks.
pub fn default_database_path() -> Option<PathBuf> {
    candidate_database_paths()
        .into_iter()
        .find(|path| path.is_file())
}

fn candidate_database_paths() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(explicit) = nonempty_env("REKORDBOX_DATABASE") {
        candidates.push(explicit);
    }
    if let Some(dir) = nonempty_env("REKORDBOX_SETTINGS_DIR") {
        candidates.push(dir.join("master.db"));
        candidates.extend(settings_file_database(&dir));
    }

    for dir in app_dirs() {
        candidates.extend(settings_file_database(&dir));
    }
    for dir in conventional_library_dirs() {
        candidates.push(dir.join("master.db"));
    }
    candidates
}

/// Directories that hold `rekordbox3.settings`.
fn app_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let app_data = if cfg!(target_os = "windows") {
        nonempty_env("APPDATA")
    } else if cfg!(target_os = "macos") {
        nonempty_env("HOME").map(|home| home.join("Library/Application Support"))
    } else {
        // Rekordbox does not run on Linux; the env overrides above are the
        // supported route there (Wine prefixes included).
        None
    };
    if let Some(app_data) = app_data {
        for name in ["rekordbox6", "rekordbox"] {
            dirs.push(app_data.join("Pioneer").join(name));
        }
    }
    dirs
}

/// Conventional `master.db` directories below a Pioneer application-data root.
fn conventional_library_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(app_data) = nonempty_env("APPDATA") {
        dirs.push(app_data.join("Pioneer").join("rekordbox"));
    }
    if let Some(home) = nonempty_env("HOME") {
        // macOS puts the library outside Application Support.
        dirs.push(home.join("Library/Pioneer/rekordbox"));
    }
    dirs
}

/// Database paths named by a `rekordbox3.settings` file in `dir`.
fn settings_file_database(dir: &Path) -> Vec<PathBuf> {
    let settings = dir.join("rekordbox3.settings");
    let Ok(text) = std::fs::read_to_string(&settings) else {
        return Vec::new();
    };
    master_db_directory(&text)
        .map(|directory| vec![directory.join("master.db")])
        .unwrap_or_default()
}

/// `masterDbDirectory` from a `rekordbox3.settings` document.
///
/// The file is a flat `<SETTINGS>` list of `<VALUE name="…" val="…"/>` rows;
/// parsing the one attribute pair this adapter needs keeps an XML dependency
/// out of the tree and tolerates the attribute order Rekordbox happens to use.
fn master_db_directory(settings: &str) -> Option<PathBuf> {
    for tag in settings.split('<').skip(1) {
        if !tag.to_ascii_lowercase().starts_with("value") {
            continue;
        }
        let Some(name) = attribute(tag, "name") else {
            continue;
        };
        if name != "masterDbDirectory" {
            continue;
        }
        if let Some(value) = attribute(tag, "val") {
            return Some(PathBuf::from(unescape_xml(&value)));
        }
    }
    None
}

/// Value of `key="…"` (also `key='…'`) inside one tag.
///
/// Attributes are space-separated, so a match must start a token: without the
/// boundary check `myval="x"` would satisfy a lookup for `val`. Values keep
/// their spaces, which absolute paths on Windows need.
fn attribute(tag: &str, key: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{key}=");
    let mut from = 0;
    while let Some(offset) = lower[from..].find(&needle) {
        let start = from + offset;
        if start == 0 || lower.as_bytes()[start - 1].is_ascii_whitespace() {
            let rest = &tag[start + needle.len()..];
            let quote = rest.chars().next()?;
            if quote != '"' && quote != '\'' {
                return None;
            }
            let end = rest[1..].find(quote)?;
            return Some(rest[1..=end].to_string());
        }
        from = start + 1;
    }
    None
}

/// Minimal XML entity decoding (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`).
fn unescape_xml(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn is_rekordbox5_file(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        name.to_string_lossy()
            .eq_ignore_ascii_case(schema::REKORDBOX5_FILE)
    })
}

fn nonempty_env(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn parse_track_id(id: &TrackId) -> Option<String> {
    id.as_str()
        .strip_prefix("rekordbox:track:")
        .map(str::to_string)
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Folder-relative name of a playlist.
///
/// The libary model has no `parent_id`, so a nested playlist is flattened into
/// `Folder / Child`. A top-level list keeps its bare name, and two same-named
/// lists in different folders stay distinguishable.
fn list_name(playlist: &PlaylistRow, by_id: &HashMap<&str, &PlaylistRow>) -> String {
    let mut prefix = Vec::new();
    let mut parent = playlist.parent.as_deref();
    // Bound the walk so a corrupt (cyclic) `ParentID` cannot hang the import.
    for _ in 0..16 {
        let Some(id) = parent else { break };
        let Some(folder) = by_id.get(id).copied() else {
            break;
        };
        if !folder.is_folder() {
            break;
        }
        prefix.push(folder.name.trim().to_string());
        parent = folder.parent.as_deref();
    }
    prefix.reverse();
    prefix.push(playlist.name.trim().to_string());
    prefix.join(FOLDER_SEPARATOR)
}

/// Lookup tables behind `djmdContent`'s foreign keys.
#[derive(Default)]
struct Lookups {
    artists: HashMap<String, String>,
    albums: HashMap<String, String>,
    genres: HashMap<String, String>,
    keys: HashMap<String, String>,
    colors: HashMap<String, ColorLabel>,
}

fn load_lookups(conn: &Connection, schema: &Schema) -> Result<Lookups> {
    let mut lookups = Lookups::default();
    if schema.artists {
        lookups.artists = load_names(conn, "SELECT ID, Name FROM djmdArtist")?;
    }
    if schema.albums {
        lookups.albums = load_names(conn, "SELECT ID, Name FROM djmdAlbum")?;
    }
    if schema.genres {
        lookups.genres = load_names(conn, "SELECT ID, Name FROM djmdGenre")?;
    }
    if schema.keys {
        lookups.keys = load_names(conn, "SELECT ID, ScaleName FROM djmdKey")?;
    }
    if schema.colors {
        lookups.colors = load_color_labels(conn)?;
    }
    Ok(lookups)
}

/// `(id, name)` rows of a lookup table; blank names are dropped.
fn load_names(conn: &Connection, sql: &'static str) -> Result<HashMap<String, String>> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| backend(format!("prepare lookup query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| backend(format!("query lookup: {e}")))?;

    let mut map = HashMap::new();
    for row in rows {
        let (id, name) = row.map_err(|e| backend(format!("read lookup row: {e}")))?;
        if let Some(name) = clean(name) {
            map.insert(id, name);
        }
    }
    Ok(map)
}

/// `djmdColor.ID → ColorLabel`.
///
/// Real Rekordbox 6/7 libraries leave `ColorCode` NULL and put the palette name
/// in `Commnt` (`"Pink"` … `"Purple"`), so the name is the primary source; a
/// numeric `ColorCode` and the one-based row id are fallbacks for databases
/// that only carry numbers.
fn load_color_labels(conn: &Connection) -> Result<HashMap<String, ColorLabel>> {
    let mut stmt = conn
        .prepare("SELECT ID, ColorCode, Commnt FROM djmdColor")
        .map_err(|e| backend(format!("prepare color query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| backend(format!("query colors: {e}")))?;

    let mut map = HashMap::new();
    for row in rows {
        let (id, code, name) = row.map_err(|e| backend(format!("read color row: {e}")))?;
        if let Some(label) = color_label(name.as_deref(), code, &id) {
            map.insert(id, label);
        }
    }
    Ok(map)
}

/// Palette name first (`"Red"`), then a one-based `ColorCode`, then the row id.
fn color_label(name: Option<&str>, code: Option<i64>, id: &str) -> Option<ColorLabel> {
    if let Some(label) = name.and_then(|name| name.trim().parse::<ColorLabel>().ok()) {
        return Some(label);
    }
    let one_based = |value: i64| -> Option<ColorLabel> {
        let index = usize::try_from(value).ok()?.checked_sub(1)?;
        ColorLabel::ALL.get(index).copied()
    };
    code.and_then(one_based)
        .or_else(|| id.trim().parse::<i64>().ok().and_then(one_based))
}

fn load_tracks(conn: &Connection, lookups: &Lookups) -> Result<Vec<RekordboxTrack>> {
    let mut stmt = conn
        .prepare(
            "SELECT ID, Title, ArtistID, AlbumID, GenreID, KeyID, ColorID, BPM, Length, \
                    SampleRate, BitRate, Rating, Commnt, ReleaseYear, DiscNo, TrackNo, ISRC, \
                    FolderPath, FileNameL, FileNameS \
             FROM djmdContent",
        )
        .map_err(|e| backend(format!("prepare track query: {e}")))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<f64>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<i64>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<i64>>(13)?,
                row.get::<_, Option<i64>>(14)?,
                row.get::<_, Option<i64>>(15)?,
                row.get::<_, Option<String>>(16)?,
                row.get::<_, Option<String>>(17)?,
                row.get::<_, Option<String>>(18)?,
                row.get::<_, Option<String>>(19)?,
            ))
        })
        .map_err(|e| backend(format!("query tracks: {e}")))?;

    let mut tracks = Vec::new();
    for raw in rows {
        let row = raw.map_err(|e| backend(format!("read track row: {e}")))?;
        let (
            id,
            title,
            artist,
            album,
            genre,
            key,
            color,
            bpm,
            length,
            sample_rate,
            bitrate,
            rating,
            comment,
            year,
            disc,
            number,
            isrc,
            folder,
            file_long,
            file_short,
        ) = row;

        // `FileNameL` is the full name; `FileNameS` is the truncated fallback
        // some libraries only fill in.
        let file = file_long.or(file_short);
        let Some(path) = track_path(folder.as_deref(), file.as_deref()) else {
            continue;
        };
        let missing = !path.is_file();
        let metadata = TrackMetadata {
            title: clean(title).or_else(|| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            }),
            artist: join_lookup(artist, &lookups.artists),
            album: join_lookup(album, &lookups.albums),
            genre: join_lookup(genre, &lookups.genres),
            bpm: bpm
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(normalize_bpm),
            key: join_lookup(key, &lookups.keys).map(|key| camelot_to_musical(&key).unwrap_or(key)),
            duration_ms: length
                .filter(|value| *value > 0)
                .map(|seconds| seconds.saturating_mul(1000).min(i64::from(i32::MAX)) as i32),
            sample_rate: sample_rate
                .filter(|value| *value > 0)
                .map(|value| value.min(i64::from(u32::MAX)) as u32),
            bitrate_kbps: bitrate.filter(|value| *value > 0).map(normalize_bitrate),
            isrc: clean(isrc),
            rating: rating.and_then(normalize_rating),
            comment: clean(comment),
            year: year.and_then(positive_u16),
            disc_number: disc.and_then(positive_u16),
            track_number: number.and_then(positive_u16),
            color_label: color.and_then(|id| lookups.colors.get(&id).copied()),
            ..TrackMetadata::default()
        };

        tracks.push(RekordboxTrack {
            content_id: id,
            path,
            metadata,
            missing,
        });
    }
    Ok(tracks)
}

/// Absolute track path from `FolderPath` + `FileNameL`.
///
/// Rekordbox 6/7 store the **full file path** in `FolderPath` and repeat the
/// name in `FileNameL` (verified against a real library: 1995/1995 rows end
/// with the file name). Other tools store only the directory, so both layouts
/// are accepted instead of doubling the file name. Backslash separators are
/// normalised, so a library copied to another platform still resolves.
fn track_path(folder: Option<&str>, file: Option<&str>) -> Option<PathBuf> {
    let file = clean(file.map(str::to_string))?;
    let folder = folder
        .map(|folder| folder.trim().replace('\\', "/"))
        .unwrap_or_default();
    let folder = folder.trim_end_matches('/');

    if !folder.is_empty() && folder.to_lowercase().ends_with(&file.to_lowercase()) {
        return Some(PathBuf::from(folder));
    }
    let joined = if folder.is_empty() {
        file
    } else {
        format!("{folder}/{file}")
    };
    Some(PathBuf::from(joined))
}

fn join_lookup(id: Option<String>, table: &HashMap<String, String>) -> Option<String> {
    id.and_then(|id| table.get(&id).cloned())
}

/// Rekordbox stores BPM as an integer; some libraries scale it by 100.
fn normalize_bpm(bpm: f64) -> f64 {
    if bpm > 1000.0 {
        bpm / 100.0
    } else {
        bpm
    }
}

/// Rekordbox stores bitrate in kbps in current libraries and in bps in older
/// ones; anything above 10 000 kbps is bytes per second.
fn normalize_bitrate(bitrate: i64) -> u32 {
    let kbps = if bitrate > 10_000 {
        bitrate / 1000
    } else {
        bitrate
    };
    kbps.min(i64::from(u32::MAX)) as u32
}

/// Rekordbox ratings are 0–5 stars; XML exports use 0–255.
///
/// A value between the two scales (6–50) belongs to neither, so it is dropped
/// rather than rounded up to one star.
fn normalize_rating(value: i64) -> Option<u8> {
    if value <= 0 {
        return None;
    }
    if value <= 5 {
        return Some(value as u8);
    }
    (value >= 51).then(|| (value / 51).clamp(1, 5) as u8)
}

fn positive_u16(value: i64) -> Option<u16> {
    u16::try_from(value).ok().filter(|value| *value > 0)
}

/// Hot cues and saved loops, grouped by content id.
fn load_cues(
    conn: &Connection,
    schema: &Schema,
    tracks: &[RekordboxTrack],
) -> Result<HashMap<String, CueSet>> {
    if !schema.cues {
        return Ok(HashMap::new());
    }

    let bpm_by_id: HashMap<&str, f64> = tracks
        .iter()
        .filter_map(|track| {
            track
                .metadata
                .bpm
                .map(|bpm| (track.content_id.as_str(), bpm))
        })
        .collect();

    let mut stmt = conn
        .prepare(
            "SELECT ContentID, InMsec, OutMsec, Kind, Color, Comment \
             FROM djmdCue ORDER BY ContentID, InMsec",
        )
        .map_err(|e| backend(format!("prepare cue query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(|e| backend(format!("query cues: {e}")))?;

    let mut cues: HashMap<String, CueSet> = HashMap::new();
    let mut next_loop_slot: HashMap<String, u8> = HashMap::new();
    for row in rows {
        let (content_id, in_ms, out_ms, kind, color, comment) =
            row.map_err(|e| backend(format!("read cue row: {e}")))?;
        let Some(content_id) = content_id else {
            continue;
        };
        let Some(in_ms) = in_ms.filter(|value| *value >= 0) else {
            continue;
        };
        let in_ms = i32::try_from(in_ms).unwrap_or(i32::MAX);
        let out_ms = out_ms
            .filter(|value| *value > i64::from(in_ms))
            .and_then(|value| i32::try_from(value).ok());
        let label = clean(comment);
        let color = cue_color(color);
        let set = cues.entry(content_id.clone()).or_default();

        if kind.is_some_and(|kind| kind > MEMORY_CUE_KIND) {
            // A cue beyond the sixteen slots the store has is dropped rather
            // than collapsed onto slot 15, where it would shadow a real cue.
            let Some(slot) = kind.and_then(hot_cue_slot) else {
                continue;
            };
            if set.hot_cues.iter().any(|cue| cue.slot == slot) {
                continue;
            }
            set.hot_cues.push(HotCueImport {
                slot,
                position_ms: in_ms,
                loop_length_beats: out_ms
                    .and_then(|out| {
                        loop_beats(in_ms, out, bpm_by_id.get(content_id.as_str()).copied())
                    })
                    .map(|beats| beats as i32),
                color,
                label,
            });
        } else {
            // A memory cue is the track's main cue; the earliest one wins. The
            // query already orders by `InMsec`, but taking the minimum here keeps
            // that contract local instead of depending on the SQL order. A memory
            // *loop* (out point set) is additionally a saved loop.
            if set.main_cue_ms.is_none_or(|earliest| in_ms < earliest) {
                set.main_cue_ms = Some(in_ms);
            }
            if let Some(out_ms) = out_ms {
                let slot = next_loop_slot.get(&content_id).copied().unwrap_or(0);
                if slot > 15 {
                    continue;
                }
                next_loop_slot.insert(content_id, slot + 1);
                set.loops.push(LoopImport {
                    slot,
                    in_ms,
                    out_ms,
                    label,
                    color,
                });
            }
        }
    }

    // Drop sets that belong to rows this adapter did not load (no resolvable
    // path), so `cues_for` never describes a track `get_track` reports absent.
    let loaded: HashSet<&str> = tracks
        .iter()
        .map(|track| track.content_id.as_str())
        .collect();
    cues.retain(|content_id, _| loaded.contains(content_id.as_str()));

    // Slot order, matching what the deck and `list_track_hot_cues` produce.
    for set in cues.values_mut() {
        set.hot_cues.sort_by_key(|cue| cue.slot);
        set.loops.sort_by_key(|saved| saved.slot);
    }

    Ok(cues)
}

/// Pad slot for a `djmdCue.Kind`: the one-based hot-cue number minus one.
///
/// `None` for a memory cue (`Kind = 0`) and for a kind past the sixteen slots
/// the store keeps, so an out-of-range cue is dropped instead of saturating.
fn hot_cue_slot(kind: i64) -> Option<u8> {
    let slot = u8::try_from(kind.checked_sub(1)?).ok()?;
    (slot <= 15).then_some(slot)
}

/// Loop length in whole beats from its millisecond span and the track's BPM.
fn loop_beats(in_ms: i32, out_ms: i32, bpm: Option<f64>) -> Option<u32> {
    let bpm = bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.0)?;
    let span = f64::from(out_ms - in_ms);
    let beats = (span * bpm / 60_000.0).round();
    (beats >= 1.0).then(|| beats.min(f64::from(u32::MAX)) as u32)
}

/// `djmdCue.Color` as `#RRGGBB`.
///
/// The column is the one-based id in Rekordbox's cue palette (`1..=8`). A real
/// Rekordbox 7 library uses `255` for a cue with no colour assigned and `-1`
/// for none at all, so anything outside the palette maps to `None` rather than
/// a guessed hue.
fn cue_color(color: Option<i64>) -> Option<String> {
    let index = usize::try_from(color.filter(|value| *value > 0)?).ok()?;
    CUE_COLORS.get(index - 1).map(|hex| (*hex).to_string())
}

fn load_playlists(conn: &Connection) -> Result<Vec<PlaylistRow>> {
    let mut stmt = conn
        .prepare("SELECT ID, Name, Attribute, ParentID FROM djmdPlaylist")
        .map_err(|e| backend(format!("prepare playlist query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PlaylistRow {
                id: row.get::<_, String>(0)?,
                name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                attribute: row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                parent: row
                    .get::<_, Option<String>>(3)?
                    .filter(|id| !id.trim().is_empty()),
            })
        })
        .map_err(|e| backend(format!("query playlists: {e}")))?;

    let mut playlists = Vec::new();
    for row in rows {
        playlists.push(row.map_err(|e| backend(format!("read playlist row: {e}")))?);
    }
    Ok(playlists)
}

/// Membership rows grouped by playlist, in `TrackNo` order.
fn load_memberships(conn: &Connection) -> Result<HashMap<String, Vec<String>>> {
    let mut stmt = conn
        .prepare(
            "SELECT PlaylistID, ContentID FROM djmdSongPlaylist \
             ORDER BY PlaylistID, TrackNo, ID",
        )
        .map_err(|e| backend(format!("prepare playlist member query: {e}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        })
        .map_err(|e| backend(format!("query playlist members: {e}")))?;

    let mut members: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (playlist, content) = row.map_err(|e| backend(format!("read member row: {e}")))?;
        if let (Some(playlist), Some(content)) = (playlist, content) {
            members.entry(playlist).or_default().push(content);
        }
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playlist(id: &str, name: &str, attribute: i64, parent: Option<&str>) -> PlaylistRow {
        PlaylistRow {
            id: id.to_string(),
            name: name.to_string(),
            attribute,
            parent: parent.map(str::to_string),
        }
    }

    fn playlists_by_id(rows: &[PlaylistRow]) -> HashMap<&str, &PlaylistRow> {
        rows.iter().map(|row| (row.id.as_str(), row)).collect()
    }

    #[test]
    fn lists_flatten_their_folder_path_and_keep_root_names() {
        let rows = vec![
            playlist("1", "House", ATTRIBUTE_FOLDER, None),
            playlist("2", "Deep", ATTRIBUTE_FOLDER, Some("1")),
            playlist("3", "Warmup", 0, Some("2")),
            playlist("4", "Top", 0, None),
        ];
        let by_id = playlists_by_id(&rows);
        assert_eq!(list_name(&rows[2], &by_id), "House / Deep / Warmup");
        assert_eq!(list_name(&rows[3], &by_id), "Top");
    }

    #[test]
    fn list_names_do_not_differ_by_case_or_whitespace_around_folders() {
        let rows = vec![
            playlist("1", " House ", ATTRIBUTE_FOLDER, None),
            playlist("2", "Warmup", 0, Some("1")),
        ];
        assert_eq!(
            list_name(&rows[1], &playlists_by_id(&rows)),
            "House / Warmup"
        );
    }

    #[test]
    fn a_cyclic_parent_chain_does_not_hang() {
        let rows = vec![
            playlist("1", "A", ATTRIBUTE_FOLDER, Some("2")),
            playlist("2", "B", ATTRIBUTE_FOLDER, Some("1")),
            playlist("3", "List", 0, Some("1")),
        ];
        let name = list_name(&rows[2], &playlists_by_id(&rows));
        assert!(name.ends_with("List"), "{name}");
    }

    #[test]
    fn settings_file_yields_the_master_db_directory() {
        let settings = r#"<?xml version="1.0" encoding="UTF-8"?>
<SETTINGS>
  <VALUE name="lastRekordboxVer" val="7.1.4"/>
  <VALUE val="/Users/dj/Library/Pioneer/rekordbox" name="masterDbDirectory"/>
  <VALUE name="audioDevice" ><DEVICESETUP deviceName="X"/></VALUE>
</SETTINGS>"#;
        assert_eq!(
            master_db_directory(settings),
            Some(PathBuf::from("/Users/dj/Library/Pioneer/rekordbox"))
        );
        assert_eq!(master_db_directory("<SETTINGS/>"), None);
    }

    #[test]
    fn attributes_need_a_token_boundary_and_keep_spaces() {
        // `myval` must not answer a lookup for `val`, and the value keeps the
        // space an absolute path can carry.
        assert_eq!(
            attribute(r#"VALUE myval="wrong" val="C:\Users\A B\Pioneer""#, "val").as_deref(),
            Some(r"C:\Users\A B\Pioneer")
        );
        assert_eq!(
            attribute(r#"VALUE name="masterDbDirectory""#, "name").as_deref(),
            Some("masterDbDirectory")
        );
        assert_eq!(attribute(r#"VALUE other="x""#, "name"), None);
    }

    #[test]
    fn settings_paths_are_unescaped() {
        let settings = r#"<SETTINGS><VALUE name="masterDbDirectory" val="C:\Users\Dj &amp; Co\Pioneer\rekordbox"/></SETTINGS>"#;
        assert_eq!(
            master_db_directory(settings),
            Some(PathBuf::from("C:\\Users\\Dj & Co\\Pioneer\\rekordbox"))
        );
    }

    #[test]
    fn track_paths_join_both_separators() {
        assert_eq!(
            track_path(Some("C:\\Music\\House"), Some("a.mp3")),
            Some(PathBuf::from("C:/Music/House/a.mp3"))
        );
        assert_eq!(
            track_path(Some("/music/house/"), Some("a.flac")),
            Some(PathBuf::from("/music/house/a.flac"))
        );
        assert_eq!(
            track_path(None, Some("a.wav")),
            Some(PathBuf::from("a.wav"))
        );
        assert_eq!(track_path(Some("/music"), None), None);
    }

    #[test]
    fn a_folder_path_that_already_holds_the_file_is_not_doubled() {
        // The layout real Rekordbox 6/7 libraries use (verified against one).
        assert_eq!(
            track_path(
                Some("G:/Meu Drive/Músicas/Pagode/Grupo Y-no - YASEI !!.mp3"),
                Some("Grupo Y-no - YASEI !!.mp3")
            ),
            Some(PathBuf::from(
                "G:/Meu Drive/Músicas/Pagode/Grupo Y-no - YASEI !!.mp3"
            ))
        );
        // Windows separators and case differences still match.
        assert_eq!(
            track_path(Some("C:\\Music\\House\\Track.MP3"), Some("track.mp3")),
            Some(PathBuf::from("C:/Music/House/Track.MP3"))
        );
        // A directory whose name merely ends with the file name is still a
        // directory: only a path separator boundary counts... except that the
        // suffix check cannot tell, so the file name wins.
        assert_eq!(
            track_path(Some("/music/demo.mp3"), Some("demo.mp3")),
            Some(PathBuf::from("/music/demo.mp3"))
        );
    }

    #[test]
    fn colour_labels_prefer_the_palette_name() {
        use crate::rekordbox::color_label;

        // Real libraries: ColorCode NULL, name in Commnt.
        assert_eq!(color_label(Some("Red"), None, "2"), Some(ColorLabel::Red));
        assert_eq!(
            color_label(Some(" aqua "), None, "6"),
            Some(ColorLabel::Aqua)
        );
        // Numeric fallbacks when the name is missing or unknown.
        assert_eq!(color_label(None, Some(2), "x"), Some(ColorLabel::Red));
        assert_eq!(
            color_label(Some("mauve"), Some(7), "x"),
            Some(ColorLabel::Blue)
        );
        assert_eq!(color_label(None, None, "5"), Some(ColorLabel::Green));
        assert_eq!(color_label(None, None, "x"), None);
        assert_eq!(color_label(None, Some(99), "x"), None);
    }

    #[test]
    fn ratings_scaled_by_both_conventions() {
        assert_eq!(normalize_rating(0), None);
        assert_eq!(normalize_rating(-1), None);
        assert_eq!(normalize_rating(3), Some(3));
        assert_eq!(normalize_rating(6), None, "neither a star count nor 0-255");
        assert_eq!(normalize_rating(51), Some(1));
        assert_eq!(normalize_rating(204), Some(4));
    }

    #[test]
    fn bpm_and_bitrate_normalise_legacy_scaling() {
        assert_eq!(normalize_bpm(128.0), 128.0);
        assert_eq!(normalize_bpm(12_800.0), 128.0);
        assert_eq!(normalize_bitrate(320), 320);
        assert_eq!(normalize_bitrate(320_000), 320);
    }

    #[test]
    fn hot_cue_slots_come_from_the_kind_and_stop_at_the_pad_count() {
        assert_eq!(hot_cue_slot(0), None, "a memory cue has no pad");
        assert_eq!(hot_cue_slot(1), Some(0), "cue A");
        assert_eq!(hot_cue_slot(8), Some(7), "cue H");
        assert_eq!(hot_cue_slot(16), Some(15));
        // Past the sixteen slots the cue is dropped, not clamped onto 15.
        assert_eq!(hot_cue_slot(17), None);
        assert_eq!(hot_cue_slot(300), None);
    }

    #[test]
    fn cue_colors_map_palette_indexes_only() {
        assert_eq!(cue_color(None), None);
        assert_eq!(cue_color(Some(-1)), None);
        assert_eq!(cue_color(Some(0)), None);
        assert_eq!(cue_color(Some(1)).as_deref(), Some("#FF007F"));
        assert_eq!(cue_color(Some(8)).as_deref(), Some("#660099"));
        // 255 is what a real Rekordbox 7 library writes for "no cue colour";
        // an out-of-palette value must never become a guessed hue.
        assert_eq!(cue_color(Some(255)), None);
        assert_eq!(cue_color(Some(0x12_34_56)), None);
    }
}
