//! Turns a library selection into the flat track and playlist lists the
//! writers consume.
//!
//! Placement rules:
//!
//! - a track reachable through a selected folder collection is mirrored under
//!   `Contents/<folder name>/…`, preserving the source tree;
//! - any other track is placed as `Contents/<artist>/<album>/<file name>`;
//! - every selected playlist is written to the device database in its stored
//!   order, and every selected folder also gets a playlist named after it so
//!   the tracks are reachable in the player's playlist browser (the folder
//!   mirror itself is only browsable as files).
//!
//! Streams cannot be written to a drive and are skipped.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use library_core::{
    AudioSource, Collection, CollectionId, CollectionType, ExportOptions, ExportSource,
    FileAudioSource, Result, SmartPlaylistRules, TrackId, TrackMetadata,
};

use super::layout::DevicePaths;
use super::path_hash;

/// One track to export.
#[derive(Clone, Debug)]
pub struct PlanTrack {
    /// Library id of the source track.
    pub track_id: TrackId,
    /// Source file on disk.
    pub source_path: std::path::PathBuf,
    /// File-tag metadata of the source track.
    pub metadata: TrackMetadata,
    /// On-drive audio path, starting with a slash.
    pub device_audio_path: String,
    /// On-drive `.DAT` analysis path stored in the device database.
    pub analyze_path: String,
}

/// One playlist to write into the device database.
#[derive(Clone, Debug)]
pub struct PlanPlaylist {
    /// Playlist name shown on the device.
    pub name: String,
    /// Library collection the playlist came from.
    ///
    /// Folder collections also produce a playlist (so the player can reach their
    /// tracks); the id lets the export manifest replace a collection's previous
    /// playlist instead of duplicating it.
    pub source: Option<String>,
    /// Indices into [`Plan::tracks`], in play order.
    pub entries: Vec<usize>,
}

/// Resolved export plan.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    /// Tracks to write, in database order.
    pub tracks: Vec<PlanTrack>,
    /// Playlists to write, in selection order.
    pub playlists: Vec<PlanPlaylist>,
    /// Folder collections that resolved and were mirrored.
    pub folders: usize,
    /// Selection problems (unknown collections, unreadable folders).
    pub warnings: Vec<String>,
}

