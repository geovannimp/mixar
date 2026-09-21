# Stems Storage + Mixar Model Cache Implementation Plan

> **For agentic workers:** Execute task-by-task. Steps use checkbox syntax.

**Goal:** Mixar-owned ONNX cache under app-support + minimal Settings → Storage clear UI.

**Architecture:** `analyzer-stems` downloads into `{app_support}/models/`; `LibraryBuses` carries `models_root`; host FRB exposes usage/clear; Flutter Storage panel.

**Tech Stack:** Rust workspace, stem-splitter-core (registry/net/crypto only for download), FRB, Flutter settings.

## Global Constraints

- Do not write model weights to stem-splitter-core ProjectDirs.
- Never delete original library audio files.
- Manual clear only; no auto-eviction.
- Cargo via `cargo --manifest-path crates/Cargo.toml`.
- Prefer fewest files; reuse Mixar settings patterns.

## File map

| File | Role |
|------|------|
| `crates/analyzer-stems/src/split.rs` | Mixar `ensure_model(models_root, …)` |
| `crates/library/src/bus.rs` | `models_root` + getters/setters |
| `crates/library/src/stems.rs` | clear all stems + dir size |
| `crates/host-flutter/src/api/library.rs` | open sets models root; storage FRB |
| `apps/gui-flutter/lib/settings/*` | Storage section + panel |
| FRB regen | after host API change |

---

### Task 1: Mixar-owned `ensure_model`

**Files:**
- Modify: `crates/analyzer-stems/src/split.rs`
- Test: unit/integration under `analyzer-stems`

**Produces:** `pub fn ensure_model(models_root: &Path, model_name: &str) -> Result<ModelHandle>` (or void + preload); `split_interleaved_stereo` takes `models_root`.

- [x] Write failing test: after ensure, artifact exists under temp `models_root` (mock/offline if needed; or ignore network in CI with fixture file).
- [x] Implement download via `stem_splitter_core::model::registry` + `io::{net, crypto}`; build `ModelHandle { manifest, local_path }`; preload.
- [x] Update callers of `ensure_model` / `split_*` to pass root.
- [x] Commit.

---

### Task 2: Library buses + clear stems

**Files:**
- Modify: `crates/library/src/bus.rs`, `stems.rs`, `lib.rs`, `worker.rs` (pass models root into analyze options if needed)

**Produces:** `set_models_root` / `models_root`; `clear_all_track_stems(stems_root)`; `dir_size(path)`; analyze ensure uses models root when calling analyzer-stems.

- [x] Add `models_root` Arc to `LibraryBuses` (default `models`).
- [x] `clear_all_track_stems`: delete all `track_stem` rows + remove `stems_root` tree (recreate empty dir).
- [x] Wire `AnalyzeTrackOptions` / ensure path to pass `models_root` into split.
- [x] Tests for clear + size.
- [x] Commit.

---

### Task 3: Host FRB storage API

**Files:**
- Modify: `crates/host-flutter/src/api/library.rs`
- FRB generate

**Produces:**
```rust
pub struct StorageUsage { pub stems_bytes: u64, pub models_bytes: u64 }
fn storage_usage(&self) -> Result<StorageUsage, String>
fn clear_stem_cache(&self) -> Result<(), String>
fn clear_model_cache(&self) -> Result<(), String>
```
- Open: `set_models_root(parent.join("models"))` next to stems.

- [x] Implement + unit test open roots / clear empties.
- [x] `moon run gui-flutter:generate` (or `flutter_rust_bridge_codegen generate`).
- [x] Commit.

---

### Task 4: Flutter Settings → Storage

**Files:**
- Modify: `settings_section.dart`, `settings_page.dart`, `settings_sidebar.dart`
- Create: `settings_storage_panel.dart`
- Test: smoke if cheap

- [x] Section + panel with sizes and Clear buttons + confirm dialogs.
- [x] Load usage on open; refresh after clear; toast on error.
- [x] Commit.

---

### Task 5: Docs + PR

- [x] Touch `docs/stems-pad-mode-design.md` model/storage lines.
- [ ] Push branch; open PR referencing #46.
