# Qualified Controller Actions + Library Navigation Design

**Date:** 2026-08-03  
**Status:** accepted  
**Branch context:** `feat/controller-mapping`  
**Related:** [engine event bus](./2026-07-26-engine-event-bus-design.md), [library event bus](./2026-07-31-library-event-bus-design.md)

## Goal

Give MIDI/controller mappings one action vocabulary that can target **engine commands** and **library UI navigation**, without collapsing structured wire `(Origin, Kind)` into strings.

Map actions become qualified ids (`Deck(_)::set_volume`, `LibraryNavigation::navigate_next`). The resolver binds wildcards from the map section and returns a **routable** result. The host publishes engine cmds on the engine cmd bus, and library-navigation signals as **events** on the library evt bus for the frontend to consume.

## Requirements

| Decision | Choice |
|----------|--------|
| Action id shape | `<OriginTemplate>::<leaf>` in map.toml / catalog only |
| Wildcard | `Deck(_)` inherits deck index from section (`deck_1` → 0) |
| Absolute deck | `Deck(0)::…` / `Deck(1)::…` |
| Index-less origins | `Mixer::…`, `Engine::…`, `LibraryNavigation::…` |
| Wire format | Unchanged MessagePack enums (`engine-api` / `library-api`) |
| Library nav direction | Backend → FE on library **evt** bus (not worker cmd) |
| Library nav origin | New `library-api::Origin::LibraryNavigation` |
| Short action names | Retired; migrate all maps/fixtures in the same change |
| CmdBody / toggles | Still resolved explicitly (not inferred from split alone) |

### Out of scope (this design)

- Full library browser UX beyond consuming nav evts (selection model details may be minimal).
- Keyboard shortcut layer (may reuse the same catalog later).
- Renaming engine/library wire `Kind` values to `Origin::Kind` strings.
- Moving library worker responsibilities; nav never touches `LibraryManager`.

## Architecture

```text
MIDI / map.toml
    │  action = "Deck(_)::set_volume" | "LibraryNavigation::navigate_next"
    ▼
MappingSession::resolve (controller)
    │  bind Deck(_) from section → Deck(n)
    │  build CmdBody / EvtBody as today for engine leaves
    ▼
RoutedAction
    ├─ EngineCmd { origin, kind, body } ──► engine cmd bus
    └─ LibraryEvt { origin, kind, body } ──► library evt bus
                                                    │
                                                    ▼
                                          library://bus → FE
                                          (move library selection)
```

Controller today only implements `BusPublish` for engine. Replace (or wrap) with a host router that can publish to either bus.

## Section 1 — Action catalog syntax

Map/catalog action ids:

```text
<OriginTemplate>::<action_leaf>
```

| Form | Meaning |
|------|---------|
| `Deck(0)::set_volume` | Engine deck 0 |
| `Deck(_)::set_volume` | Deck index from map section |
| `Mixer::set_crossfader` | Mixer |
| `Engine::start_engine` | Engine lifecycle |
| `LibraryNavigation::navigate_next` | Library UI evt (no index) |

Rules:

- Exactly one `::` separator between origin template and leaf. Leaf may still be parameterized (`trigger_hot_cue_1`, `auto_loop_4`).
- `_` only where the section can supply that origin (e.g. `Deck(_)` under `deck_*`). Invalid combo → `map-check` / load validation error.
- Short names (`set_volume`) are removed from `catalog::ACTIONS`; fixtures and any shipped maps migrate.
- Parsing lives in `controller` (catalog + resolve). FE does not parse these strings for engine traffic.

Serde/display spelling for templates matches debug-ish origin forms: `Deck(0)`, `Deck(_)`, `Mixer`, `Engine`, `LibraryNavigation` (PascalCase origin name, snake_case leaf — leaf stays today’s action leaf vocabulary).

## Section 2 — Resolver and `RoutedAction`

`resolve_action` today returns `Option<(engine_api::Origin, Kind, CmdBody)>`.

Change to roughly:

```rust
enum RoutedAction {
    EngineCmd {
        origin: engine_api::Origin,
        kind: engine_api::Kind,
        body: engine_api::CmdBody,
    },
    LibraryEvt {
        origin: library_api::Origin,
        kind: library_api::Kind,
        body: library_api::EvtBody,
    },
}
```

Flow:

