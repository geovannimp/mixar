# Library Event Bus Design

**Issue:** [#123](https://github.com/geovannimp/rust-dj-engine/issues/123)  
**Date:** 2026-07-31  
**Status:** accepted  
**Parent patterns:** [engine event bus](./2026-07-26-engine-event-bus-design.md), [engine session bus](./2026-07-29-engine-session-bus-design.md)

## Goal

Add a **library-owned** command/event omnibus (cmd + evt), so hosts (Tauri now, WASM later) are a thin MessagePack bytes bridge and the frontend can subscribe to library changes instead of only polling via request/response invokes.

First slice: migrate **track analysis** onto the bus end-to-end. Other library RPCs stay on existing invokes until later PRs.

## Decisions

| Decision | Choice |
|----------|--------|
| Bus ownership | `LibrarySession` in `library` (wraps `LibraryManager` + omnibus) |
| Wire crate | New `library-api` (Origin / Kind / CmdBody / EvtBody + MessagePack) |
| Topology | Two buses: **cmd** ingress, **evt** egress |
| Publish semantics | Fire-and-forget; domain failures via `Error` / `Notice` on evt |
| Worker | Dedicated library worker thread drains cmd; host never blocks on analyze |
| Host | `library_publish` + `library://bus` forwarder (bytes only) |
| Frontend | `LibraryTransport.publish` + `subscribe`; analyze is fire-and-forget |
| First cmd / evt | `AnalyzeTrack` → `TrackAnalyzed` (or `Error`) |
| Analysis duration | Session-held default (from engine/settings); cmd carries `track_id` + `force` |
| Out of scope | Migrating other library invokes; hot-cue/loop events (#122); engine cue ownership |

## Architecture

```text
┌──────────────┐  LibraryTransport.publish      ┌──────────────┐
│ React /      │ ─────────────────────────────► │ Host bridge  │  Tauri now;
│ hooks/store  │ ◄───────────────────────────── │ (bytes only) │  WASM later
└──────────────┘  LibraryTransport.subscribe    └──────┬───────┘
                                                       │
                          library-api                  │
                          Origin / Kind / msgpack      │
                                                       ▼
                                                ┌──────────────┐
                                                │ LibrarySession│
                                                │ cmd Bus      │  omnibus
                                                │ evt Bus      │
                                                │ worker thd   │
                                                │ LibraryManager│
                                                └──────────────┘
```

### Crate split

| Crate | Owns |
|-------|------|
| `library-api` | `Origin`, `Kind`, `CmdBody`, `EvtBody`, `TrackSummary`, MessagePack encode/decode. No Tauri, no SQLite. |
| `library` | `LibrarySession`: omnibus buses, worker thread, `Arc<Mutex<LibraryManager>>`, publish helpers. |
| `apps/gui-app` (Tauri) | `library_publish` + evt→`library://bus` forwarder; keep other library invokes. |
| Frontend | Extend `LibraryTransport` with `publish` / `subscribe`; drop Promise `analyzeTrack`. |

### Omnibus mapping

| Role | Origin | Kind |
|------|--------|------|
| Analyze cmd | `Library` | `AnalyzeTrack` |
| Analyze result | `Track` (id string) | `TrackAnalyzed` |
| Failures / notices | `Library` or `Track` | `Error` / `Notice` |

Subscriptions:

- Worker: `cmd.subscribe(Any, Any)` (first slice; narrow later).
- Host bridge / UI: `evt.subscribe(Any, Any)`.

**Two buses** so hosts never observe commands.

### Worker thread

- Single dedicated thread owns the cmd subscriber and drains the cmd bus.
- Runs `LibraryManager::analyze_track` under the library mutex (sync API unchanged).
- Publishes `TrackAnalyzed` with a wire `TrackSummary`, or `Error` with message (+ track id when known).
- Does not touch the engine omnibus.

### Wire payloads (first slice)

**CmdBody**

- `AnalyzeTrack { track_id: String, force: bool }`
- `Empty`

**EvtBody**

- `TrackAnalyzed { track: TrackSummary }` — same fields as today’s GUI `TrackSummary`
- `Error { message: String, track_id: Option<String> }`
- `Notice { message: String }`
- `Empty`

**WireMessage** — same shape as engine: `origin`, `kind`, `revision`, `action_timestamp_ms`, nested `body` bytes.

### AppState / engine sharing

- `LibrarySession` owns `Arc<Mutex<LibraryManager>>`.
- `AppState` holds `Arc<LibrarySession>` (and may keep a cloned `library` Arc for existing call sites).
- `EngineSession::new_with_library` still receives the shared `LibraryManager` Arc via `session.library()`.
- Host updates session analysis-duration default when settings change (same source as today’s `engine_config.analysis_duration`).

## Host bridge (Tauri)

```text
invoke("library_publish", { payload: number[] })
  → decode WireMessage → LibrarySession.publish_cmd

evt Bus (Any, Any) → MessagePack WireMessage → emit("library://bus", bytes)
```

- Install library evt forwarder at app startup (library session always exists; unlike engine).
- Remove `analyze_library_track` Tauri command once FE no longer invokes it.
- Other library commands unchanged.

## Frontend `LibraryTransport`

```ts
interface LibraryTransport {
  // existing RPCs unchanged (except analyzeTrack removed)
  publish(origin: Origin, kind: CmdKind, fields?: Record<string, unknown>): Promise<void>;
  subscribe(handler: (message: Uint8Array) => void): Promise<() => void>;
}
```

- `useLibrary` / `LibraryPanel`: publish `analyze_track`; subscribe once; on `track_analyzed` patch tracks + resolved lookup; clear analyzing flag; surface `error` evt.
- `MemoryLibraryTransport`: in-memory handler set for publish/subscribe tests (publish may no-op or synthesize evt in tests).

## Errors

- `library_publish` returns `Err` only for encode/bridge/bus rejection.
- Analyze failures → `Kind::Error` on evt; UI toasts or sets hook error from that.

## Tests

- `library-api`: MessagePack roundtrip for wire + AnalyzeTrack / TrackAnalyzed bodies.
- `library`: session test — import wav → publish AnalyzeTrack cmd → recv TrackAnalyzed evt (feature `analysis`).
- FE: transport subscribe + memory publish wiring smoke test.

## Acceptance (from #123)

- [x] Library owns cmd + evt buses (omnibus); host does not mirror library domain state
- [x] Shared MessagePack wire types (`library-api`)
- [x] Tauri bridges bytes only (`library_publish` + `library://bus`)
- [x] `LibraryTransport` can subscribe (Tauri + memory)
- [x] Analyze mutation publishes evt observed by a Rust test (and FE via subscribe)
- [x] Design doc under `docs/superpowers/specs/`
