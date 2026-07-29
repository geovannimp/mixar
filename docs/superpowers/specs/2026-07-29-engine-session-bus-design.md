# Engine session on the bus (drop get_status / bootstrap hook)

Date: 2026-07-29  
Issue: [#109](https://github.com/geovannimp/rust-dj-engine/issues/109)  
Parent: [engine event bus](./2026-07-26-engine-event-bus-design.md)  
Status: accepted

## Goal

Move engine session lifecycle fully onto `EngineTransport` + the Zustand engine store:

- Delete `invoke("get_status")` and the Tauri `get_status` command.
- Delete `useEngineBootstrap` (and its `AppLayout` call / re-export).
- Start the engine via host-handled `publishCmd` / `engine_publish` (not a raw store invoke).
- Store owns bus subscribe + start; UI stays a thin caller (`ensureEngineRunning`).

Keep settings, devices, FS browser, and `LibraryTransport` off this path.

## Why drop hydrate

`get_status` only seeds the store before the engine exists. There is no meaningful engine mirror until `start_engine` succeeds, and that path already emits `status` via `publish_status`. Bootstrap subscribe + start is enough if the listener is ready before start emits.

## Design

### Wire

| Kind | Origin | Body | Host |
|------|--------|------|------|
| `start_engine` | `engine` | empty | Create session, start engine, install evt forwarder, sampler ready, emit `status` (same as today’s `start_engine` command). Idempotent if already running. |
| `get_status` | — | — | **Not added.** Removed. |

Not forwarded to the omnibus control thread (session create / `EvtForwarder` / `AppState` are host-owned), same class as load and sampler bank cmds.

On start failure: `engine_publish` returns `Err` (toast.promise as today). Optional `error` evt if useful; no hard-exit in this slice.

### Frontend store

- `ensureBusSubscribed()`: one-shot; `await` transport `listen` registration, then wire `applyBusBytes`. Expose readiness so start cannot race the async listener.
- `ensureEngineRunning()`: await bus subscribe → if not `status?.running` and not `starting` → `publishCmd("engine", "start_engine")` inside existing toast.promise.
- Remove `setStatus` if unused after dropping hydrate.
- Delete `useEngineBootstrap.ts`; `AppLayout` no longer mounts it. `MixerPage` keeps calling `ensureEngineRunning` on enter.

### Host

- Move `start_engine` body into `bus_bridge` host-handled match (or shared helper called from there).
- Unregister Tauri `get_status` and `start_engine` commands once the store no longer invokes them.
- `save_settings` / other host paths that restart the engine may keep calling the shared start helper in Rust; they do not need a FE invoke.

### Tests

- Store / `MemoryEngineTransport`: subscribe → publish `start_engine` (memory host stub or direct `applyBusBytes` with a `status` payload) → `status.running` true; no `get_status` invoke.
- Wire kind round-trip for `start_engine` if added to `engine-api` / TS `wire.ts`.

## Out of scope

- Hard-exit on start failure (AGENTS preference) — separate change.
- Folding settings/devices into the engine bus.
- Moving `LibraryTransport` into the engine store.
- Request/response correlation on the transport.

## Acceptance (from #109, updated)

- [x] No `invoke("get_status")` in the app; Tauri command removed.
- [x] No `invoke("start_engine")` from the store/UI; start goes through `publishCmd` / `engine_publish`.
- [x] No `useEngineBootstrap`; bus subscribe lives in the engine store.
- [x] Entering decks still auto-starts; failures still surface via toast.promise.
- [x] First `status` after start is not missed (subscribe ready before start emit).
- [x] `MemoryEngineTransport` can exercise subscribe + start/status without Tauri.

## Docs

Update `deck-spec` §3 / §8 / §9 bootstrap notes and close the “tracked issue” hydrate wording once implemented.
