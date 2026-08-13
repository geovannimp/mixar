# Flutter LibraryTransport parity + LibraryManager buses — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Attach host-owned omnibus buses to `LibraryManager`, deprecate `LibrarySession` to a shim, and extend Flutter FRB `LibraryTransport` with Tauri-parity RPCs plus a thin typed analyze/refresh event stream.

**Architecture:** Host creates `LibraryBuses`, injects clones into `LibraryManager`, owns `spawn_library_worker` JoinHandle. Flutter `LibraryTransport` is that host. Tauri keeps compiling via thin `LibrarySession` shim.

**Tech Stack:** Rust (`library`, `library-api`, `host-flutter`), flutter_rust_bridge 2.12, existing omnibus worker.

## Global Constraints

- Worker `JoinHandle` must not live inside `Mutex<LibraryManager>`
- `publish_evt` for controller must work via `LibraryBuses` / bus clones without locking the DB mutex
- Flutter UI stays browse-only (no new Dart UI)
- Thin bus: analyze/refresh cmds; TrackAnalyzed/TrackUpdated/Error/Notice evts only
- Artwork = raw bytes; waveform = Tauri packed frame bytes
- Prefer shortest diffs; no new crates unless unavoidable (waveform helpers may live under `host-flutter`)

## File map

| Path | Role |
|------|------|
| `crates/library/src/bus.rs` | Add `LibraryBuses` + publish helpers |
| `crates/library/src/lib.rs` | `set_buses` / publish / subscribe on `LibraryManager` |
| `crates/library/src/worker.rs` | Export `spawn_library_worker` / `LibraryWorker` |
| `crates/library/src/session.rs` | Thin deprecated shim |
| `crates/library/tests/manager_buses.rs` | Manager + buses smoke (no session) |
| `crates/host-flutter/src/api/library.rs` | Full transport + DTOs + stream |
| `crates/host-flutter/src/waveform_render.rs` | Port from Tauri (pack + lane render) |
| `crates/host-flutter/tests/library_transport_parity.rs` | RPC + analyze stream smoke |
| FRB generated files | Regenerate after Rust API change |

---

### Task 1: `LibraryBuses` + manager attach + public worker spawn

**Files:**
- Modify: `crates/library/src/bus.rs`, `crates/library/src/lib.rs`, `crates/library/src/worker.rs`, `crates/library/src/session.rs`
- Test: `crates/library/tests/manager_buses.rs`

**Interfaces:**
- Produces:
  - `LibraryBuses { cmd, evt, revision, analysis_duration }` with `new()`, `publish_cmd`, `publish_evt`, `subscribe_evt_all`, `subscribe_evt_track`, `set_analysis_duration`, `revision`
  - `LibraryManager::set_buses(&mut self, buses: LibraryBuses)`
  - `LibraryManager::buses(&self) -> Option<LibraryBuses>` (clone)
  - `LibraryManager::publish_cmd/publish_evt/subscribe_evt_*` (err if no buses)
  - `LibraryWorker` + `spawn_library_worker(library: Arc<Mutex<LibraryManager>>) -> Result<LibraryWorker, LibraryError>`
  - `LibrarySession` reimplemented as shim holding `Arc<Mutex<LibraryManager>>` + `LibraryBuses` + `LibraryWorker`

- [ ] **Step 1: Failing test** — `manager_buses.rs`: open in-memory manager, `LibraryBuses::new()`, `set_buses`, subscribe, `publish_evt(Navigate)`, assert recv; second test: spawn worker, `publish_cmd(AnalyzeTrack)` on missing track → Error evt (or skip analyze if too heavy — RefreshTrack on missing → Error).

```rust
#[test]
fn manager_publish_evt_without_session() {
    let mut lib = LibraryManager::open_in_memory(LibraryConfig::default()).unwrap();
    let buses = LibraryBuses::new();
    lib.set_buses(buses.clone());
    let rx = lib.subscribe_evt_all().unwrap();
    lib.publish_evt(Origin::LibraryNavigation, Kind::Navigate, EvtBody::Navigate { delta: 1 }).unwrap();
    let ev = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
    assert_eq!(ev.kind(), &Kind::Navigate);
}
```

- [ ] **Step 2: Implement `LibraryBuses` in `bus.rs`** — move publish_evt encoding/revision bump from session; export from `lib.rs`.

- [ ] **Step 3: Attach buses on `LibraryManager`** — store `Option<LibraryBuses>`; implement publish/subscribe forwarding.

- [ ] **Step 4: Refactor worker** — `spawn_library_worker` clones buses from locked manager (error if unset), spawns existing loop with shutdown `Arc<AtomicBool>`; `LibraryWorker` Drop sets shutdown and joins.

- [ ] **Step 5: Rewrite `LibrarySession` as shim** — `open`/`open_in_memory` create buses, open manager, `set_buses`, wrap `Arc<Mutex<_>>`, spawn worker; methods delegate to `buses` or lock manager as today (`library()` returns Arc clone; `publish_*` use `buses` directly — no DB lock).

- [ ] **Step 6: Run** `cargo test --manifest-path crates/Cargo.toml -p library` — expect PASS.

- [ ] **Step 7: Commit** `refactor(library): attach omnibus buses to LibraryManager; session shim`

---

### Task 2: Flutter transport RPCs (add / resolve / artwork)

**Files:**
- Modify: `crates/host-flutter/src/api/library.rs`, `crates/host-flutter/Cargo.toml` (add `library-api` if needed)
- Test: extend `crates/host-flutter/tests/library_browse.rs` or new `library_transport_parity.rs`

