# Mixxx Library Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import a Mixxx `mixxxdb.sqlite` (tracks, playlists, crates, watched dirs) into the user's one canonical Mixar library, driven from Settings → Library.

**Architecture:** `library-adapters::mixxx` reads the Mixxx DB read-only and implements `library_core::Library` + a new `Migratable` trait; migration targets `&mut dyn WritableLibrary`. `library-core` gains `WritableLibrary::import_track` (insert explicit metadata, missing files allowed). `host-flutter` exposes FRB preview/import methods on `LibraryTransport`; Flutter adds a Settings → Library import panel.

**Tech Stack:** Rust (rusqlite bundled, sea-orm-backed `library`), flutter_rust_bridge 2.13, Flutter/Riverpod.

**Spec:** `docs/mixxx-library-adapter-design.md`

## Global Constraints

- Workspace root is `crates/Cargo.toml`; run cargo via `cargo --manifest-path crates/Cargo.toml …`.
- `library` must never depend on `library-adapters`. Adapters depend on `library-core` only.
- `rusqlite` version `0.38` with `features = ["bundled"]`.
- UI widgets must not call raw host invoke/listen; all library I/O goes through `LibraryTransport`.
- FRB regen: `moon run gui-flutter:generate`.
- Pre-commit hook runs rustfmt/clippy, `dart format`, `flutter analyze`; do not bypass.
- Mapping per tech-spec §10.4: watched dir → `Folder`, playlist → `Playlist{sortable:true}`, crate → `Playlist{sortable:false}`, hidden Mixxx playlists skipped.

## Review Focus

1. Missing-file tracks: a Mixxx row whose file is absent must import (path recorded, listed, selectable) and not crash import.
2. Legacy Mixxx schema: `library.location` as a text path (pre-revision-3) must still yield track paths.
3. Hidden playlists (`hidden` 1/2) must never appear as user playlists.
4. Re-running the import must not duplicate playlists/crates or tracks.
5. Key normalization: a Camelot key (`8A`) must become musical (`Am`); already-musical keys pass through unchanged.

---

### Task 1: `library-core` migration traits + `WritableLibrary::import_track`

**Files:**
- Modify: `crates/library-core/src/types.rs`
- Modify: `crates/library-core/src/traits.rs`
- Modify: `crates/library-core/src/lib.rs`
- Modify: `crates/library/src/lib.rs`

**Interfaces:**
- Produces:
  - `pub struct MigrateOptions { pub include_folders: bool, pub include_playlists: bool, pub include_crates: bool, pub skip_existing_lists: bool }` (Default all `true`).
  - `pub struct MigrateReport { pub tracks_added: usize, pub tracks_updated: usize, pub tracks_missing_files: usize, pub folders_imported: usize, pub playlists_imported: usize, pub crates_imported: usize, pub collections_skipped: usize, pub failed: usize, pub errors: Vec<String> }` (Default).
  - `pub struct ImportedTrack { pub source: AudioSource, pub created: bool }`.
  - `pub trait Migratable: Library { fn migrate(&self, target: &mut dyn WritableLibrary, options: &MigrateOptions) -> Result<MigrateReport>; }`.
  - `WritableLibrary::import_track(&mut self, path: &Path, metadata: &TrackMetadata) -> Result<ImportedTrack>`.

- [ ] **Step 1: Add types** to `crates/library-core/src/types.rs`: `MigrateOptions`, `MigrateReport`, `ImportedTrack` with `#[derive(Clone, Debug, Default, PartialEq)]` and a hand-written `Default` for `MigrateOptions` (all `true`). Re-export all three from `crates/library-core/src/lib.rs`.
- [ ] **Step 2: Add trait methods** in `crates/library-core/src/traits.rs`. Add `import_track` to `WritableLibrary`, and `Migratable` (needs `use crate::types::{MigrateOptions, MigrateReport};`). Re-export `Migratable` from `lib.rs`.
- [ ] **Step 3: Implement `import_track` for `LibraryManager`** in `crates/library/src/lib.rs`: `let id = TrackId::new(path.to_string_lossy()); let created = !self.store().track_exists(&id)?; let source = self.upsert_file_source(path, metadata)?; Ok(ImportedTrack { source, created })`. (Note: `upsert_file_source` normalizes; compute `created` against the normalized id by checking after normalization — use `normalize_path` then `track_id_for`.)
- [ ] **Step 4: Add a `library` unit test** in the existing `#[cfg(test)] mod tests` in `crates/library/src/lib.rs`: `import_track_inserts_missing_file_track` — import a path under a tempdir that does not exist, assert `created == true`, and `get_track` returns the source.
- [ ] **Step 5: Run** `cargo --manifest-path crates/Cargo.toml test -p library-core -p library` → PASS.
- [ ] **Step 6: Commit** with `feat(library-core): Migratable trait and metadata-track import`.

### Task 2: `library-adapters::mixxx` reader

