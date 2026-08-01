# Engine Hot-Cue Save + Library Performance Events

**Issue:** [#122](https://github.com/geovannimp/rust-dj-engine/issues/122)  
**Date:** 2026-07-31  
**Status:** accepted  
**Depends on:** [library event bus](./2026-07-31-library-event-bus-design.md) (#123)  
**Related:** [drop engine_controller](./2026-07-31-drop-engine-controller-design.md), [bus hot-cue recall](./2026-07-27-bus-hot-cue-recall-design.md)

## Goal

Move quantize-aware hot-cue (and sibling loop) **persistence** off the Tauri host intercept path:

- **Engine** owns playhead + quantize and decides snapped positions on save.
- **Library** owns cue/loop rows and **emits** change events on its evt bus.
- **Tauri** only bridges bytes (no `SaveHotCue` / `DeleteHotCue` / `SaveLoop` / `DeleteLoop` intercept).
- **FE** patches deck cue/loop UI from library evts, filtered to the **loaded track**.

## Decisions

| Topic | Choice |
|-------|--------|
| Snap owner | Engine (`snap_ms` + deck BPM/quantize + playhead) |
| Persist owner | Library worker via library **cmd** bus |
| Engine → library | Engine publishes library cmds (snapped `position_ms` / loop in-out already resolved); no sync DB I/O on the engine control thread |
| Cue list cache in engine | **No** — library is source of truth |
| Change notification | Library evt `HotCuesChanged` / `LoopsChanged` with `Origin::Track(track_id)` |
| Per-track listen | Consumers filter on `Origin::Track`; Rust: `subscribe_evt_track`; FE: existing `SubscribeFilter` |
| Host intercepts | Remove for the four kinds |
| Host `deck_quantize` / `snap_ms` | Delete once unused |
| AppState cue overlay | Stop overlaying `hot_cues` / `saved_loops` on engine `DeckUpdated`; hydrate via library evt on load |

## Flow

```text
FE → engine cmd SaveHotCue { slot }
  → control: snap playhead → library cmd SaveHotCue { track_id, slot, position_ms, … }
       → library worker: LibraryManager save → list cues
       → library evt HotCuesChanged @ Origin::Track(id)
            → library://bus → FE SubscribeFilter { origin: { track } }
            → patch decks whose track_id matches
```

Same pattern for `DeleteHotCue`, `SaveLoop` (engine supplies active loop in/out), `DeleteLoop`.

## Library wire (`library-api`)

**Kinds (cmd):** `SaveHotCue`, `DeleteHotCue`, `SaveLoop`, `DeleteLoop`  
**Kinds (evt):** `HotCuesChanged`, `LoopsChanged` (plus existing `Error` / `Notice`)

**CmdBody**

- `SaveHotCue { track_id, slot, position_ms, loop_length_beats?, color?, label? }`
- `DeleteHotCue { track_id, slot }`
- `SaveLoop { track_id, slot, in_ms, out_ms, label?, color? }`
- `DeleteLoop { track_id, slot }`

**EvtBody**

- `HotCuesChanged { track_id, hot_cues: Vec<HotCue> }` — full list after mutation
- `LoopsChanged { track_id, loops: Vec<SavedLoop> }` — full list after mutation

Wire `HotCue` / `SavedLoop` mirror existing engine-api / GUI marker fields (`slot`, `position_ms`, …).

**Origin:** mutation cmds may use `Origin::Library` or `Origin::Track`; **evts always** `Origin::Track(track_id)`.

## Engine

- Extend `DeckControlState` with `track_id: Option<TrackId>` (set on `load_prepared_track` / library load; cleared on unload).
- `Engine` holds optional `library_cmd: Option<LibraryBus>` (from `LibrarySession::cmd_bus()` at session start).
- On `SaveHotCue`: require `track_id`; `snap_ms(playhead, bpm, quantize)`; encode/publish library cmd; return `CmdOutcome` that does **not** need cue-enriched `DeckUpdated`.
- On `SaveLoop`: require loaded track + active loop region; publish library `SaveLoop` with engine in/out.
- Errors (no track, no library bus, no active loop) → engine evt `Error` as today.

## Host (Tauri)

- Delete intercept arms for the four kinds in `bus_bridge`.
- Delete `deck_quantize`, host `snap_ms`, and `save_*_inner` / `delete_*_inner` performance mutators (keep `fetch_deck_performance` helpers if useful for hydrate).
- On track load: after reading cues/loops from DB, `LibrarySession::publish_evt` `HotCuesChanged` + `LoopsChanged` for that track (hydrate). Do not rely on AppState overlay for those fields.
- `overlay_host_enrichment`: stop writing `hot_cues` / `saved_loops` (leave empty / omit trust path).

## Frontend

- Extend `library/wire.ts` kinds + bodies.
- Subscribe (deck or store) with `filter: { origin: { track: trackId } }` when a deck’s `track_id` is set; on `hot_cues_changed` / `loops_changed`, patch matching decks in `engineStore`.
- Unsubscribe / resubscribe when loaded track changes.
- `saveHotCue` etc. remain engine `publishCmd` (unchanged FE cmd path).

## Tests

- Library session: save/delete hot cue cmd → `HotCuesChanged` with expected position; same for loops.
- Engine: with quantize on, `SaveHotCue` publishes library cmd whose `position_ms` is snapped (assert via library evt or cmd subscriber).
- FE: `matchesSubscribeFilter` / apply helper patches only the deck for that track.

## Acceptance (extends #122)

- [ ] Saving a hot cue with quantize on/off matches engine quantize without host `deck_quantize`
- [ ] Host does not call `snap_ms` / intercept the four kinds
- [ ] Library emits `HotCuesChanged` / `LoopsChanged` on `Origin::Track`
- [ ] FE can listen to a single track’s performance events
- [ ] Engine-side (or library+engine) test covers snap-on-save

## Non-goals

- Migrating all remaining library invokes onto the cmd bus
- Engine-owned long-lived cue tables
- Moving `LoadLibraryTrack` off the host in this PR
