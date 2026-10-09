# library-adapters

Third-party DJ library adapters for [Mixar](https://github.com/geovannimp/mixar).

Each adapter implements [`library_core::Library`] (read-only browse) and
[`library_core::Migratable`] (copy into the user's library) so an external DJ
application's library can be previewed and imported into Mixar's one canonical
`library::LibraryManager`. Adapters depend on `library-core` only; `library`
never depends on this crate.

Implemented adapters:

- `mixxx` — read a Mixxx `mixxxdb.sqlite` and import its tracks, playlists,
  crates, and watched directories.

## Architecture

```text
Settings → Library  [ Import from Mixxx library… ]
        │  (LibraryTransport FRB calls — the only allowed I/O path)
        ▼
host-flutter::api::library
   ├─ mixxx_default_database_path()                   -> Option<String>
   ├─ mixxx_import_preview(db_path)                   -> MixxxImportPreview
   └─ LibraryTransport::import_mixxx_library(db_path) -> MixxxImportReport
        │
        ├─ library_adapters::mixxx::MixxxLibrary::open(path)   (read-only SQLite)
        │     implements library_core::Library + Migratable
        └─ borrows the same Arc<Mutex<LibraryManager>> the transport already owns
              └─ migrates into the user's one canonical library
```

The migration targets the `WritableLibrary` trait (`&mut dyn WritableLibrary`),
never the concrete manager, so proprietary-format parsing stays out of the
manager schema.

## Mixxx adapter

Supporting traits and types live in `library-core`: `WritableLibrary::import_track`
(upsert a file track from explicit metadata; the path need not exist on disk —
such tracks are listed and selectable but not playable until the file returns)
returns `ImportedTrack { source, created }`, and `Migratable::migrate` copies
into a target using `MigrateOptions` / `MigrateReport`.

### Database detection

`MixxxLibrary::open(path)` opens read-only
(`SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`, `PRAGMA query_only = 1`) and
never writes to the Mixxx database.

`mixxx_default_database_path()` returns the first existing `mixxxdb.sqlite`
among the platform settings directories:

- Linux: `~/.mixxx`, Flatpak `~/.var/app/org.mixxx.Mixxx/.mixxx`, Snap
  `~/snap/mixxx/{current,common}/.mixxx`, `$XDG_DATA_HOME/mixxx`,
  `~/.local/share/mixxx`
- macOS: `~/Library/Application Support/Mixxx`
- Windows: `%LOCALAPPDATA%\Mixxx`

`MIXXX_DATABASE` (a file) and `MIXXX_SETTINGS_PATH` / `MIXXX_SETTINGS_DIR`
(a directory) override the search.

### Schema handling

`PRAGMA table_info(library)` is read once; the `location` column type decides
the join:

- modern (revision ≥ 3): `library.location` is an integer FK →
  `track_locations.id`;
- legacy: `library.location` is the text path → join `track_locations.location`.

A database that is not a Mixxx library (or lacks `location`) returns
`LibraryError::Unsupported`. Optional metadata columns are detected and only
selected when present (`NULL AS <name>` otherwise), so older databases still
import — `METADATA_COLUMNS` is the single list both the `SELECT` and the row
reader derive from.

### Collection mapping

| Mixxx | Mixar | Id |
| --- | --- | --- |
| `directories.directory` (existing dirs only) | `Folder { scan_folder_tree: true }` | `mixxx:folder:<path>` |
| `Playlists` where `hidden = 0` | `Playlist { sortable: true }` | `mixxx:playlist:<id>` |
| `crates` | `Playlist { sortable: false }` | `mixxx:crate:<id>` |

Hidden playlists (`hidden = 1` AutoDJ queue, `hidden = 2` history) are skipped.
Ids are namespaced `mixxx:*` so they cannot collide with native collection ids.

### Track metadata

Path comes from `track_locations.location`. Mapping:

| Mixar `TrackMetadata` | Mixxx source |
| --- | --- |
| `title`, `artist`, `album`, `genre` | direct |
| `bpm` | `library.bpm`; `<= 0` (or non-finite) → `None` |
| `key` | `library.key` normalized (see below) |
| `duration_ms` | `round(library.duration * 1000)`; non-finite → `None`, clamped to `i32::MAX` |
| `sample_rate`, `channels`, `bitrate_kbps` | direct (clamped to target width) |
| `replaygain_track_gain_db` | `20 * log10(library.replaygain)`; `1.0` (unset) or non-finite → `None` |

Key normalization is best-effort: `library_core::camelot_to_musical` first
(`8A` → `Am`), otherwise the trimmed text is kept; Open Key notation passes
through. Mixxx stores the key in the user's configured notation.

### Migration

1. Import every loaded track once (`import_track`), remembering its target id;
   absent files are still imported and counted in `tracks_missing_files`.
2. Optionally register watched directories as `Folder` collections; missing
   directories are failures.
3. **Catch-all:** ensure an unsortable playlist named `Mixxx` exists and add
   every imported track to it (set membership upserts, so re-imports merge new
   tracks). Because tracks are only reachable through a collection, this
   guarantees nothing is orphaned. It is created even when folders, playlists,
   or crates are excluded, and is not counted in `playlists_imported` /
   `crates_imported` (those report source lists).
4. For each visible playlist (sortable) / crate (unsortable): skip when
   `skip_existing_lists` and a collection with the same trimmed name and
   `sortable` already exists, else create it and add members in order. Blank
   names never dedupe, so unnamed lists all import.
5. Per-item errors go to `errors` / `failed` (basename-only via
   `library_core::path_label`); the run never aborts mid-way.

### Library core reads

`name()` is `"mixxx"`; `get_track` parses `mixxx:track:<library_id>`;
`list_collection_tracks` resolves playlists by `PlaylistTracks` order, crates by
`crate_tracks`, and folders by path prefix; `list_collection_entries` returns
synthesized rows (dropping members whose track was deleted from the Mixxx
library), or `WrongCollectionType` for folders.

## Host + UI wiring

`host-flutter` exposes the three FRB entry points above on the Rust worker
thread (FRB `Future`-based, so the Flutter isolate is not blocked), regenerated
with `moon run gui-flutter:generate`. The Mixxx database is fully read before
the manager lock is taken; only the write phase runs under it.

The Flutter UI (`apps/gui-flutter/lib/settings/mixxx_import_panel.dart`) adds an
"Import from Mixxx library…" action to Settings → Library: it auto-detects the
database (button disabled until one is found), previews counts, confirms,
imports, invalidates the library providers, and reports the result — a
destructive toast when any item failed. Widgets never call raw host
invoke/listen; all I/O goes through `LibraryTransport`.

## Testing

- `crates/library-adapters/tests/mixxx_import.rs` — a synthetic `mixxxdb.sqlite`
  (Camelot key, missing file, playlist, crate, watched dir, hidden AutoDJ
  playlist) exercising the reader, key normalization, migration (incl. the
  missing file and the catch-all), idempotent re-imports, unnamed-list dedupe,
  and the legacy text-`location` schema.
- `crates/library/tests` — `import_track` inserts a non-existent-file track.
- `crates/host-flutter/tests/mixxx_import.rs` — end-to-end import through
  `LibraryTransport`.
- `apps/gui-flutter/test/mixxx_import_panel_test.dart` — panel states and
  summary formatting.

CI gates: `cargo fmt`, `clippy -D warnings`, `flutter analyze`, `flutter test`.

## Known lossiness (deferred)

- Not imported: Mixxx `comment`, `rating`, `color`, `year`, `tracknumber`,
  `composer`, `grouping`; cue points, saved loops, beat grids, and
  `beats`/`keys` blobs.
- Hidden playlists (AutoDJ queue, history) are skipped.
- Key normalization is best-effort; Open Key notation passes through as text.
- Missing files import as unavailable tracks (path recorded).
- Watched directories that no longer exist are skipped.
- Re-imports are idempotent at the list level; tracks upsert by path and folder
  collections dedupe by path; the catch-all merges new tracks.
- The migration writes per-item without an enclosing transaction, and runs its
  write phase under the transport's manager lock — a batched/transactional
  import API on `library-core` is a follow-up.

[`library_core::Library`]: ../library-core/src/traits.rs
[`library_core::Migratable`]: ../library-core/src/traits.rs