/// Build the plan for `options` against `source`.
///
/// Reading the library is the only failure mode; individual tracks and
/// collections that cannot be resolved are reported as warnings so the run
/// still exports what it can.
pub fn build(
    source: &dyn ExportSource,
    options: &ExportOptions,
    previous: &HashMap<String, String>,
) -> Result<Plan> {
    let collections = source.list_collections()?;
    let mut plan = Plan::default();

    // Resolve the whole selection before placing anything: the paths of
    // unselected tracks from earlier runs stay reserved, so a new track cannot
    // take one of them.
    let mut folders: Vec<(&Collection, Vec<AudioSource>)> = Vec::new();
    for folder in select(
        &collections,
        &options.folders,
        &[CollectionType::Folder],
        "folder",
        &mut plan.warnings,
    ) {
        match source.list_collection_tracks(&folder.id) {
            Ok(tracks) => folders.push((folder, tracks)),
            Err(err) => plan.warnings.push(format!("folder {}: {err}", folder.name)),
        }
    }
    let mut playlists: Vec<(&Collection, Vec<AudioSource>)> = Vec::new();
    for playlist in select(
        &collections,
        &options.playlists,
        &[CollectionType::Playlist, CollectionType::SmartPlaylist],
        "playlist",
        &mut plan.warnings,
    ) {
        match source.list_collection_tracks(&playlist.id) {
            Ok(tracks) => playlists.push((playlist, tracks)),
            Err(err) => {
                plan.warnings
                    .push(format!("playlist {}: {err}", playlist.name));
            }
        }
    }

    let selected: HashSet<&str> = folders
        .iter()
        .chain(playlists.iter())
        .flat_map(|(_, tracks)| tracks.iter())
        .filter_map(|track| track.file().map(|file| file.id.as_str()))
        .collect();
    let mut index: HashMap<TrackId, usize> = HashMap::new();
    let mut paths = DevicePaths::default();
    for (source_id, device_path) in previous {
        if !selected.contains(source_id.as_str()) {
            paths.reserve(device_path);
        }
    }

    // Folders first: they decide the placement of the tracks they contain.
    let mut folder_members: Vec<(&Collection, Vec<TrackId>)> = Vec::new();
    for (folder, tracks) in &folders {
        let root = folder.fs_path().map(Path::to_path_buf);
        let mut member_ids = Vec::new();
        for track in tracks {
            let Some(file) = track.file() else {
                continue;
            };
            member_ids.push(file.id.clone());
            if index.contains_key(&file.id) {
                continue;
            }
            let device_audio_path = match previous.get(file.id.as_str()) {
                Some(path) => paths.take(path),
                None => {
                    let relative = root
                        .as_deref()
                        .and_then(|root| file.path().strip_prefix(root).ok())
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|| file_name(file));
                    paths.folder_mirror(&folder.name, &relative)
                }
            };
            push_track(&mut plan, &mut index, file, device_audio_path);
        }
        folder_members.push((folder, member_ids));
    }

    for (playlist, tracks) in &playlists {
        // A smart playlist resolves its membership at export time; when its
        // rules need a playing deck and nothing matches, say so instead of
        // silently writing an empty list.
        if tracks.is_empty()
            && playlist
                .smart_rules_json()
                .and_then(|json| SmartPlaylistRules::from_json(json).ok())
                .is_some_and(|rules| rules.uses_deck_reference())
        {
            plan.warnings.push(format!(
                "playlist {}: matched no tracks (deck-relative rules need a playing deck)",
                playlist.name
            ));
        }
        let mut entries = Vec::new();
        for track in tracks {
            let Some(file) = track.file() else {
                continue;
            };
            let track_index = match index.get(&file.id) {
                Some(track_index) => *track_index,
                None => {
                    let device_audio_path = match previous.get(file.id.as_str()) {
                        Some(path) => paths.take(path),
                        None => paths.artist_album(
                            file.metadata.artist.as_deref(),
                            file.metadata.album.as_deref(),
                            file.path()
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or_default(),
                        ),
                    };
                    push_track(&mut plan, &mut index, file, device_audio_path)
                }
            };
            entries.push(track_index);
        }
        plan.playlists.push(PlanPlaylist {
            name: playlist.name.clone(),
            source: Some(playlist.id.to_string()),
            entries,
        });
    }

    // Folders are reachable in the player's playlist browser only as
    // playlists, so each one also lists its tracks.
    plan.folders = folder_members.len();
    for (folder, member_ids) in folder_members {
        let entries = member_ids
            .iter()
            .filter_map(|id| index.get(id).copied())
            .collect();
        plan.playlists.push(PlanPlaylist {
            name: folder.name.clone(),
            source: Some(folder.id.to_string()),
            entries,
        });
    }

    Ok(plan)
}

/// Append a track to the plan and remember its index.
fn push_track(
    plan: &mut Plan,
    index: &mut HashMap<TrackId, usize>,
    file: &FileAudioSource,
    device_audio_path: String,
) -> usize {
    let analyze_path = path_hash::anlz_file(&device_audio_path, "DAT");
    let track_index = plan.tracks.len();
    plan.tracks.push(PlanTrack {
        track_id: file.id.clone(),
        source_path: file.path().to_path_buf(),
        metadata: file.metadata.clone(),
        device_audio_path,
        analyze_path,
    });
    index.insert(file.id.clone(), track_index);
    track_index
}

fn file_name(file: &FileAudioSource) -> std::path::PathBuf {
    std::path::PathBuf::from(
        file.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default(),
    )
}