**Interfaces:**
- Consumes: Task 1 manager+buses+worker
- Produces: `LibraryTransport` owns `Arc<Mutex<LibraryManager>>`, `LibraryBuses`, `LibraryWorker`; methods `add_folder_collection`, `resolve_tracks_for_paths`, `get_track_artwork`

DTO sketches:

```rust
pub struct AddFolderCollectionResult {
    pub collection: LibraryCollectionSummary,
    pub files_scanned: u32,
    pub tracks_added: u32,
    // map from ScanReport fields that exist today
}

pub struct ResolvedLibraryTrack {
    pub request_path: String,
    pub track: LibraryTrackSummary,
}
```

- [ ] **Step 1: Refactor `LibraryTransport::open` / `open_in_memory`** to create buses, set_buses, Arc, spawn worker (Drop joins worker).

- [ ] **Step 2: Implement add/resolve/artwork** mirroring Tauri (`NewCollection::folder`, `sync_collection`, `lookup_file_tracks_at_paths`, `read_artwork` / path resolve).

- [ ] **Step 3: Tests** — add folder via transport, list collections; resolve path; artwork may be `None` for wav.

- [ ] **Step 4: Commit** `feat(host-flutter): LibraryTransport add/resolve/artwork RPCs`

---

### Task 3: Thin typed analyze/refresh + event stream

**Files:**
- Modify: `crates/host-flutter/src/api/library.rs`
- Test: `crates/host-flutter/tests/library_transport_parity.rs`

**Interfaces:**
- Produces:

```rust
pub enum LibraryEvt {
    TrackAnalyzed { track: LibraryTrackSummary },
    TrackUpdated { track: LibraryTrackSummary },
    Error { message: String, track_id: Option<String> },
    Notice { message: String },
}

impl LibraryTransport {
    pub fn analyze_track(&self, track_id: String, force: bool) -> Result<(), String>;
    pub fn refresh_track(&self, track_id: String) -> Result<(), String>;
    pub fn subscribe_events(&self, sink: StreamSink<LibraryEvt>) -> Result<(), String>;
}
```

- [ ] **Step 1: Implement analyze/refresh** via `buses.publish_cmd` with encoded `CmdBody`.

- [ ] **Step 2: `subscribe_events`** — spawn forwarder thread: `subscribe_evt_all`, map decoded bodies to `LibraryEvt`, `sink.add(...)`; stop on sink close / transport drop (use AtomicBool on transport).

- [ ] **Step 3: Test** — seed track, `refresh_track`, expect `TrackUpdated` or `Error` on stream within timeout (use std channel test helper if StreamSink hard in unit tests — alternatively test buses path via refresh → subscribe_evt_all without FRB sink, and a small FRB-free forwarder function unit-tested).

Prefer extract `fn map_library_evt(ev: &Evt) -> Option<LibraryEvt>` + test that; StreamSink wiring smoke optional.

- [ ] **Step 4: Commit** `feat(host-flutter): typed analyze/refresh library event stream`

---

### Task 4: Waveform lane RPC

**Files:**
- Create: `crates/host-flutter/src/waveform_render.rs` (port pack + `render_scrolling_lane` + gains from Tauri)
- Modify: `crates/host-flutter/src/lib.rs` / `api/library.rs`, `Cargo.toml` (`audio-core` if needed)
- Test: pack round-trip unit test in host-flutter; optional render smoke

**Interfaces:**
- Produces:

```rust
pub struct RenderWaveformLaneRequest {
    pub track_id: Option<String>,
    pub path: Option<String>,
    pub width: u32,
    pub height: u32,
    pub position_ms: i32,
    pub visible_ms: i32,
    pub buffer_ratio: f64,
    pub include_detail: bool,
    pub include_beat_grid: bool,
    pub eq_low_db: f32,
    pub eq_mid_db: f32,
    pub eq_high_db: f32,
}

impl LibraryTransport {
    pub fn render_waveform_lane(&self, request: RenderWaveformLaneRequest) -> Result<Vec<u8>, String>;
}
```

- [ ] **Step 1: Copy pack + render helpers** from `apps/gui-app/src-tauri/src/waveform_render.rs` into host-flutter; keep pack unit test.

- [ ] **Step 2: Implement render** — resolve path from track id; load overview from library if present else empty/minimal compute; detail optional YAGNI (`include_detail` may return overview-only strip); pack frame.

- [ ] **Step 3: Test** — render with missing track errors; with seeded track returns non-empty bytes with valid header (width/height).

- [ ] **Step 4: Commit** `feat(host-flutter): render_waveform_lane packed frames`

---

### Task 5: FRB regenerate + verify browse still works

**Files:**
- Generated: `crates/host-flutter/src/frb_generated.rs`, `apps/gui-flutter/lib/src/rust/**`
- Docs: update `apps/gui-flutter/README.md` one line on LibraryTransport surface

- [ ] **Step 1:** `moon run gui-flutter:generate` (or project’s FRB generate script)

- [ ] **Step 2:** `cargo test --manifest-path crates/Cargo.toml -p host_flutter -p library`

- [ ] **Step 3:** Dart analyze / existing widget tests if cheap (`flutter test` for library providers)

- [ ] **Step 4: Commit** `chore(gui-flutter): regenerate FRB for LibraryTransport parity`

---

### Task 6: PR

- [ ] **Step 1:** Push branch, `gh pr create` summarizing library buses refactor + Flutter host API parity.

## Self-review

- Spec coverage: buses on manager, worker host-owned, session shim, controller-ready LibraryBuses, Flutter RPCs, thin typed bus, waveform/artwork shapes — tasked
- No MessagePack on Dart — tasked
- UI non-goals — respected
