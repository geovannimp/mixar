# Mixxx Library Adapter — Design

**Date:** 2026-10-08
**Status:** Approved
**Scope:** First third-party library adapter: read a Mixxx `mixxxdb.sqlite` and
import its tracks, playlists, crates, and watched directories into the user's one
canonical `library::LibraryManager`, with a Settings → Library entry point in the
Flutter desktop UI.

## 1. Goal and success criteria

A Mixxx user can, from inside Mixar, point at their Mixxx database and bring their
DJ library across into Mixar's canonical library — no parallel manager, no coupling
of the Mixxx schema to `library`.

Success criteria:

- `library-adapters` exposes a read-only `Library` view over a Mixxx
  `mixxxdb.sqlite` and a migration that copies it into a `WritableLibrary`.
- Mapping follows tech-spec §10.4: watched dir → `Folder`, playlist →
  `Playlist { sortable: true }`, crate → `Playlist { sortable: false }`, tracks →
  the track pool. Hidden Mixxx playlists (AutoDJ queue, history) are skipped.
- Metadata comes from the Mixxx `library` row; tracks whose file is currently
  missing are still imported (path recorded, playback unavailable until present).
- Settings → Library has an "Import from Mixxx library…" action that previews
  counts, confirms, imports, reports the result, and refreshes the library UI.
- The whole path is covered by tests; `library` does not depend on
  `library-adapters`.

### Non-goals (this adapter)

- Cue points, saved loops, beat grids, and Mixxx `beats`/`keys` blobs
  (tech-spec §10 step 2, "DJ metadata on `Track`").
- Mixxx `comment`, `rating`, `color`, `year`, `tracknumber`, `composer`,
  `grouping` (no `TrackMetadata` fields yet).
- Write-back / export to Mixxx.
- Watched-directory auto-sync after import (Mixar folders scan on demand).

## 2. Architecture and data flow

```text
Settings → Library  [ Import from Mixxx library… ]
        │  (LibraryTransport FRB calls — the only allowed I/O path)
        ▼
host-flutter::api::library
   ├─ mixxx_default_database_path()                  -> Option<String>
   ├─ mixxx_import_preview(db_path)                  -> MixxxImportPreview
   └─ LibraryTransport::import_mixxx_library(db_path) -> MixxxImportReport
        │
        ├─ library_adapters::mixxx::MixxxLibrary::open(path)   (read-only SQLite)
        │     implements library_core::Library + Migratable
        └─ borrows the same Arc<Mutex<LibraryManager>> the transport already owns
              └─ migrates into the user's one canonical library
```

- `library-adapters` depends on `library-core` only. Migration targets the
  `WritableLibrary` trait (`&mut dyn WritableLibrary`), not the concrete manager.
  This keeps proprietary-format parsers out of the manager schema and preserves
  the dependency direction: `library` never depends on `library-adapters`.
- The host owns the one `LibraryManager`; the adapter is a short-lived reader used
  inside a host call.

## 3. `library-core` changes

### 3.1 `WritableLibrary::import_track`

Today `WritableLibrary` has no way to insert a track into the pool; the only
insert path is folder sync / file import, both of which require an existing file.
Because the chosen metadata policy imports Mixxx rows even when the file is
missing, the trait gains an explicit insert:

```rust
/// Upsert a file track from explicit metadata.
///
/// Unlike a folder scan, the path need not exist on disk: imported libraries may
/// reference files that are currently moved or absent. Such tracks are listed
/// and selectable; playback fails until the file returns at the path.
fn import_track(&mut self, path: &Path, metadata: &TrackMetadata) -> Result<AudioSource>;
```

`LibraryManager` implements it by exposing the existing private
`upsert_file_source`, whose `normalize_path` already tolerates non-existent paths
(canonicalizes when present, returns as-is otherwise).

### 3.2 `Migratable` trait and migration types

```rust
/// Copy an external library into a target (the user's one library).
pub trait Migratable: Library {
    fn migrate(
        &self,
        target: &mut dyn WritableLibrary,
        options: &MigrateOptions,
    ) -> Result<MigrateReport>;
}

pub struct MigrateOptions {
    pub include_folders: bool,
    pub include_playlists: bool,
    pub include_crates: bool,
    /// Skip source lists whose (name, sortable) already exists in the target.
    /// Makes re-running an import idempotent instead of duplicating lists.
    pub skip_existing_lists: bool,
}

impl Default for MigrateOptions { /* all true */ }

pub struct MigrateReport {
    pub tracks_added: usize,
    pub tracks_updated: usize,
    pub tracks_missing_files: usize,
    pub folders_imported: usize,
    pub playlists_imported: usize,
    pub crates_imported: usize,
    pub collections_skipped: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}
```

Both types live in `library-core::types` and are re-exported from the crate root.
`WritableLibrary` methods are object-safe, so `&mut dyn WritableLibrary` is valid.

## 4. `library-adapters` — `mixxx` module