/// Resolve the selected collection ids of the expected type, warning about
/// anything unknown or of the wrong kind.
fn select<'a>(
    collections: &'a [Collection],
    ids: &[CollectionId],
    kinds: &[CollectionType],
    label: &str,
    warnings: &mut Vec<String>,
) -> Vec<&'a Collection> {
    ids.iter()
        .filter_map(
            |id| match collections.iter().find(|collection| &collection.id == id) {
                Some(collection) if kinds.contains(&collection.collection_type()) => {
                    Some(collection)
                }
                Some(_) => {
                    warnings.push(format!("{id} is not a {label} collection"));
                    None
                }
                None => {
                    warnings.push(format!("{label} {id} is not in the library"));
                    None
                }
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use library_core::{
        AudioSource, CollectionConfig, CollectionEntry, Library, StreamAudioSource, StreamProvider,
    };
    use std::path::PathBuf;

    /// Library stub: fixed collections and per-collection tracks.
    struct FakeLibrary {
        collections: Vec<Collection>,
        tracks: HashMap<String, Vec<AudioSource>>,
    }

    impl FakeLibrary {
        fn new() -> Self {
            Self {
                collections: Vec::new(),
                tracks: HashMap::new(),
            }
        }

        fn folder(&mut self, id: &str, name: &str, fs_path: &str) -> Collection {
            let collection = Collection {
                id: CollectionId::new(id),
                name: name.to_string(),
                config: CollectionConfig::Folder {
                    fs_path: PathBuf::from(fs_path),
                    scan_folder_tree: true,
                },
            };
            self.collections.push(collection.clone());
            collection
        }

        fn playlist(&mut self, id: &str, name: &str, sortable: bool) -> Collection {
            let collection = Collection {
                id: CollectionId::new(id),
                name: name.to_string(),
                config: CollectionConfig::Playlist { sortable },
            };
            self.collections.push(collection.clone());
            collection
        }

        fn file(&mut self, collection_id: &str, path: &str, artist: &str, album: &str) -> TrackId {
            let id = TrackId::new(path);
            let metadata = TrackMetadata {
                artist: Some(artist.to_string()),
                album: Some(album.to_string()),
                ..TrackMetadata::default()
            };
            self.tracks
                .entry(collection_id.to_string())
                .or_default()
                .push(AudioSource::File(FileAudioSource::new(
                    id.clone(),
                    PathBuf::from(path),
                    metadata,
                )));
            id
        }

        fn stream(&mut self, collection_id: &str, uri: &str) -> TrackId {
            let id = TrackId::new(uri);
            self.tracks
                .entry(collection_id.to_string())
                .or_default()
                .push(AudioSource::Stream(StreamAudioSource::new(
                    id.clone(),
                    uri,
                    TrackMetadata::default(),
                    Some(StreamProvider::Http),
                )));
            id
        }

        /// Add an already-imported source to another collection.
        fn add_existing(&mut self, collection_id: &str, source: AudioSource) {
            self.tracks
                .entry(collection_id.to_string())
                .or_default()
                .push(source);
        }
    }

    impl Library for FakeLibrary {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn get_track(&self, id: &TrackId) -> Result<Option<AudioSource>> {
            Ok(self
                .tracks
                .values()
                .flatten()
                .find(|source| source.id() == id)
                .cloned())
        }

        fn list_collections(&self) -> Result<Vec<Collection>> {
            Ok(self.collections.clone())
        }

        fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>> {
            Ok(self
                .collections
                .iter()
                .find(|collection| &collection.id == id)
                .cloned())
        }

        fn list_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<AudioSource>> {
            Ok(self
                .tracks
                .get(collection_id.as_str())
                .cloned()
                .unwrap_or_default())
        }

        fn list_collection_entries(
            &self,
            _collection_id: &CollectionId,
        ) -> Result<Vec<CollectionEntry>> {
            Ok(Vec::new())
        }
    }

    impl ExportSource for FakeLibrary {
        fn export_data(&self, _id: &TrackId) -> Result<Option<library_core::TrackExportData>> {
            Ok(None)
        }
    }

    #[test]
    fn folder_tracks_are_mirrored_and_also_listed_as_a_playlist() {
        let mut library = FakeLibrary::new();
        let folder = library.folder("folder-1", "House", "/music/House");
        library.file(
            "folder-1",
            "/music/House/Artist/Album/One.wav",
            "Artist",
            "Album",
        );
        library.file(
            "folder-1",
            "/music/House/Artist/Album/Two.wav",
            "Artist",
            "Album",
        );

        let plan = build(
            &library,
            &ExportOptions {
                playlists: Vec::new(),
                folders: vec![folder.id.clone()],
                refresh_files: false,
            },
            &HashMap::new(),
        )
        .expect("plan");

        assert_eq!(plan.folders, 1);
        assert_eq!(
            plan.tracks
                .iter()
                .map(|track| track.device_audio_path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "/Contents/House/Artist/Album/One.wav",
                "/Contents/House/Artist/Album/Two.wav"
            ]
        );
        assert_eq!(
            plan.tracks[0].analyze_path,
            super::path_hash::anlz_file("/Contents/House/Artist/Album/One.wav", "DAT")
        );
        assert_eq!(plan.playlists.len(), 1);
        assert_eq!(plan.playlists[0].name, "House");
        assert_eq!(plan.playlists[0].source.as_deref(), Some("folder-1"));
        assert_eq!(plan.playlists[0].entries, vec![0, 1]);
        assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    }

    #[test]
    fn playlist_only_tracks_use_the_metadata_layout() {
        let mut library = FakeLibrary::new();
        let playlist = library.playlist("pl-1", "Warmup", true);
        library.file(
            "pl-1",
            "/elsewhere/Deep/Tune.mp3",
            "Slam",
            "Positive Education",
        );

        let plan = build(
            &library,
            &ExportOptions {
                playlists: vec![playlist.id.clone()],
                folders: Vec::new(),
                refresh_files: false,
            },
            &HashMap::new(),
        )
        .expect("plan");

        assert_eq!(plan.folders, 0);
        assert_eq!(
            plan.tracks[0].device_audio_path,
            "/Contents/Slam/Positive Education/Tune.mp3"
        );
        assert_eq!(plan.playlists[0].entries, vec![0]);
    }

    #[test]
    fn folder_placement_wins_over_the_metadata_layout() {
        let mut library = FakeLibrary::new();
        let folder = library.folder("folder-1", "House", "/music");
        let playlist = library.playlist("pl-1", "Warmup", true);
        let id = library.file("folder-1", "/music/Artist/Album/One.wav", "Artist", "Album");
        // The same track is also a member of the playlist.
        let source = library.get_track(&id).expect("lookup").expect("exists");
        library.add_existing("pl-1", source);

        let plan = build(
            &library,
            &ExportOptions {
                playlists: vec![playlist.id.clone()],
                folders: vec![folder.id.clone()],
                refresh_files: false,
            },
            &HashMap::new(),
        )
        .expect("plan");

        assert_eq!(plan.tracks.len(), 1, "the track is planned once");
        assert_eq!(
            plan.tracks[0].device_audio_path,
            "/Contents/House/Artist/Album/One.wav"
        );
        // Playlist entry remaps onto the single planned track.
        assert_eq!(plan.playlists[0].name, "Warmup");
        assert_eq!(plan.playlists[0].entries, vec![0]);
    }

    #[test]
    fn unknown_and_mistyped_selections_are_warnings() {
        let mut library = FakeLibrary::new();
        let folder = library.folder("folder-1", "House", "/music");

        let plan = build(
            &library,
            &ExportOptions {
                playlists: vec![folder.id.clone(), CollectionId::new("missing")],
                folders: Vec::new(),
                refresh_files: false,
            },
            &HashMap::new(),
        )
        .expect("plan");

        assert!(plan.playlists.is_empty());
        assert_eq!(plan.warnings.len(), 2, "{:?}", plan.warnings);
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.contains("not a playlist")));
        assert!(plan
            .warnings
            .iter()
            .any(|warning| warning.contains("not in the library")));
    }

    #[test]
    fn stream_tracks_are_skipped() {
        let mut library = FakeLibrary::new();
        let playlist = library.playlist("pl-1", "Streams", true);
        library.stream("pl-1", "http://example.test/stream");
        library.file("pl-1", "/music/Tune.mp3", "Artist", "Album");

        let plan = build(
            &library,
            &ExportOptions {
                playlists: vec![playlist.id.clone()],
                folders: Vec::new(),
                refresh_files: false,
            },
            &HashMap::new(),
        )
        .expect("plan");

        assert_eq!(plan.tracks.len(), 1);
        assert_eq!(plan.playlists[0].entries, vec![0]);
    }
    #[test]
    fn a_previous_placement_keeps_a_track_at_its_path() {
        let mut library = FakeLibrary::new();
        let playlist = library.playlist("pl-1", "Warmup", true);
        let id = library.file(
            "pl-1",
            "/elsewhere/Deep/Tune.mp3",
            "Slam",
            "Positive Education",
        );
        let previous = HashMap::from([(
            id.to_string(),
            "/Contents/Slam/Positive Education/Tune.mp3".to_string(),
        )]);

        let plan = build(
            &library,
            &ExportOptions {
                playlists: vec![playlist.id.clone()],
                folders: Vec::new(),
                refresh_files: false,
            },
            &previous,
        )
        .expect("plan");

        assert_eq!(
            plan.tracks[0].device_audio_path, "/Contents/Slam/Positive Education/Tune.mp3",
            "a re-export keeps the path the track already has on the drive"
        );
    }

    #[test]
    fn a_stored_path_two_tracks_claim_is_disambiguated() {
        let mut library = FakeLibrary::new();
        let playlist = library.playlist("pl-1", "Warmup", true);
        let first = library.file("pl-1", "/a/Tune.mp3", "Slam", "Positive Education");
        let second = library.file("pl-1", "/b/Tune.mp3", "Slam", "Positive Education");
        // Only a state file that names one path twice can produce this.
        let previous = HashMap::from([
            (
                first.to_string(),
                "/Contents/Slam/Positive Education/Tune.mp3".to_string(),
            ),
            (
                second.to_string(),
                "/Contents/Slam/Positive Education/Tune.mp3".to_string(),
            ),
        ]);

        let plan = build(
            &library,
            &ExportOptions {
                playlists: vec![playlist.id.clone()],
                folders: Vec::new(),
                refresh_files: false,
            },
            &previous,
        )
        .expect("plan");

        assert_eq!(
            plan.tracks
                .iter()
                .map(|track| track.device_audio_path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "/Contents/Slam/Positive Education/Tune.mp3",
                "/Contents/Slam/Positive Education/Tune (2).mp3",
            ],
            "two tracks never share one file"
        );
    }

    #[test]
    fn a_previous_track_outside_the_selection_keeps_its_path() {
        let mut library = FakeLibrary::new();
        // The collection holding the track is not part of this export.
        library.playlist("pl-1", "Warmup", true);
        let kept = library.file(
            "pl-1",
            "/elsewhere/Deep/Tune.mp3",
            "Slam",
            "Positive Education",
        );
        // Another collection holds a same-named file, which would otherwise be
        // placed at exactly the same device path.
        let other = library.playlist("pl-2", "Other", true);
        library.file("pl-2", "/other/Deep/Tune.mp3", "Slam", "Positive Education");
        let previous = HashMap::from([(
            kept.to_string(),
            "/Contents/Slam/Positive Education/Tune.mp3".to_string(),
        )]);

        let plan = build(
            &library,
            &ExportOptions {
                playlists: vec![other.id.clone()],
                folders: Vec::new(),
                refresh_files: false,
            },
            &previous,
        )
        .expect("plan");

        assert_eq!(plan.tracks.len(), 1);
        assert_eq!(
            plan.tracks[0].device_audio_path, "/Contents/Slam/Positive Education/Tune (2).mp3",
            "a track left on the drive keeps its path"
        );
    }
}
