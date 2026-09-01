//! Library capability traits.

use crate::error::Result;
use crate::source::AudioSource;
use crate::types::{
    AnalyzeTrackOptions, Collection, CollectionEntry, CollectionEntryId, CollectionId,
    NewCollection, ScanReport, TrackId, UpdateCollection,
};

/// Read-only library manager access.
pub trait Library: Send + Sync {
    /// Backend name (e.g. `"library"`, `"rekordbox"`).
    fn name(&self) -> &'static str;

    /// Fetch a single source by id.
    fn get_track(&self, id: &TrackId) -> Result<Option<AudioSource>>;

    /// List all collections (folders and playlists), ordered by name.
    fn list_collections(&self) -> Result<Vec<Collection>>;

    /// Fetch a single collection by id.
    fn get_collection(&self, id: &CollectionId) -> Result<Option<Collection>>;

    /// Sources in a collection. Folders use path-prefix on file sources;
    /// playlists use M2M membership.
    fn get_collection_tracks(&self, collection_id: &CollectionId) -> Result<Vec<AudioSource>>;

    /// Playlist entries in display order. Errors if not a playlist.
    fn list_playlist_entries(&self, collection_id: &CollectionId) -> Result<Vec<CollectionEntry>>;
}

/// Mutable library manager operations.
pub trait WritableLibrary: Library {
    /// Re-read tags and/or run DSP analysis for a track and update the pool.
    ///
    /// When [`AnalyzeTrackOptions::force`] is false, file tags are kept for BPM/key
    /// when present; analysis fills missing fields only. When `force` is true,
    /// analysis results override tag values.
    fn analyze_track(&mut self, id: &TrackId, options: AnalyzeTrackOptions) -> Result<AudioSource>;

    /// Add a collection (folder or playlist).
    fn add_collection(&mut self, collection: &NewCollection) -> Result<Collection>;

    /// Sync one collection with its source, or all collections when `collection_id` is `None`.
    ///
    /// Folders rescan their disk path into the track pool. Playlists refresh
    /// metadata for member tracks from disk when files are present.
    fn sync_collection(&mut self, collection_id: Option<&CollectionId>) -> Result<ScanReport>;

    /// Update collection fields (name, sortable, …).
    fn update_collection(&mut self, id: &CollectionId, update: &UpdateCollection) -> Result<()>;

    /// Delete a collection (folder or playlist). Tracks in the pool are kept.
    fn delete_collection(&mut self, id: &CollectionId) -> Result<()>;

    /// Add a track to a playlist collection. Errors if not a Playlist.
    fn add_collection_entry(
        &mut self,
        collection_id: &CollectionId,
        track_id: &TrackId,
        position: Option<i32>,
    ) -> Result<CollectionEntryId>;

    /// Remove one playlist entry. Errors if not a Playlist.
    /// Drops the track from the pool when it is no longer linked to any collection.
    fn remove_collection_entry(
        &mut self,
        collection_id: &CollectionId,
        entry_id: &CollectionEntryId,
    ) -> Result<()>;

    /// Reorder playlist entries in a sortable playlist. Errors if not sortable.
    fn update_collection_entries(
        &mut self,
        collection_id: &CollectionId,
        entry_ids: &[CollectionEntryId],
    ) -> Result<()>;
}
