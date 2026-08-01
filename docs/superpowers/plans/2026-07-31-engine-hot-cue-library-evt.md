# Engine Hot-Cue Snap + Library Performance Events Implementation Plan

> **For agentic workers:** Execute task-by-task. Steps use checkbox syntax.

**Goal:** Engine snaps and publishes library cmds for hot-cue/loop save/delete; library persists and emits per-track change evts; Tauri stops intercepting; FE patches decks from filtered library evts.

**Architecture:** Extend `library-api` + worker; wire `LibraryBus` into `Engine`; remove Tauri performance intercepts; hydrate and update UI via `Origin::Track` library evts.

**Tech Stack:** Rust (`library-api`, `library`, `engine-core`), Tauri bridge, React/`LibraryTransport` MessagePack wire.

## Global Constraints

- No sync library DB I/O on the engine control thread — publish library cmds only.
- Library performance evts always use `Origin::Track(track_id)`.
- Tauri does not intercept `SaveHotCue` / `DeleteHotCue` / `SaveLoop` / `DeleteLoop`.
- Follow existing MessagePack / omnibus patterns from #123.

---

### Task 1: library-api wire types

**Files:**
- Modify: `crates/library-api/src/kind.rs`
- Modify: `crates/library-api/src/payload.rs`
- Modify: `crates/library-api/src/lib.rs` (re-exports)
- Test: `crates/library-api` existing encode roundtrips or add small test

- [ ] Add kinds: `SaveHotCue`, `DeleteHotCue`, `SaveLoop`, `DeleteLoop`, `HotCuesChanged`, `LoopsChanged`
- [ ] Add `HotCue` / `SavedLoop` structs and CmdBody/EvtBody variants per design
- [ ] `cargo test -p library-api`

### Task 2: library worker + subscribe_evt_track

**Files:**
- Modify: `crates/library/src/worker.rs`
- Modify: `crates/library/src/bus.rs`, `session.rs`
- Create: `crates/library/tests/session_deck_performance_bus.rs`

- [ ] Handle four cmds: mutate via `LibraryManager`, list, publish changed evt on `Origin::Track`
- [ ] Add `subscribe_evt_track(track_id)` using `Filter::Is(Origin::Track(...))`
- [ ] Integration test: save hot cue → filtered recv with snapped-irrelevant stored position

### Task 3: engine snap + library cmd publish

**Files:**
- Modify: `crates/engine-core/src/sync.rs` (`track_id` on `DeckControlState`)
- Modify: `crates/engine-core/src/engine.rs` (library_cmd bus, set track_id on load, save/delete methods)
- Modify: `crates/engine-core/src/session.rs` / `control.rs`
- Modify: `crates/engine-core/Cargo.toml` (library-api if needed)
- Test: `crates/engine-core/tests/bus_save_hot_cue.rs`

- [ ] On SaveHotCue: snap + publish library cmd; test asserts library `HotCuesChanged` position

### Task 4: Tauri host cleanup + load hydrate

**Files:**
- Modify: `apps/gui-app/src-tauri/src/bus_bridge.rs`
- Modify: `apps/gui-app/src-tauri/src/deck_performance.rs`
- Modify: `apps/gui-app/src-tauri/src/lib.rs` (engine start passes library cmd bus; load hydrate)

- [ ] Remove four intercepts; stop overlaying cues; hydrate via `publish_evt` on load
- [ ] Wire `LibrarySession.cmd_bus()` into `EngineSession::new_with_library`

### Task 5: FE wire + per-track patch

**Files:**
- Modify: `apps/gui-app/src/lib/library/wire.ts`
- Modify: `apps/gui-app/src/stores/engineStore.ts` (or small apply helper)
- Test: wire filter / apply unit test

- [ ] Extend kinds/bodies; subscribe per loaded track; patch `hot_cues` / `saved_loops`

### Task 6: Verify + PR

- [ ] `cargo test -p library-api -p library -p engine-core --no-default-features` (plus analysis feature tests as needed)
- [ ] Commit(s); push; `gh pr create` linking #122
