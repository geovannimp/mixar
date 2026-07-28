# Engine Event Bus Design

**Spec refs:** `docs/deck-spec.md` §7, §9; `docs/tech-spec.md` §9 (WASM)  
**Date:** 2026-07-26

## Goal

Replace per-action Tauri invokes and Tauri-owned engine status mirroring with an **engine-owned** command/event bus. Hosts (Tauri desktop now, WASM/browser later) are a thin bytes bridge. Frontend Zustand is the only UI mirror; MIDI and UI share the same egress.

## Requirements

| Decision | Choice |
|----------|--------|
| Bus ownership | Engine (`engine-core`); modules subscribe by origin/kind |
| Host role | Bridge only — no deck/mixer status mirror in Tauri |
| Library | Out of scope; keep existing library invokes |
| Topology | Two buses: **cmd** (ingress) and **evt** (egress) |
| Publish semantics | Fire-and-forget; failures via `evt` Error/Notice |
| Multi-consumer egress | UI + future MIDI both subscribe to evt bus |
| Bus crate | [`omnibus`](https://docs.rs/omnibus/latest/omnibus/) `0.1` (wasm32-ok; `Filter::Any` ≈ wildcards) |
| Wire codec | MessagePack (named maps) end-to-end — see `2026-07-26-engine-msgpack-wire-design.md` |
| Control path | Dedicated control thread drains cmd bus; audio never calls the bus |
| Shared schema | New `engine-api` crate (origin/kind + MessagePack encode/decode) |
| Frontend host swap | `EngineTransport` service wraps publish/subscribe |
| Migration | Incremental (approach 1) on top of `engine-api` (approach 3) |

Out of scope: library/fs/settings commands; implementing MIDI; shipping WASM host (design must not block it).

## Architecture

```text
┌─────────────┐  EngineTransport.publish     ┌──────────────┐
│  React /    │ ────────────────────────────► │ Host bridge  │  Tauri now;
│  Zustand    │ ◄──────────────────────────── │ (bytes only) │  WASM later
└─────────────┘  EngineTransport.subscribe   └──────┬───────┘
                                                    │
                         engine-api                 │
                         Origin / Kind / msgpack    │
                                                    ▼
                                             ┌─────────────┐
                                             │ engine-core │
                                             │ cmd Bus     │  omnibus
                                             │ evt Bus     │
                                             │ control thd │
                                             └──────┬──────┘
                        ┌───────────┬───────────────┼───────────────┐
                        ▼           ▼               ▼               ▼
                     deck sub   mixer sub     sync … later     host egress
                                                              (UI + MIDI)
```

### Crate split

| Crate | Owns |
|-------|------|
| `engine-api` | `Origin`, `Kind` (shared enum; cmd vs evt separated by which bus is used), command/event payload types, MessagePack encode/decode helpers, filter helpers. No Tauri, no audio I/O. |
| `engine-core` | Omnibus **cmd** + **evt** buses, control thread, module subscribers that call into `Engine` / DSP state, publishes evt messages. |
| `apps/gui-app` (Tauri) | `engine_publish` invoke + evt→webview forwarder only (for migrated domains). |
| Frontend | `EngineTransport` + Zustand; no direct `invoke`/`listen` for engine traffic. |

### Omnibus mapping

Events are `(origin, kind, payload)`:

| Role | origin examples | kind examples |
|------|-----------------|---------------|
| Commands | `Deck(0)`, `Deck(1)`, `Mixer`, `Engine` | `Play`, `Pause`, `SetVolume`, `SetCrossfader`, `Start` |
| Events | same entity origins | `Updated`, `Position`, `Levels`, `Status`, `Error`, `Notice` |

Subscriptions:

- Deck module: `cmd.subscribe(Filter::Is(Origin::Deck(id)), Filter::Any)` — all actions for that deck.
- Mixer / engine lifecycle: `Is(Mixer)` / `Is(Engine)` on cmd bus.
- Host bridge / UI: `evt.subscribe(Filter::Any, Filter::Any)` (one consumer multiplexes into Zustand).
- Future MIDI: narrow filters, e.g. `Is(Deck(1))` + specific kinds.

**Two buses** so hosts never observe commands. In-process payloads may be `Arc<T>`; the host bridge MessagePack-encodes at the boundary.

### Control thread

- Single dedicated thread owns module cmd subscribers and drains the cmd bus (`recv` / `drain`).
- Audio / producer threads never call omnibus; they post into a lock-free queue (or equivalent) that the control thread turns into evt publishes (position/levels/transport).
- Peak-hold / coalesce high-rate `Position` / `Levels` on the control path (≤ ~60 Hz) before evt publish.

### Payload / revision

- MessagePack for all host-facing messages.
- Discrete evt messages carry a monotonic `revision` so hosts can ignore stale patches.
- Full snapshot: `Origin::Engine` + `Kind::Status` (hydrate / multi-deck changes).
- Single-deck patch: `Origin::Deck(id)` + `Kind::Updated`.
- No request/response correlation IDs on publish.

### Errors

- `publish` returns `Err` only for encode/bridge/bus rejection (toast that).
- Domain failures (load failed, empty deck, etc.) → `Kind::Error` / `Kind::Notice` on the evt bus; UI and MIDI both see them.

## Host bridge (Tauri)

```text
invoke("engine_publish", { origin, kind, payload: number[] })
  → decode → cmd Bus::publish

evt Bus (Any, Any) → MessagePack → emit("engine://bus", bytes)
```

- No Tauri-owned `DeckStatus` / `EngineStatus` mirror for migrated paths.
- Per-action Tauri engine commands are gone; host-only cmds (load, sampler bank persistence) stay in `bus_bridge` and emit on `engine://bus`.
- The JSON `engine://event` path is **retired**. Runtime engine traffic is MessagePack on `engine://bus` only. Host-enriched deck/status payloads (track metadata, hot cues, loops, sampler) live in `engine-api` evt bodies.

## Frontend `EngineTransport`

Zustand and UI never call Tauri engine APIs directly:

```ts
interface EngineTransport {
  publish(origin: Origin, kind: Kind, payload: Uint8Array): Promise<void>;
  subscribe(handler: (msg: BusEvent) => void): () => void;
}
```

| Impl | Use |
|------|-----|
| `TauriEngineTransport` | Desktop: `engine_publish` + `engine://bus` |
| `WasmEngineTransport` | Browser later: same interface, wasm bindgen |
| `MemoryEngineTransport` | Unit tests |

- `createEngineTransport()` selects impl from build/env.
- Optional typed facade (`publishDeck(id, action)`) on top of wire bytes.

## Frontend `LibraryTransport`

Library I/O is **not** on the engine bus. Hooks use `LibraryTransport` (`listTracks`, artwork, waveform lane, …) with Tauri / memory impls under `apps/gui-app/src/lib/library/`.

Host ownership: `LibraryManager` prepares `PreparedTrackPlayback` (decode cache, waveform ensure); engine consumes it via `load_prepared_track`. Do not hold `AppState` across prepare/decode.

## Testing

| Layer | Check |
|-------|--------|
| `engine-api` | Wire round-trip; origin/kind helpers |
| `engine-core` | Headless: publish cmd `Play` → handler → evt `Updated` (no Tauri) |
| Frontend | Store + `MemoryEngineTransport` / `MemoryLibraryTransport` without Tauri |

## Migration

**Done** for engine domains (transport, mixer, sync, pads, sampler, performance, load) and library frontend transport. Remaining: WASM `EngineTransport` when the engine wasm target exists; no bus redesign.

## Relation to deck-spec §9

This supersedes the Tauri-centric half of §9 (per-command returns + Tauri-owned event emit) while keeping the goals: single source of truth in the engine, shared path for UI/MIDI, push not poll, testable headless. Status types remain conceptual mirrors in the frontend store, not in the Tauri layer.

## Non-goals / deliberate ceilings

- `ponytail:` Omnibus per-subscriber channel capacity (default 64) may drop under flood; coalesce high-rate meters on the control thread first; raise capacity only if `PublishResult::dropped` shows up in real use.
- MessagePack + hand-maintained TS decode for the first payload set; codegen later if the schema grows painful.
- Dual path during migration is gone; do not add new per-action Tauri engine commands.
