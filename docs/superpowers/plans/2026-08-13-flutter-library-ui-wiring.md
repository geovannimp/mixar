# Flutter Library UI Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Flutter library UI to `LibraryTransport` (add folder, events, analyze/refresh, visible-row artwork) and extract shared `fs-browser` for Tauri + Flutter drive browse — no load-to-deck.

**Architecture:** Riverpod providers on top of existing FRB `LibraryTransport`; new `crates/fs-browser` with Tauri thin wrappers + Flutter FRB; artwork cache keyed by track id filled only for visible grid rows.

**Tech Stack:** Rust workspace crate, flutter_rust_bridge 2.12, Riverpod 3, Forui, trina_grid, `file_picker` (desktop folder pick).

## Global Constraints

- No load-to-deck / engine bus expansion this pass.
- Do not load artwork in `listCollectionTracks`; only `getTrack` for visible ids.
- Prefer reuse of Tauri `fs_browser.rs` logic verbatim in the new crate.
- Cargo via `cargo --manifest-path crates/Cargo.toml`; FRB regen from `apps/gui-flutter` as existing docs describe.
- Shortest working diffs; no new abstractions unless needed for the shared crate boundary.

## File map

| Path | Role |
|------|------|
| `crates/fs-browser/` | Shared volumes + directory listing |
| `apps/gui-app/src-tauri/src/fs_browser.rs` | Delete; re-export or call crate from commands |
| `crates/host-flutter/src/api/fs_browser.rs` | FRB surface |
| `apps/gui-flutter/lib/library/providers.dart` | Events, invalidate, artwork cache, drive |
| `apps/gui-flutter/lib/library/*.dart` | Panes: tabs, add folder, actions, artwork column |
| `apps/gui-flutter/pubspec.yaml` | `file_picker` |

---

### Task 1: Extract `fs-browser` crate + keep Tauri green

**Files:**
- Create: `crates/fs-browser/Cargo.toml`, `crates/fs-browser/src/lib.rs` (move from Tauri)
- Modify: `crates/Cargo.toml` (workspace member)
- Modify: `apps/gui-app/src-tauri/Cargo.toml` (dep)
- Modify: `apps/gui-app/src-tauri/src/lib.rs` (+ delete local `fs_browser.rs` or thin re-export)
- Test: `crates/fs-browser` unit tests for `browse_directory` on a tempfile tree with one wav + one dir

**Interfaces:**
- Produces: `fs_browser::list_volumes() -> Result<Vec<VolumeInfo>, String>`, `browse_directory(path) -> Result<DirectoryListing, String>`, public types `VolumeInfo`, `FsEntry`, `DirectoryListing`

- [ ] **Step 1:** Create crate; move Tauri `fs_browser.rs` body; depend on `library-core`.
- [ ] **Step 2:** Add tempfile test: dir with nested folder + `.wav` → listing splits dirs vs audio.
- [ ] **Step 3:** Wire Tauri to crate; remove duplicate module; `cargo check -p gui-app` (or tauri package name).
- [ ] **Step 4:** Commit.

### Task 2: Flutter FRB for fs-browser

**Files:**
- Create: `crates/host-flutter/src/api/fs_browser.rs`
- Modify: `crates/host-flutter/src/lib.rs` / `api/mod.rs`, `Cargo.toml`
- Regenerate: `apps/gui-flutter/lib/src/rust/**`
- Test: `crates/host-flutter/tests/fs_browser_frb.rs` or call free functions from a small rust test that mirrors FRB wrappers

**Interfaces:**
- Produces: Dart `listFsVolumes()`, `browseFsDirectory({required String path})` returning same shapes

- [ ] **Step 1:** Expose FRB wrappers mapping crate types to Dart structs (or reuse crate types if FRB-friendly).
- [ ] **Step 2:** Regenerate FRB; fix compile.
- [ ] **Step 3:** Rust test browse tempfile via host wrapper.
- [ ] **Step 4:** Commit.

### Task 3: Library events + invalidateation providers

**Files:**
- Modify: `apps/gui-flutter/lib/library/providers.dart`
- Create: `apps/gui-flutter/lib/library/library_events.dart` (optional if providers stay small)
- Test: Dart unit test for event → invalidate logic if extracted pure; else rely on manual

**Interfaces:**
- Consumes: `LibraryTransport.subscribeEvents()`
- Produces: side effects — `ref.invalidate(collectionsProvider)` / tracks; patch `analyzingTrackId`; optional last error string provider

- [ ] **Step 1:** `libraryEventsBootstrapProvider` or similar that listens while transport is alive.
- [ ] **Step 2:** On `trackUpdated` / `trackAnalyzed`, invalidate `collectionTracksProvider` (and collections counts if needed).
- [ ] **Step 3:** Commit.

### Task 4: Add folder + analyze/refresh UI

**Files:**
- Modify: `collections_pane.dart`, `track_table_pane.dart`, `pubspec.yaml` (`file_picker`)
- Modify: `providers.dart` (actions helpers)

**Interfaces:**
- Consumes: `addFolderCollection`, `analyzeTrack`, `refreshTrack`

- [ ] **Step 1:** Add folder button → `FilePicker.platform.getDirectoryPath` → transport → invalidate.
- [ ] **Step 2:** Track row actions: Analyze / Refresh (context menu or overflow); set analyzing id.
- [ ] **Step 3:** Commit.

### Task 5: Artwork column (visible rows only)

**Files:**
- Modify: `track_table_pane.dart`, `providers.dart`
- Create: `apps/gui-flutter/lib/library/artwork_cache.dart` if needed

**Interfaces:**
- Consumes: `getTrack`, visible track ids from grid
- Produces: `ProviderFamily` or map of `Uint8List?` by id

- [ ] **Step 1:** Artwork cache notifier: `ensureLoaded(ids)` with concurrency limit; ignore stale.
- [ ] **Step 2:** Trina column rendering `Image.memory` / placeholder; hook visibility (scroll listener or row build).
- [ ] **Step 3:** Commit.

### Task 6: Drive tab UI

**Files:**
- Modify: `library_panel.dart`
- Create: `drive_pane.dart`, drive providers in `providers.dart` or `drive_providers.dart`
- Modify: README snippet

**Interfaces:**
- Consumes: `listFsVolumes`, `browseFsDirectory`, `resolveTracksForPaths`

- [ ] **Step 1:** Tabs Collections | Drive.
- [ ] **Step 2:** Volumes + listing UI; navigate into dirs / up.
- [ ] **Step 3:** Resolve audio paths for metadata columns when in library.
- [ ] **Step 4:** Commit.

### Task 7: Verify + PR

- [ ] **Step 1:** `cargo test -p fs-browser -p host_flutter`; `flutter analyze` on gui-flutter if feasible.
- [ ] **Step 2:** Push branch; `gh pr create` with summary + test plan.