New dependency: `rusqlite` (bundled, workspace version), behind a default-on
`mixxx` feature.

```text
crates/library-adapters/
  Cargo.toml              + rusqlite (bundled); [features] mixxx (default)
  src/lib.rs              pub mod mixxx (cfg feature)
  src/mixxx/mod.rs        MixxxLibrary, open(), default_database_path(), migrate
  src/mixxx/schema.rs     schema detection, row structs, key normalization
  src/mixxx/convert.rs    Mixxx row -> TrackMetadata / Collection mapping
```

### 4.1 Opening and schema detection

- `MixxxLibrary::open(path)` opens with
  `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX` and `PRAGMA query_only = 1`.
  The adapter never writes to the Mixxx database.
- `default_database_path()` returns the first existing `mixxxdb.sqlite` among the
  platform settings directories — Linux native `~/.mixxx`, Flatpak
  `~/.var/app/org.mixxx.Mixxx/.mixxx`, Snap
  `~/snap/mixxx/{current,common}/.mixxx`, `$XDG_DATA_HOME/mixxx`,
  `~/.local/share/mixxx`; macOS `~/Library/Application Support/Mixxx`; Windows
  `%LOCALAPPDATA%\Mixxx`. `MIXXX_DATABASE` (file) and `MIXXX_SETTINGS_PATH` /
  `MIXXX_SETTINGS_DIR` (directory) override the search.
- `PRAGMA table_info(library)` is read once. The `location` column type decides
  the join:
  - modern (schema revision ≥ 3): `library.location` is an integer FK →
    `track_locations.id`;
  - legacy: `library.location` is the text path → join `track_locations.location`.
  - If neither yields a resolvable path, return
    `LibraryError::Unsupported("mixxx schema")`.
- Optional columns (`key`, `key_id`, `album_artist`, `rating`, `color`,
  `composer`, `grouping`, `replaygain`, `sample_rate`, `channels`, `bitrate`,
  `duration`, `bpm`, `genre`, `album`, `artist`, `title`) are detected and only
  selected when present, so older databases still import.

### 4.2 Collection mapping

| Mixxx | Mixar | Id |
| --- | --- | --- |
| `directories.directory` (existing dirs only) | `Folder { scan_folder_tree: true }` | `mixxx:folder:<path>` |
| `Playlists` where `hidden = 0` | `Playlist { sortable: true }` | `mixxx:playlist:<id>` |
| `crates` | `Playlist { sortable: false }` | `mixxx:crate:<id>` |

Hidden playlists (`hidden = 1` AutoDJ queue, `hidden = 2` history) are skipped.
Ids are namespaced `mixxx:*` so an adapter collection can never collide with a
native collection id.

### 4.3 Track mapping

Path comes from `track_locations.location`. Metadata:

| Mixar `TrackMetadata` | Mixxx source |
| --- | --- |
| `title`, `artist`, `album`, `genre` | direct |
| `bpm` | `library.bpm`; `<= 0` (Mixxx "undefined") → `None` |
| `key` | `library.key` normalized (see below) |
| `duration_ms` | `round(library.duration * 1000)` |
| `sample_rate`, `channels`, `bitrate_kbps` | direct |
| `replaygain_track_gain_db` | `20 * log10(library.replaygain)` when `> 0` |

Key normalization is best-effort: try `library_core::camelot_to_musical` (handles
`8A` / `12b`); if that fails, keep the trimmed text (already musical, e.g. `Am`).
Open Key notation passes through untouched. Mixxx stores the key in the user's
configured notation; this is documented as lossy.

### 4.4 `Library` implementation

- `name()` → `"mixxx"`.
- `get_track(id)` parses `mixxx:track:<library_id>`.
- `list_collections()` returns folders + visible playlists + crates.
- `list_collection_tracks(id)`:
  - playlist → `PlaylistTracks` joined to `library`, ordered by `position`;
  - crate → `crate_tracks` joined to `library`;
  - folder → path-prefix match over the library's track paths.
- `list_collection_entries(id)` returns synthesized `CollectionEntry` rows
  (`position` for playlists, `None` for crates); errors for folders
  (`WrongCollectionType`).

### 4.5 `migrate`

1. Load the full Mixxx track index once (`library_id → (path, metadata)`).
2. Optionally import folders: for each existing watched directory,
   `target.add_collection(NewCollection::folder(path))`, then import the tracks
   under that path. Missing directories are skipped and counted.
3. For each visible playlist (sortable) / crate (unsortable):
   - If `skip_existing_lists` and a target collection with the same trimmed name
     and same `sortable` exists — including one created earlier in this run
     (in-run set seeded from the target) — skip it (`collections_skipped`).
   - Otherwise `add_collection`, then `add_collection_entry` per member in order
     (position for playlists, `None` for crates).
4. Tracks absent from disk are still imported; `tracks_missing_files` counts them.
5. Per-item errors are pushed to `errors` and `failed`, never aborting the run.
6. `import_track` upserts by path, so a track shared by several lists is imported
   once and referenced by each.

