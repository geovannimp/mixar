# Flutter LibraryTransport host API parity + LibraryManager buses

Date: 2026-08-13  
Status: accepted (user waived per-section + written-spec review gates)  
Depends: Flutter library browse (`2026-08-10-flutter-library-browse-design.md`), library event bus (`2026-07-31-library-event-bus-design.md`)

## Goal

1. Collapse bus ownership out of `LibrarySession` into **host-created buses + `LibraryManager`**, so the manager can publish/subscribe. Keep a thin deprecated `LibrarySession` shim so Tauri keeps compiling.
2. Finish Flutter FRB `LibraryTransport` **host API parity** with Tauri’s transport surface for this slice: add folder, resolve paths, artwork, waveform lane, plus a **thin typed** analyze/refresh bus (host encodes MessagePack; Dart never sees wire bytes). UI stays browse-only.

## Decisions

| Topic | Choice |
|--------|--------|
| Scope (Flutter) | Host API parity; no drive UI, no load-to-deck, no Dart UI for new RPCs/bus |
| Bus ownership | Host creates cmd+evt once; injects clones into `LibraryManager` via `set_buses` |
| Worker | `spawn_library_worker(Arc<Mutex<LibraryManager>>)` — `JoinHandle` stays host-owned (not inside the DB mutex) |
| Controller share | Host also keeps/clones bus handles (`LibraryBuses` / bus clones) for controller/engine later — never require locking `Mutex<LibraryManager>` just to `publish_evt` |
| `LibrarySession` | Deprecated shim over manager + buses + worker; same public methods for Tauri |
| Flutter bus wire | Typed FRB only (analyze/refresh cmds; analyze/refresh/error/notice/track_* evts) |
| Waveform | Packed `Uint8List` matching Tauri `pack_waveform_frame`; request is typed |
| Artwork | `Uint8List?` (raw bytes, not base64) |
| Auto-emit from `add_collection` | Out of scope this pass |

## Architecture

```text
Host (LibraryTransport / LibrarySession shim)
  new_buses() → LibraryBuses { cmd, evt, revision, analysis_duration }
  LibraryManager::open → set_buses(clones)
  Arc<Mutex<LibraryManager>>
  spawn_library_worker(arc)  // JoinHandle on host
  share LibraryBuses / bus clones with controller/engine (no DB lock)

LibraryManager
  DB state + Option<attached bus clones + Arc revision/analysis>
  publish_cmd / publish_evt / subscribe_evt_* / cmd_bus / evt_bus
  no JoinHandle

Worker (existing)
  drains cmd; locks manager for DB; publishes on evt via buses from manager
```

### `LibraryBuses` (library crate)

Shared, cloneable handle bundle for hosts that must publish without the DB mutex:

- `cmd: LibraryBus`, `evt: LibraryBus`
- `revision: Arc<AtomicU64>`
- `analysis_duration: Arc<Mutex<AnalysisDurationMode>>`
- helpers: `publish_cmd`, `publish_evt`, `subscribe_evt_all` / `subscribe_evt_track`

`LibraryManager::set_buses(&mut self, buses: LibraryBuses)` stores clones used by manager methods and by the worker (worker reads buses after locking manager, or receives bus clones at spawn — prefer reading from manager at spawn time via `buses()` clone so worker does not hold the DB lock).

### Flutter `LibraryTransport`

Opaque FRB type owning:

- `Arc<Mutex<LibraryManager>>`
- `LibraryBuses` (for stream + engine/controller-ready sharing)
- worker `JoinHandle` + shutdown flag (Drop joins)

Methods (extend existing browse APIs):

| Method | Notes |
|--------|--------|
| `add_folder_collection(path)` | → summary + scan report DTO |
| `resolve_tracks_for_paths(paths)` | → `{ request_path, track }` |
| `get_track_artwork(track_id?, path?)` | → `Option<Vec<u8>>` |
| `render_waveform_lane(request)` | → packed `Vec<u8>` (Tauri layout) |
| `analyze_track(track_id, force)` | typed → `publish_cmd` AnalyzeTrack |
| `refresh_track(track_id)` | typed → `publish_cmd` RefreshTrack |
| `subscribe_events(sink)` | FRB `StreamSink` of thin typed evt enum |

Thin evt variants: `TrackAnalyzed`, `TrackUpdated`, `Error`, `Notice` (other kinds ignored by the Flutter forwarder for now).

Waveform: port Tauri `waveform_render` helpers into `host-flutter` (or a tiny shared module under host-flutter). Prefer on-demand overview/detail without Tauri’s `AppState` audio cache (YAGNI); correct packed frame bytes matter more than cache.

## Non-goals

- Deleting `LibrarySession` from Tauri call sites (shim only)
- Dart UI for add folder / artwork / waveform / analyze
- Full evt taxonomy on Flutter stream (cues/loops/nav/load)
- Auto-publishing collection-changed evts from mutating manager APIs
- Engine MessagePack bus / load-to-deck

## Acceptance

- [ ] `LibraryManager` can `set_buses` and `publish_evt` / `publish_cmd` without `LibrarySession`
- [ ] `spawn_library_worker` + Drop/shutdown works; existing library session tests still pass via shim
- [ ] Controller-ready: `LibraryBuses` (or bus clones) usable for `publish_evt` without locking the manager mutex
- [ ] Flutter `LibraryTransport` exposes add/resolve/artwork/waveform + analyze/refresh + event stream
- [ ] `cargo test -p library` and `cargo test -p host_flutter` pass
- [ ] FRB regenerated; browse UI still works
