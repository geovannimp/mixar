# Flutter library browse Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or subagent-driven-development) to implement task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Browse-only Flutter library panel backed by FRB `LibraryTransport` + Riverpod + FItemGroup + TrinaGrid, sharing Tauri `library.db`.

**Architecture:** Opaque Rust `LibraryTransport` over `LibraryManager`; Dart opens via `path_provider` support dir; Riverpod drives UI.

**Tech Stack:** flutter_rust_bridge 2.12, library, flutter_riverpod, path_provider, forui FItemGroup, trina_grid

## Global Constraints

- App ID / support dir: `com.geovanni.gui-app` (match Tauri)
- No drive/FS, analyze, bus, or deck load in this plan
- List RPCs are typed FRB methods; MessagePack streams deferred

## File map

| Path | Role |
|------|------|
| `crates/host-flutter/src/api/library.rs` | `LibraryTransport` + DTOs |
| `crates/host-flutter/tests/library_browse.rs` | in-memory smoke |
| `apps/gui-flutter/lib/library/*` | providers + panel widgets |
| `apps/gui-flutter/lib/mixer/library_panel.dart` | wire real UI |
| `apps/gui-flutter/lib/main.dart` | ProviderScope, open transport |
| Platform ID files | Linux/macOS/Windows identity |

---

### Task 1: Rust LibraryTransport + smoke test

**Files:**
- Create: `crates/host-flutter/src/api/library.rs`
- Modify: `crates/host-flutter/src/api/mod.rs`, `Cargo.toml`
- Test: `crates/host-flutter/tests/library_browse.rs`

**Interfaces:**
- Produces: `LibraryTransport::open`, `open_in_memory`, `list_collections`, `list_collection_tracks`; `LibraryCollectionSummary`, `LibraryTrackSummary`

- [ ] **Step 1: Add deps** `library`, `library-core`, `tempfile` (dev)

- [ ] **Step 2: Implement `library.rs`** mirroring Tauri `collection_summary` / `track_summary` mapping

- [ ] **Step 3: Test** seed folder collection in memory / tempfile, assert list APIs

- [ ] **Step 4: Commit** `feat(host-flutter): FRB LibraryTransport browse APIs`

---

### Task 2: Regenerate FRB + app IDs + deps

**Files:**
- Modify: platform APPLICATION_ID / bundle id / Windows VERSIONINFO
- Modify: `apps/gui-flutter/pubspec.yaml` (riverpod, path_provider, trina_grid)
- Regenerate FRB bindings

- [ ] **Step 1: Align IDs** to `com.geovanni.gui-app`
- [ ] **Step 2: pub add** flutter_riverpod, path_provider, trina_grid
- [ ] **Step 3: `flutter_rust_bridge_codegen generate`**
- [ ] **Step 4: Commit**

---

### Task 3: Riverpod + LibraryPanel UI

**Files:**
- Create: `apps/gui-flutter/lib/library/providers.dart`, `collections_pane.dart`, `track_table_pane.dart`
- Modify: `library_panel.dart`, `main.dart`
- Test: widget test with provider overrides

- [ ] **Step 1: Providers** transport / collections / selection / tracks / filter
- [ ] **Step 2: Replace placeholder panes** with FItemGroup + TrinaGrid
- [ ] **Step 3: Widget test**
- [ ] **Step 4: Commit + PR**

## Self-review

- Spec coverage: browse, shared path via path_provider, FRB struct, Riverpod, FItemGroup, TrinaGrid, tests — covered
- Streams/MessagePack explicitly deferred — OK