`import_track` returns the target `AudioSource`; its `TrackId` is used for
`add_collection_entry`. Crate membership is set-like (manager upserts), sortable
playlists keep duplicates.

## 5. host-flutter

- Add `library-adapters` to `crates/host-flutter/Cargo.toml`.
- New FRB DTOs and methods in `crates/host-flutter/src/api/library.rs`:

```rust
pub struct MixxxImportPreview {
    pub db_path: String,
    pub track_count: u32,
    pub missing_file_count: u32,
    pub playlist_count: u32,
    pub crate_count: u32,
    pub folder_count: u32,
}

pub struct MixxxImportReport {
    pub tracks_added: u32,
    pub tracks_updated: u32,
    pub tracks_missing_files: u32,
    pub folders_imported: u32,
    pub playlists_imported: u32,
    pub crates_imported: u32,
    pub collections_skipped: u32,
    pub failed: u32,
    pub errors: Vec<String>,
}

/// Default Mixxx database location for the current OS, if it exists.
pub fn mixxx_default_database_path() -> Option<String>;

/// Read-only summary used to confirm an import.
pub fn mixxx_import_preview(db_path: String) -> Result<MixxxImportPreview, String>;

impl LibraryTransport {
    /// Import the Mixxx library at `db_path` into this transport's manager.
    pub fn import_mixxx_library(&self, db_path: String) -> Result<MixxxImportReport, String>;
}
```

- `import_mixxx_library` opens `MixxxLibrary`, reads everything it needs outside
  the manager lock, then briefly locks the existing `Arc<Mutex<LibraryManager>>`
  and calls `migrate`. The FRB call runs on a worker thread, so the Flutter UI
  stays responsive. No library bus/worker changes are needed for this MVP (no
  per-phase progress events).
- Regenerate the bridge with `moon run gui-flutter:generate`.

## 6. Flutter UI

- New `apps/gui-flutter/lib/settings/mixxx_import_panel.dart`
  (`ConsumerStatefulWidget`, modeled on `SettingsStoragePanel`): a
  `SettingsPanel` with a title, explanation, a status line, a primary
  **"Import from Mixxx library…"** button, and a busy state.
  The Mixxx database is auto-detected via `mixxx_default_database_path`;
  **the button is disabled until one is found** (no file picker). Flow:
  1. A `mixxxDatabasePathProvider` resolves the host's default DB path
     (errors degrade to "not found").
  2. `mixxxImportPreview` and `showMixarConfirm` showing track / playlist / crate
     / missing-file counts, noting that missing files import as unavailable.
  3. `importMixxxLibrary`, then `ref.invalidate(collectionsProvider)` and
     `ref.invalidate(collectionTracksProvider)`.
  4. `showMixarToast` with a summary, or a destructive toast on failure.
- Insert `const MixxxImportPanel()` into `settings_library_panel.dart`.

Widgets never call raw host invoke/listen; all I/O goes through
`LibraryTransport` (per the repo bridge rule).

## 7. Testing

- `crates/library-adapters/tests/mixxx_import.rs` — build a synthetic
  `mixxxdb.sqlite` with rusqlite: tracks (one with a Camelot key, one whose file
  is absent), a visible playlist, a crate, a watched directory, and a hidden
  AutoDJ playlist. Assert:
  - collection listing, track order, and `list_collection_entries`;
  - `8A` → `Am`;
  - `migrate` into `LibraryManager::open_in_memory()` imports all tracks
    (including the missing one), a sortable playlist with ordered entries, a
    crate, and a folder; the hidden playlist is absent; counts are correct;
  - re-running `migrate` with `skip_existing_lists` adds no duplicates.
- `crates/library/src/lib.rs` unit test — `import_track` inserts a track whose
  path does not exist.
- `crates/host-flutter/tests/mixxx_import.rs` — fixture DB, then
  `LibraryTransport::open_in_memory()` + `import_mixxx_library`, asserting the
  report and `list_collections`.
- `apps/gui-flutter/test/mixxx_import_panel_test.dart` — panel renders the button
  and labels with `libraryTransportProvider` overridden.
- CI: `cargo fmt`, `clippy -D warnings`, `flutter analyze`, `flutter test`.

## 8. Documented lossiness

- Not imported: Mixxx `comment`, `rating`, `color`, `year`, `tracknumber`,
  `composer`, `grouping`; cue points, saved loops, beat grids, and
  `beats`/`keys` blobs.
- Hidden playlists (AutoDJ queue and history) are skipped.
- Key normalization is best-effort; Open Key notation passes through as text.
- Missing files import as unavailable tracks (path recorded).
- Watched directories that no longer exist are skipped (a `Folder` requires a
  real directory in the manager).
- Re-running an import is idempotent at the list level (`skip_existing_lists`);
  tracks upsert by path and folder collections dedupe by path.