**Files:**
- Modify: `crates/library-adapters/Cargo.toml`
- Modify: `crates/library-adapters/src/lib.rs`
- Create: `crates/library-adapters/src/mixxx/mod.rs`
- Create: `crates/library-adapters/src/mixxx/schema.rs`

**Interfaces:**
- Consumes: `library_core::{Library, TrackId, TrackMetadata, AudioSource, FileAudioSource, Collection, CollectionConfig, CollectionId, CollectionEntry, CollectionEntryId, CollectionType, LibraryError, Result}`.
- Produces:
  - `pub struct MixxxLibrary` implementing `Library`.
  - `pub fn MixxxLibrary::open(path: impl AsRef<Path>) -> Result<MixxxLibrary>`.
  - `pub fn default_database_path() -> Option<PathBuf>`.
  - `pub struct MixxxPreview { pub track_count, pub missing_file_count, pub playlist_count, pub crate_count, pub folder_count: usize }` + `pub fn preview(&self) -> MixxxPreview`.
  - `pub fn track_count/missing_file_count/playlist_count/crate_count/folder_count(&self) -> usize`.

- [ ] **Step 1: Write failing tests** in `crates/library-adapters/tests/mixxx_import.rs` (create the helper that builds a fixture DB — see Task 3 step 1; this task only needs the reader assertions): build a fixture, `MixxxLibrary::open`, then assert `name() == "mixxx"`, `list_collections()` contains the visible playlist (`sortable: true`), crate (`sortable: false`), watched folder, and NOT the hidden playlist; `list_collection_entries(playlist)` returns library ids in `position` order; a Camelot `"8A"` key track maps to `"Am"` via `get_track`.
- [ ] **Step 2: Run** `cargo --manifest-path crates/Cargo.toml test -p library-adapters` → FAIL (crate/module missing).
- [ ] **Step 3: Add dependencies**: `rusqlite = { version = "0.38", features = ["bundled"] }`, and `[features] default = ["mixxx"]; mixxx = ["dep:rusqlite"]`; in `src/lib.rs` add `#[cfg(feature = "mixxx")] pub mod mixxx;` and `#[cfg(feature = "mixxx")] pub use mixxx::{default_database_path, MixxxLibrary, MixxxPreview};`.
- [ ] **Step 4: Implement `schema.rs`**: detect `library.location` type via `PRAGMA table_info(library)` (`INTEGER` ⇒ modern FK join `track_locations.id`, otherwise legacy text join `track_locations.location`); collect present optional columns. Define `struct MixxxTrack { library_id: i64, path: PathBuf, metadata: TrackMetadata }`, `struct MixxxCollection { collection: Collection, members: Vec<i64> }`.
- [ ] **Step 5: Implement `mod.rs`**: `open` uses `OpenFlags::SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`, sets `PRAGMA query_only=1`, eagerly loads `tracks` (skipping `mixxx_deleted = 1`), visible `Playlists` (`hidden = 0`) + ordered `PlaylistTracks`, `crates` + `crate_tracks`, and `directories`. Implement `Library` over the loaded data; ids `mixxx:folder:<path>` / `mixxx:playlist:<id>` / `mixxx:crate:<id>`; `list_collection_entries` synthesizes rows; folders error with `WrongCollectionType`. Key: `camelot_to_musical(key).unwrap_or_else(|| key.trim().to_string())`; `bpm <= 0.0 → None`; `replaygain > 0 → Some(20*log10)`.
- [ ] **Step 6: Run** `cargo --manifest-path crates/Cargo.toml test -p library-adapters` → PASS.
- [ ] **Step 7: Commit** with `feat(library-adapters): read-only Mixxx library`.

### Task 3: `library-adapters` migration

**Files:**
- Create: `crates/library-adapters/src/mixxx/convert.rs`
- Modify: `crates/library-adapters/src/mixxx/mod.rs`
- Create/modify: `crates/library-adapters/tests/mixxx_import.rs`

**Interfaces:**
- Consumes: Task 1's `Migratable`, `MigrateOptions`, `MigrateReport`, `ImportedTrack`; Task 2's `MixxxLibrary`.
- Produces: `impl Migratable for MixxxLibrary`.

- [ ] **Step 1: Write failing tests** `migrate_imports_tracks_lists_and_missing_files`, `migrate_is_idempotent_for_lists`, `migrate_skips_hidden_playlists`: a `LibraryManager::open_in_memory()`, run `mixxx.migrate(&mut lib, &MigrateOptions::default())`, assert the report counts, that `lib.list_collections()` has the playlist/crate/folder, playlist entries are in order, the missing-file track is present with `created` counted, and a second `migrate` adds 0 playlists/crates/tracks but increments `tracks_updated`.
- [ ] **Step 2: Run** → FAIL (`migrate` missing).
- [ ] **Step 3: Implement `Migratable for MixxxLibrary`** in `mod.rs` (helpers in `convert.rs`): import every loaded Mixxx track via `target.import_track(&t.path, &t.metadata)`, tally `tracks_added`/`tracks_updated` from `created` and `tracks_missing_files` when `!t.path.is_file()`; remember target `TrackId` per library id. Then, when enabled, import folders (`target.add_collection(NewCollection::folder(path))`, skip non-existent dirs into `failed`), playlists, and crates; seed an existing `(name.trim(), sortable)` set from `target.list_collections()` for `skip_existing_lists` and skip duplicates. Push per-item errors to `errors`/`failed` without aborting.
- [ ] **Step 4: Run** `cargo --manifest-path crates/Cargo.toml test -p library-adapters` → PASS.
- [ ] **Step 5: Commit** with `feat(library-adapters): migrate Mixxx into the user library`.