1. Split action on first `::` → `(origin_template, leaf)`.
2. Resolve template + section → concrete origin (engine or library).
3. Match on `(domain, leaf)` (and `norm` / `active` / snapshot) to build body — same special cases as today (`toggle_play`, EQ merge, hot-cue save-or-trigger, etc.).
4. Return `RoutedAction`.

`origin_for_section` remains the source for `Deck(_)` binding. Absolute `Deck(n)` ignores section for the deck index (section may still gate soft-takeover keys).

`BusPublish` becomes something like:

```rust
trait ActionPublish {
    fn publish_engine(&mut self, origin: engine_api::Origin, kind: engine_api::Kind, body: engine_api::CmdBody);
    fn publish_library_evt(&mut self, origin: library_api::Origin, kind: library_api::Kind, body: library_api::EvtBody);
}
```

Host (Tauri / probe) implements both; headless tests use fakes.

## Section 3 — Library navigation on the library evt bus

### API additions (`library-api`)

- `Origin::LibraryNavigation` (alongside `Library`, `Track(String)`).
- Kinds (evt): at minimum `NavigateNext`, `NavigatePrev` (snake_case on wire: `navigate_next`, `navigate_prev`). Extend later with `NavigateParent` / `Activate` / etc. only when needed.
- `EvtBody`: `Empty` is enough for next/prev; add fields only if a later kind needs them.
- No new `CmdBody` variants for navigation. Worker does **not** subscribe to or handle these kinds.

### Publish path

When MIDI resolves `LibraryNavigation::navigate_next`:

1. Controller emits `RoutedAction::LibraryEvt { origin: LibraryNavigation, kind: NavigateNext, body: Empty }`.
2. Host publishes onto the library **evt** omnibus (same forwarder as worker-produced evts → `library://bus`).
3. Revision: use the library session’s normal evt revision counter (host calls into `LibrarySession` helper e.g. `publish_ui_evt`, so revision stays monotonic with other library evts).

Worker thread ignores unknown origins/kinds; nav must not require worker involvement.

### Why evt, not cmd

Library bus design: hosts never observe cmds. Navigation is a signal **to** the UI, so it is egress. Treating it as cmd would either force FE to listen to cmds (break the model) or force a pointless worker echo.

## Section 4 — Frontend

- Existing `LibraryTransport.subscribe` already receives all library evts.
- In the library store / panel focus owner: on `origin: library_navigation` + `navigate_next` / `navigate_prev`, move the focused row (or tree selection) the same way arrow keys would.
- If the library UI is unfocused / no rows: no-op (or optional notice later — YAGNI).
- Engine Zustand path unchanged for `Deck` / `Mixer` / `Engine` actions.

## Section 5 — Migration and validation

- Update `catalog.rs` action list to qualified forms.
- Update all `crates/controller/tests/fixtures/**/map.toml` bindings.
- `map-check` / `is_known_action`: validate template parse + leaf ∈ allowed set for that origin domain.
- Soft-takeover / absolute-action sets key off the **bound** qualified id (e.g. `Deck(0)::set_volume` after `_` resolution), not the unbound map string.

## Tests

| Layer | Check |
|-------|--------|
| `controller` unit | Parse `Deck(_)::set_volume` + section `deck_1` → `Deck(0)`; `Deck(1)::…` absolute; invalid `Mixer::` under `deck_1` with `_` where N/A |
| `controller` unit | `LibraryNavigation::navigate_next` → `RoutedAction::LibraryEvt` |
| `controller` session | Fake `ActionPublish` records engine vs library publishes |
| `library-api` | MessagePack roundtrip for new Origin + Kind |
| `library` | `publish_ui_evt` appears on evt subscriber; worker does not need to run |
| FE (minimal) | Memory transport delivers nav evt; store/selection helper advances index |

## Acceptance

- [x] Map actions use `OriginTemplate::leaf`; short names gone from catalog + fixtures
- [x] `Deck(_)` binds from section; `Deck(n)` absolute
- [x] Engine mappings still publish engine cmds with correct bodies (incl. toggles / soft-takeover)
- [x] `LibraryNavigation::navigate_*` publishes library **evt**; FE can move selection
- [x] Library worker unchanged for nav; no `LibraryManager` calls
- [x] Wire remains typed enums (no string kinds on the bus)

## Non-goals / deferred

- Keyboard shortcuts sharing the catalog (reuse later).
- Rich library nav kinds (page up, focus collections tree, load to deck) until product asks.
- Script bindings emitting qualified strings (scripts can call publish APIs directly).
