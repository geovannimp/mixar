# library-adapters

Third-party DJ library adapters for [Mixar](https://github.com/geovannimp/mixar).

Each adapter implements [`library_core::Library`] (read-only browse) and
[`library_core::Migratable`] (copy into the user's library) so an external DJ
application's library can be previewed and imported into Mixar's one canonical
`library::LibraryManager`. Exporters implement [`library_core::Exportable`] and
write the manager's collections out to a foreign layout. Adapters depend on
`library-core` only; `library` never depends on this crate.

Implemented adapters:

- `mixxx` — read a Mixxx `mixxxdb.sqlite` and import its tracks, playlists,
  crates, and watched directories.
- `cdj` — export the user's library into the Pioneer CDJ/XDJ USB layout
  (DeviceSQL `export.pdb` plus ANLZ analysis files).

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

Collection ⋮ menu  [ Export to CDJ USB… ]
        │  (LibraryTransport FRB calls)
        ▼
host-flutter::api::library
   └─ LibraryTransport::export_cdj_usb(destination, playlists, folders, refresh)
        │
        ├─ library_adapters::CdjExporter (implements library_core::Exportable)
        │     reads through library::LockedExportSource, which locks the
        │     manager per call so browsing stays responsive
        └─ writes Contents/, PIONEER/rekordbox/export.pdb, PIONEER/USBANLZ/…
```

Both directions go through traits: migration targets `&mut dyn WritableLibrary`,
export reads `&dyn ExportSource`, so format parsing never reaches into the
manager's schema.

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

## CDJ USB export

`library_adapters::CdjExporter` implements `library_core::Exportable`: it reads
the user's library through `library_core::ExportSource` and writes the layout
Rekordbox puts on a USB drive, so CDJ/XDJ players browse and play the library
without Rekordbox.

```text
<destination>/
  Contents/<Artist>/<Album>/<file>                 audio (folder collections mirror their tree)
  PIONEER/rekordbox/export.pdb                     DeviceSQL database: tracks + playlists
  PIONEER/USBANLZ/P###/########/ANLZ0000.DAT       beat grid, preview waveforms, cue lists
  PIONEER/USBANLZ/P###/########/ANLZ0000.EXT       detail waveforms, extended cue lists
```

The database is the legacy DeviceSQL format every CDJ/XDJ generation reads. The
analysis directory is derived from the on-drive audio path with the algorithm
the hardware itself uses, so a track's analysis files must sit at exactly that
hash path (`CdjExporter::analysis_dir`) — otherwise the player ignores them and
re-analyzes the track on load.

User guide: [Export to a CDJ USB drive](https://mixar.top/docs/users/library/export-to-cdj-usb);
internals: [CDJ USB export](https://mixar.top/docs/developers/library/cdj-usb-export).

| Module | Role |
| --- | --- |
| `cdj/mod.rs` | `CdjExporter`: the `Exportable` run loop, file copying, cover thumbnails, report |
| `cdj/plan.rs` | selection → tracks, placements and playlists |
| `cdj/layout.rs` | FAT32-safe, de-duplicated on-drive paths |
| `cdj/pdb.rs` | `export.pdb` writer (pages, tables, rows) |
| `cdj/anlz.rs` | `ANLZ0000.DAT` / `.EXT` writers |
| `cdj/waveform.rs` | Mixar waveform tiers → Pioneer waveform encodings |
| `cdj/device_sql.rs`, `cdj/keys.rs`, `cdj/path_hash.rs` | DeviceSQL strings, key table, path hash |

### Selection and placement

- `ExportOptions::playlists` lists the playlists to write, in the caller's
  order; each becomes a node in the device playlist tree with its entries in
  play order.
- `ExportOptions::folders` lists folder collections; each is mirrored as a real
  directory tree under `Contents/<folder name>/…` **and** gets a playlist of the
  same name, because the player's playlist browser cannot see the file tree.
- A track reachable through a selected folder keeps its mirrored path;
  everything else is placed as `Contents/<artist>/<album>/<file name>`. Names
  are sanitized and de-duplicated case-insensitively, so every track is copied
  once and playlist entries point at the single copy.
- `ExportOptions::refresh_files` re-copies audio and rewrites analysis files
  even when files of the same size (audio) or the files themselves (analysis)
  are already on the drive. By default they are reused, which makes re-exports
  cheap; reuse for audio is a name + size check, not a content hash.

### Analysis

The exporter never runs DSP itself: `library::LibraryManager` implements
`ExportSource::export_data`, handing over the beat grid, cue points and two
waveform tiers. The detail tier the players want is sampled at 150 entries per
second, which is finer than what the library stores for its own displays, so an
export decodes each exported track once to build it. Tracks with no analysis
data are still registered in the database and reported as a warning; the player
analyzes them on first load.

### Fidelity

Written: title, artist, album, genre, BPM, key, duration, bitrate, sample rate,
bit depth, file type, date added, playlists, beat grid, preview and detail
waveforms, hot cues, memory cues and saved loops (with labels and colors), and
one cover thumbnail per distinct artwork (`PIONEER/Artwork/<id>/a1.jpg`, scaled
to the 80 px Rekordbox writes; tracks reference it by artwork id, so an album's
cover is stored once).

Dropped or approximated:

- the higher-resolution `_m` cover variant newer Rekordbox releases write beside the primary
  thumbnail (the device database's single path is the 80 px one);
- rating, release year, track/disc number, comments, color labels — Mixar's
  `TrackMetadata` has no fields for them;
- the OneLibrary format (`exportLibrary.db`) and the `.2EX` 3-band waveform
  files newer players prefer; those units still read the legacy format, they
  just draw lower-resolution waveforms;
- device settings files (`MYSETTING.DAT`, …), which belong to the player;
- history playlists, My Tags, stems and sampler banks;
- beat grids are written from Mixar's grid (constant tempo or analyzed beats)
  with a numerical tempo per beat; non-quantized loop lengths are written as
  loop points without a numerator/denominator;
- extended cue entries use Rekordbox's entry size (88 bytes plus the comment)
  so the layout stays byte-compatible, but the undocumented bytes it fills
  there (a 20-byte gap plus a 20-byte tail, meaning unknown to every public
  source) are written zeroed;
- long paths are truncated per component (100 characters) and de-duplicated,
  which can differ from the source file names.

### Provenance

The formats are undocumented; the byte layouts here come from public
reverse-engineering work, credited in the module docs:

- Deep Symmetry's *DJ Link Ecosystem Analysis* (PDB and ANLZ references) and the
  Kaitai Struct specs in `crate-digger`;
- the *fourfour* `pioneer-usb-writer` reverse-engineering notes (CDJ-3000
  firmware 3.19): path-hash algorithm, `PPTH` null terminator, `PQTZ` header
  size, Columns/History table requirements;
- the GPL-3.0 `sustain-pioneer` crate (XDJ-XZ), which this GPL-3.0 project's
  writer is modelled on for page allocation and the fixed tables;
- `rekordcrate` (MPL-2.0), used as a dev-dependency to verify what we write and
  as a model for the format's structure.

Verification is by re-parsing our own output with `rekordcrate` plus
byte-level unit tests. Physical hardware validation (CDJ/XDJ) has **not** been
performed for this implementation.

## Host + UI wiring

`host-flutter` exposes the Mixxx entry points above on the Rust worker thread
(FRB `Future`-based, so the Flutter isolate is not blocked), regenerated with
`moon run gui-flutter:generate`. The Mixxx database is fully read before the
manager lock is taken; only the write phase runs under it.

The Flutter UI (`apps/gui-flutter/lib/widgets/settings/mixxx_import_panel.dart`)
adds an "Import from Mixxx library…" action to Settings → Library: it
auto-detects the database (button disabled until one is found), previews counts,
confirms, imports, invalidates the library providers, and reports the result — a
destructive toast when any item failed. Widgets never call raw host
invoke/listen; all I/O goes through `LibraryTransport`.

The export side is
`LibraryTransport::export_cdj_usb(destination, playlists, folders, refresh)`: the
destination is a mounted drive or any folder, the collection ids come from the
collection the user acted on, and the returned report carries tracks
exported/reused, playlists, folders, bytes copied, warnings and errors. It reads
through `library::LockedExportSource`, which locks the manager per call, so
browsing stays responsive while a long export runs. The entry point is the
collection's three-dot menu
(`apps/gui-flutter/lib/widgets/library/collection_actions_menu.dart`, alongside
"Analyze tracks…"), which opens
`apps/gui-flutter/lib/widgets/library/export_cdj_usb_dialog.dart`: the destination
comes from the platform's own directory picker, the dialog carries the **Override
files already on the drive** checkbox (`refresh_files`), and it summarises the
report in a toast (warning or error toast when it carries any).

## Testing

- `crates/library-adapters/tests/mixxx_import.rs` — a synthetic `mixxxdb.sqlite`
  (Camelot key, missing file, playlist, crate, watched dir, hidden AutoDJ
  playlist) exercising the reader, key normalization, migration (incl. the
  missing file and the catch-all), idempotent re-imports, unnamed-list dedupe,
  and the legacy text-`location` schema.
- `crates/library-adapters/tests/cdj_export.rs` — exports a small library (two
  WAVs, one carrying an embedded cover, a folder collection, a playlist, a beat
  grid, a hot cue and a saved loop) into a temp "drive" and reads the database
  and analysis files back with `rekordcrate`, asserting the cover thumbnail's
  size and its artwork row, plus file-reuse, missing-file, empty-selection and a
  40-track case whose tables span several pages.
- `crates/codec` / `crates/library` — thumbnail scaling (aspect, no upscaling,
  alpha over white, unsupported formats skipped) and the UI/export edge sizes.
- `crates/library/tests` — `import_track` inserts a non-existent-file track.
- `crates/host-flutter/tests/mixxx_import.rs` — end-to-end import through
  `LibraryTransport`; `crates/host-flutter/tests/cdj_export.rs` does the same for
  an export.
- `apps/gui-flutter/test/mixxx_import_panel_test.dart` — panel states and summary
  formatting; `apps/gui-flutter/test/export_cdj_usb_dialog_test.dart` covers the
  export dialog (native picker seam, per-kind request, Override checkbox, toasts)
  and the summary line.

CI gates: `cargo fmt`, `clippy -D warnings`, `flutter analyze`, `flutter test`.

## Mixxx adapter — known lossiness (deferred)

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
[`library_core::Exportable`]: ../library-core/src/export.rs