### Task 4: host-flutter bridge

**Files:**
- Modify: `crates/host-flutter/Cargo.toml`
- Modify: `crates/host-flutter/src/api/library.rs`
- Modify (generated): `crates/host-flutter/src/frb_generated.rs`, `apps/gui-flutter/lib/src/rust/**`
- Create: `crates/host-flutter/tests/mixxx_import.rs`

**Interfaces:**
- Consumes: Tasks 1–3 (`MixxxLibrary`, `Migratable`, `MigrateOptions`).
- Produces (FRB): `MixxxImportPreview`, `MixxxImportReport`, `mixxx_default_database_path() -> Option<String>`, `mixxx_import_preview(String) -> Result<MixxxImportPreview, String>`, `LibraryTransport::import_mixxx_library(String) -> Result<MixxxImportReport, String>`.

- [ ] **Step 1: Write failing test** `crates/host-flutter/tests/mixxx_import.rs`: write a fixture Mixxx DB, `LibraryTransport::open_in_memory()`, call `import_mixxx_library(path)`, assert report counts and `transport.list_collections()` includes the playlist.
- [ ] **Step 2: Run** `cargo --manifest-path crates/Cargo.toml test -p host_flutter --test mixxx_import` → FAIL.
- [ ] **Step 3: Implement** the dep + DTOs + methods in `src/api/library.rs` (import `library_adapters`), mapping `MigrateReport` → `MixxxImportReport` and `MixxxPreview` → `MixxxImportPreview`.
- [ ] **Step 4: Regen bridge** `moon run gui-flutter:generate` (or `flutter_rust_bridge_codegen generate`).
- [ ] **Step 5: Run** the test → PASS; run `cargo --manifest-path crates/Cargo.toml build -p host_flutter`.
- [ ] **Step 6: Commit** with `feat(host-flutter): Mixxx import bridge`.

### Task 5: Flutter Settings → Library import panel

**Files:**
- Create: `apps/gui-flutter/lib/settings/mixxx_import_panel.dart`
- Modify: `apps/gui-flutter/lib/settings/settings_library_panel.dart`
- Create: `apps/gui-flutter/test/mixxx_import_panel_test.dart`

**Interfaces:**
- Consumes: generated `MixxxImportPreview`, `MixxxImportReport`, `mixxxDefaultDatabasePath`, `mixxxImportPreview`, `LibraryTransport.importMixxxLibrary`; providers `libraryTransportProvider`, `collectionsProvider`, `collectionTracksProvider`.

- [ ] **Step 1: Write failing widget test** `mixxx_import_panel_test.dart`: pump the panel inside `ProviderScope` with `libraryTransportProvider` overridden to `null` and assert the title, description, and "Import from Mixxx library…" button render.
- [ ] **Step 2: Run** `mise exec -- flutter test test/mixxx_import_panel_test.dart` → FAIL.
- [ ] **Step 3: Implement `MixxxImportPanel`** (`ConsumerStatefulWidget`): button → host default path, else `FilePicker.pickFiles(allowedExtensions: ['sqlite','db'])` → `mixxxImportPreview` → `showMixarConfirm` with counts and a missing-files note → `importMixxxLibrary` → `ref.invalidate(collectionsProvider)` / `collectionTracksProvider` → `showMixarToast` (destructive on error); busy spinner while running.
- [ ] **Step 4: Insert `const MixxxImportPanel()`** into `settings_library_panel.dart` after the existing fields.
- [ ] **Step 5: Run** `mise exec -- flutter test test/mixxx_import_panel_test.dart` and `mise exec -- flutter analyze` → PASS.
- [ ] **Step 6: Commit** with `feat(gui-flutter): import from Mixxx in Library settings`.

### Task 6: docs + full verification

**Files:**
- Modify: `docs/tech-spec.md` (§10.5 / §10.10: note the first adapter shipped)

- [ ] **Step 1:** Update tech-spec §10.10 status to list `mixxx` as the implemented adapter and the documented lossiness pointer.
- [ ] **Step 2: Run** `cargo --manifest-path crates/Cargo.toml fmt --check`, `cargo --manifest-path crates/Cargo.toml clippy --all-targets --all-features -- -D warnings`, `cargo --manifest-path crates/Cargo.toml test -p library-core -p library -p library-adapters -p host_flutter`, `mise exec -- flutter analyze`, `mise exec -- flutter test`.
- [ ] **Step 3: Commit** with `docs: record the Mixxx adapter`.
