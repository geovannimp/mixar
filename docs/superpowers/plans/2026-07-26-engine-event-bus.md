# Engine Event Bus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an engine-owned omnibus cmd/evt bus with `engine-api` postcard wire types, a Tauri bytes bridge, and a frontend `EngineTransport`, then migrate the first deck/mixer control slice off per-action invokes.

**Architecture:** `engine-api` owns `Origin`/`Kind`/postcard payloads. `engine-core` owns two omnibus buses + a control thread that applies cmds to `Engine` and publishes evt. Tauri only `engine_publish` + forwards evt to `engine://bus`. React uses `EngineTransport` (Tauri impl now, WASM later) so Zustand never calls `invoke`/`listen` for engine traffic.

**Tech Stack:** Rust workspace (`engine-api`, `engine-core`), [`omnibus`](https://docs.rs/omnibus/latest/omnibus/) 0.1, `postcard` + `serde`, Tauri 2, React/Zustand, hand-written TS wire codec with golden hex fixtures.

**Spec:** `docs/superpowers/specs/2026-07-26-engine-event-bus-design.md`

## Global Constraints

- Library / fs / settings invokes stay as they are (out of scope).
- No new per-action Tauri engine commands once the bridge exists; extend bus kinds instead.
- Fire-and-forget publish; domain errors go out on evt (`Error` / `Notice`).
- Audio/producer threads never call omnibus; control thread only.
- Postcard binary on the host wire; no JSON `EngineEvent` for migrated paths.
- First migration slice only: play/pause/seek/volume/eq/speed/crossfader/cue_mix/master_cue + status/position/levels egress. Sync/pads/sampler/performance stay on old invokes until a later plan.
- Run Cargo via `cargo --manifest-path crates/Cargo.toml …` (or `cd crates`).
- Follow ponytail: smallest working diff; one runnable check per non-trivial unit.

---

## File map

| File | Responsibility |
|------|----------------|
| `crates/engine-api/` | Shared `Origin`, `Kind`, payload structs, postcard encode/decode, golden vectors |
| `crates/engine-core/src/bus.rs` | Cmd/evt omnibus handles + `EngineSession` publish helpers |
| `crates/engine-core/src/control.rs` | Control thread: cmd subscribers → `Engine` → evt publish; position/levels tick |
| `crates/engine-core/src/session.rs` | Owns `Engine`, buses, revision, start/stop control thread |
| `apps/gui-app/src-tauri/src/bus_bridge.rs` | `engine_publish` + evt→`engine://bus` forwarder |
| `apps/gui-app/src/lib/engine/transport.ts` | `EngineTransport` interface + factory |
| `apps/gui-app/src/lib/engine/tauriTransport.ts` | Tauri impl |
| `apps/gui-app/src/lib/engine/memoryTransport.ts` | Test fake |
| `apps/gui-app/src/lib/engine/wire.ts` | Postcard-compatible encode/decode for first-slice messages |
| `apps/gui-app/src/lib/engine/applyBusEvent.ts` | Map wire evt → store patches (successor to JSON `engineEvents` for migrated kinds) |
| `apps/gui-app/src/hooks/useEngineBootstrap.ts` | Subscribe via transport; hydrate via evt Status or slim get |
| `apps/gui-app/src/stores/engineStore.ts` | First-slice actions call transport.publish |

---

### Task 1: `engine-api` crate (types + postcard)

**Files:**
- Create: `crates/engine-api/Cargo.toml`
- Create: `crates/engine-api/src/lib.rs`
- Create: `crates/engine-api/src/origin.rs`
- Create: `crates/engine-api/src/kind.rs`
- Create: `crates/engine-api/src/payload.rs`
- Create: `crates/engine-api/src/wire.rs`
- Create: `crates/engine-api/tests/postcard_roundtrip.rs`
- Modify: `crates/Cargo.toml` (add workspace member + optional workspace deps)

**Interfaces:**
- Produces: `Origin`, `Kind`, `CmdBody`, `EvtBody`, `WireMessage { origin, kind, revision, body: Vec<u8> }`, `encode_wire` / `decode_wire`, `encode_cmd_body` / `decode_cmd_body`, `encode_evt_body` / `decode_evt_body`

- [ ] **Step 1: Add workspace member and crate skeleton**

Add to `crates/Cargo.toml` members: `"engine-api"`.

`crates/engine-api/Cargo.toml`:

```toml
[package]
name = "engine-api"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Host-facing engine bus origin/kind/payload schema"

[dependencies]
serde = { workspace = true, features = ["derive"] }
postcard = { version = "1.1", features = ["alloc"] }
thiserror.workspace = true
```

`lib.rs` modules: `origin`, `kind`, `payload`, `wire`; re-export public types.

- [ ] **Step 2: Write failing round-trip test**

```rust
// crates/engine-api/tests/postcard_roundtrip.rs
use engine_api::{
    decode_wire, encode_wire, CmdBody, Kind, Origin, WireMessage,
};

#[test]
fn play_cmd_round_trips() {
    let body = engine_api::encode_cmd_body(&CmdBody::Empty).unwrap();
    let msg = WireMessage {
        origin: Origin::Deck(1),
        kind: Kind::Play,
        revision: 0,
        body,
    };
    let bytes = encode_wire(&msg).unwrap();
    let decoded = decode_wire(&bytes).unwrap();
    assert_eq!(decoded.origin, Origin::Deck(1));
    assert_eq!(decoded.kind, Kind::Play);
}
```

- [ ] **Step 3: Run test — expect FAIL (crate/types missing)**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-api --test postcard_roundtrip`

Expected: compile failure or link failure for `engine_api`.

- [ ] **Step 4: Implement minimal types + encode/decode**

`Origin`: `Engine`, `Mixer`, `Deck(u16)`.

`Kind` (first slice + egress): `Play`, `Pause`, `Seek`, `SetVolume`, `SetEq`, `SetSpeed`, `SetCrossfader`, `SetCueMix`, `SetMasterCue`, `Updated`, `Position`, `Levels`, `Status`, `Error`, `Notice`.

`CmdBody`: `Empty` | `Seek { position_secs: f64 }` | `SetVolume { volume: f32 }` | `SetEq { low: f32, mid: f32, high: f32 }` | `SetSpeed { speed: f32 }` | `SetCrossfader { position: f32 }` | `SetCueMix { mix: f32 }` | `SetMasterCue { enabled: bool }`.

`EvtBody`: `Empty` | `DeckUpdated { /* slim fields: id, playing, volume, speed, eq, position_secs, duration_secs */ }` | `Position { position_secs: f64 }` | `Levels { peak_l, peak_r, peak_hold_l, peak_hold_r: f32 }` | `EngineStatus { running, sample_rate, crossfader, cue_mix, master_cue, decks: Vec<…> }` | `Error { message: String }` | `Notice { message: String }`.

All derive `Clone, Debug, PartialEq, Serialize, Deserialize`. Use `#[serde(rename_all = "snake_case")]` only if needed; postcard uses enum discriminants — keep field order stable forever.

`wire.rs`:

```rust
pub fn encode_wire(msg: &WireMessage) -> Result<Vec<u8>, EncodeError> {
    postcard::to_allocvec(msg).map_err(...)
}
pub fn decode_wire(bytes: &[u8]) -> Result<WireMessage, DecodeError> { ... }
```

Nested: `body` is postcard of `CmdBody` or `EvtBody` depending on direction (document: cmd bus uses `CmdBody`, evt bus uses `EvtBody`).

- [ ] **Step 5: Run test — expect PASS**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-api`

Expected: PASS.

- [ ] **Step 6: Add golden hex fixture test** (for TS later)

```rust
#[test]
fn play_deck1_golden_bytes_stable() {
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    let bytes = encode_wire(&WireMessage {
        origin: Origin::Deck(1),
        kind: Kind::Play,
        revision: 0,
        body,
    }).unwrap();
    // Update expected once, then lock:
    assert_eq!(hex::encode(&bytes), "<lock-after-first-run>");
}
```

Add `hex` as dev-dep. Commit the locked hex string into `crates/engine-api/tests/golden/play_deck1.hex` (single line) and assert file contents match encode output.

- [ ] **Step 7: Commit**

```bash
git add crates/Cargo.toml crates/engine-api
git commit -m "feat(engine-api): add postcard bus origin/kind wire schema"
```

---

### Task 2: `EngineSession` + omnibus buses in `engine-core`

**Files:**
- Modify: `crates/engine-core/Cargo.toml` (deps: `engine-api`, `omnibus`)
- Create: `crates/engine-core/src/session.rs`
- Create: `crates/engine-core/src/bus.rs`
- Modify: `crates/engine-core/src/lib.rs` (mod + re-exports)
- Create: `crates/engine-core/tests/bus_play_emits_updated.rs`

**Interfaces:**
- Consumes: `engine_api::{Origin, Kind, WireMessage, encode_*, decode_*}`
- Produces: `EngineSession` with `cmd_bus()`, `evt_bus()`, `publish_cmd(origin, kind, body_bytes)`, `subscribe` helpers, `revision()`, `with_engine(|eng| …)`

- [ ] **Step 1: Write failing integration test**

```rust
// crates/engine-core/tests/bus_play_emits_updated.rs
use engine_api::{decode_evt_body, decode_wire, encode_cmd_body, CmdBody, EvtBody, Kind, Origin};
use engine_core::{EngineConfig, EngineSession};
use omnibus::Filter;
use std::time::Duration;

#[test]
fn play_on_empty_deck_publishes_error_or_updated() {
    let session = EngineSession::new(EngineConfig::default()).expect("session");
    // start engine with null backend if config allows; or skip start and expect Error
    let evt = session
        .evt_bus()
        .subscribe(Filter::Any, Filter::Any)
        .expect("sub");
    let body = encode_cmd_body(&CmdBody::Empty).unwrap();
    session
        .publish_cmd(Origin::Deck(0), Kind::Play, body)
        .expect("publish");
    let event = evt
        .recv_timeout(Duration::from_secs(1))
        .expect("recv")
        .expect("event");
    assert_eq!(*event.origin(), Origin::Deck(0));
    // Kind::Error (no track) or Kind::Updated once engine running with track — pick one and document
    assert!(matches!(*event.kind(), Kind::Error | Kind::Updated | Kind::Notice));
}
```

Adjust assertion to the real first-slice behavior you implement in Task 3 (prefer: not running → `Error`).

- [ ] **Step 2: Run test — expect FAIL**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_play_emits_updated`

Expected: `EngineSession` not found.

- [ ] **Step 3: Implement buses + session shell (no handlers yet)**

```rust
// bus.rs sketch
pub type EngineBus = omnibus::Bus<Origin, Kind, std::sync::Arc<[u8]>>;

pub fn new_buses() -> (EngineBus, EngineBus) {
    (EngineBus::with_capacity(256), EngineBus::with_capacity(256))
}
```

`EngineSession::new`: create buses, `Mutex<Option<Engine>>`, `AtomicU64` revision, spawn control thread (Task 3 fills handlers — for this step, control thread can no-op drain or forward Error "not implemented").

`publish_cmd`: `cmd.publish(Event::new(origin, kind, Arc::from(body)))`.

- [ ] **Step 4: Make test pass with minimal control loop**

Control thread: `subscribe(Filter::Any, Filter::Any)` on cmd; on any message publish evt `Kind::Error` with body `EvtBody::Error { message: "no handler".into() }` until Task 3 — **or** skip this soft stub and implement Task 3 in the same commit if smaller. Prefer implementing Task 3 next without shipping a permanent stub.

If splitting commits: test temporarily expects `Error` with `"no handler"`, Task 3 updates test to real behavior.

- [ ] **Step 5: Commit**

```bash
git add crates/engine-core
git commit -m "feat(engine-core): add EngineSession with omnibus cmd/evt buses"
```

---

### Task 3: Control thread handlers (first-slice cmds + egress)

**Files:**
- Modify: `crates/engine-core/src/control.rs` (create if needed)
- Modify: `crates/engine-core/src/session.rs`
- Modify: `crates/engine-core/tests/bus_play_emits_updated.rs`
- Optional: move notifier logic from Tauri `engine_notifier.rs` pattern into control tick

**Interfaces:**
- Consumes: `Engine::{play, pause, seek_deck, set_deck_volume, set_deck_eq_bands, set_deck_speed, set_crossfader, set_cue_mix, set_master_cue, …}`
- Produces: evt `Updated` / `Status` / `Position` / `Levels` / `Error`

- [ ] **Step 1: Update integration test for real play behavior**

With null backend + started engine + no track: `Play` → evt `Error` message containing `"track"` or `"load"`.  
With track loaded via existing `Engine::load_track` in test setup: `Play` → evt `Updated` with `playing: true`.

- [ ] **Step 2: Run — expect FAIL (handler missing)**

- [ ] **Step 3: Implement cmd dispatch**

On control thread, per deck `Filter::Is(Origin::Deck(id))` + `Filter::Any`, and mixer/engine filters:

```rust
match kind {
    Kind::Play => engine.play(deck_id),
    Kind::Pause => engine.pause(deck_id),
    Kind::Seek => { let CmdBody::Seek { position_secs } = decode...; engine.seek_deck(...) },
    Kind::SetVolume => ...,
    Kind::SetEq => ...,
    Kind::SetSpeed => ...,
    Kind::SetCrossfader => ...,
    Kind::SetCueMix => ...,
    Kind::SetMasterCue => ...,
    _ => Err(anyhow!("unsupported kind on cmd bus")),
}
```

On `Ok`: bump revision; publish `Updated` or `Status` as appropriate (crossfader → `Status` or `Mixer`+`Updated`; prefer `Origin::Mixer` + slim mix fields for crossfader/cue).  
On `Err`: publish `Origin::Engine` or deck origin + `Kind::Error` + `EvtBody::Error { message }`.

- [ ] **Step 4: Position/levels tick**

~33 ms loop on control thread (or reuse pattern from `engine_notifier.rs`): read engine snapshots / transport events; publish `Position` / `Levels`; on `TransportEvent::TrackEnded` publish `Updated`.

- [ ] **Step 5: Run tests — expect PASS**

Run: `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_play_emits_updated`

- [ ] **Step 6: Commit**

```bash
git add crates/engine-core
git commit -m "feat(engine-core): dispatch first-slice bus cmds and emit deck evt"
```

---

### Task 4: Tauri bytes bridge

**Files:**
- Create: `apps/gui-app/src-tauri/src/bus_bridge.rs`
- Modify: `apps/gui-app/src-tauri/src/lib.rs` (register commands, hold `Arc<EngineSession>`)
- Modify: `apps/gui-app/src-tauri/Cargo.toml` (`engine-api` dep)
- Modify: `AppState` to use session (or dual-hold during migration: session wraps engine)

**Interfaces:**
- Produces: `#[tauri::command] fn engine_publish(origin, kind, payload: Vec<u8>) -> Result<(), String>`
- Produces: background task forwarding evt bus → `app.emit("engine://bus", WireBytes { data: Vec<u8> })`

- [ ] **Step 1: Add `engine_publish` that publishes raw body to cmd bus**

Host sends **full** `WireMessage` bytes (origin/kind/revision/body already encoded) **or** `{ origin, kind, body }` with origin/kind as postcard/serde. Prefer single `payload: Vec<u8>` = full `encode_wire` output to keep one codec.

```rust
#[tauri::command]
fn engine_publish(session: State<SharedSession>, payload: Vec<u8>) -> Result<(), String> {
    let msg = decode_wire(&payload).map_err(|e| e.to_string())?;
    session.publish_cmd(msg.origin, msg.kind, msg.body).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Spawn evt forwarder on setup**

```rust
let rx = session.evt_bus().subscribe(Filter::Any, Filter::Any)?;
thread::spawn(move || {
    while let Ok(Some(ev)) = rx.recv() {
        let wire = encode_wire(&WireMessage {
            origin: ev.origin().clone(),
            kind: ev.kind().clone(),
            revision: session_revision, // or embed in evt body only
            body: ev.payload().as_ref().to_vec(),
        }).ok();
        if let Some(data) = wire {
            let _ = app.emit("engine://bus", data);
        }
    }
});
```

Note: omnibus payload is already body bytes; revision should be inside `EvtBody` or prepended when publishing from control thread. Pick **revision inside `WireMessage` at publish_evt time** in session helper.

- [ ] **Step 3: Manual smoke** — `cargo build -p gui-app` (or moon/tauri build path used in repo)

Run: from `apps/gui-app`, existing package build script.

Expected: compiles with new command registered in `invoke_handler!`.

- [ ] **Step 4: Commit**

```bash
git add apps/gui-app/src-tauri
git commit -m "feat(gui-app): add Tauri engine_publish and evt bus bridge"
```

---

### Task 5: Frontend `EngineTransport` + wire codec

**Files:**
- Create: `apps/gui-app/src/lib/engine/transport.ts`
- Create: `apps/gui-app/src/lib/engine/tauriTransport.ts`
- Create: `apps/gui-app/src/lib/engine/memoryTransport.ts`
- Create: `apps/gui-app/src/lib/engine/wire.ts`
- Create: `apps/gui-app/src/lib/engine/wire.test.ts` (or project’s existing test runner)
- Copy: golden hex from Task 1 into `apps/gui-app/src/lib/engine/golden/play_deck1.hex`

**Interfaces:**
- Produces:

```ts
export interface EngineTransport {
  publish(message: Uint8Array): Promise<void>;
  subscribe(handler: (message: Uint8Array) => void): () => void;
}
export function createEngineTransport(): EngineTransport;
```

- [ ] **Step 1: Write failing TS test** — decode golden `play_deck1.hex` → `{ origin: { deck: 1 }, kind: "play" }`

- [ ] **Step 2: Implement `wire.ts` encode/decode** matching Rust postcard layout (use golden vectors; do not guess — if layout fights you, add a `engine-api` test that prints field-by-field and fix TS until golden matches).

- [ ] **Step 3: Implement `TauriEngineTransport`**

```ts
publish: (message) => invoke("engine_publish", { payload: Array.from(message) }),
subscribe: (handler) => {
  const unlistenPromise = listen<number[]>("engine://bus", (ev) => {
    handler(Uint8Array.from(ev.payload));
  });
  return () => { void unlistenPromise.then((u) => u()); };
},
```

Prefer Tauri binary/ArrayBuffer emit if available; otherwise `number[]` as above.

- [ ] **Step 4: `MemoryEngineTransport`** — in-memory queue for store unit tests.

- [ ] **Step 5: Commit**

```bash
git add apps/gui-app/src/lib/engine
git commit -m "feat(gui-app): add EngineTransport and postcard wire helpers"
```

---

### Task 6: Wire Zustand first-slice actions + bootstrap

**Files:**
- Create: `apps/gui-app/src/lib/engine/applyBusEvent.ts`
- Modify: `apps/gui-app/src/hooks/useEngineBootstrap.ts`
- Modify: `apps/gui-app/src/stores/engineStore.ts` (play/pause/volume/eq/speed/crossfader/cueMix/masterCue/seek only)
- Modify: `apps/gui-app/src/lib/engineEvents.ts` — keep for unmigrated events until Task 7

**Interfaces:**
- Consumes: `createEngineTransport()`, `encodePlay(deckId)`, …
- Produces: store methods that publish; bootstrap subscribes and `applyBusEvent`

- [ ] **Step 1: Helper encoders** in `wire.ts`: `encodeDeckCmd(deckId, kind, body)`, etc.

- [ ] **Step 2: Change `play` / `pause` / … in `engineStore` to transport.publish** — do not await domain success; catch bridge errors with toast.

- [ ] **Step 3: `applyBusEvent`** — decode wire; on `Updated`/`Status`/`Position`/`Levels`/`Error` patch store (preserve title/artist/hot_cues from previous status when slim snapshot omits them — same pattern as levels merge today).

- [ ] **Step 4: Bootstrap** — `transport.subscribe(bytes => applyBusEvent(...))`; keep `get_status` hydrate until `Status` evt is emitted on subscribe (optional: publish synthetic Status on bridge start).

- [ ] **Step 5: Manual / typecheck**

Run: `npm run lint` / package typecheck for `gui-app` (moon target used in repo).

Expected: no type errors; play still moves UI via evt.

- [ ] **Step 6: Commit**

```bash
git add apps/gui-app/src
git commit -m "feat(gui-app): route first-slice deck/mixer controls through EngineTransport"
```

---

### Task 7: Remove migrated Tauri commands + dual event path cleanup

**Files:**
- Modify: `apps/gui-app/src-tauri/src/lib.rs` — remove `play_deck`, `pause_deck`, `seek_deck`, `set_deck_volume`, `set_deck_eq`, `set_deck_speed`, `set_crossfader`, `set_cue_mix`, `set_master_cue` handlers **only if** fully unused
- Modify: notifier — stop emitting JSON `engine://event` for position/levels once bus path covers them; leave JSON path for unmigrated domains or remove if bootstrap no longer listens
- Modify: `useEngineBootstrap` — remove `listen(ENGINE_EVENT)` when all store updates for first slice come from bus; unmigrated features still need JSON until later plans — **if** unmigrated cmds still call `publish_deck` JSON, keep both listeners until those migrate

** pragmatic rule:** Keep `engine://event` listener until no Tauri code emits it for paths the store still needs. First slice can leave JSON emits for load/sync/pads; store merges both.

- [ ] **Step 1: Delete first-slice command functions and invoke_handler entries**

- [ ] **Step 2: Build + smoke play/pause/volume/crossfader in the app**

- [ ] **Step 3: Commit**

```bash
git add apps/gui-app
git commit -m "refactor(gui-app): drop first-slice per-action engine Tauri commands"
```

---

### Task 8: Docs touch-up

**Files:**
- Modify: `docs/deck-spec.md` §9 — short note pointing at `docs/superpowers/specs/2026-07-26-engine-event-bus-design.md` as the implementation direction (omnibus, transport, postcard)
- Modify: `AGENTS.md` Learned Workspace Facts — one bullet on engine bus + `EngineTransport`

- [ ] **Step 1: Edit docs**

- [ ] **Step 2: Commit**

```bash
git add docs/deck-spec.md AGENTS.md
git commit -m "docs: point deck-spec §9 at engine event bus design"
```

---

## Spec coverage (self-review)

| Spec item | Task |
|-----------|------|
| `engine-api` schema + postcard | 1 |
| omnibus cmd/evt in engine | 2 |
| Control thread handlers | 3 |
| Fire-and-forget + evt errors | 3 |
| Position/levels coalesce | 3 |
| Tauri bridge only | 4 |
| `EngineTransport` + Tauri/Memory | 5 |
| Zustand first slice | 6 |
| Remove migrated invokes | 7 |
| Library out of scope | (no task — preserved) |
| WASM transport later | 5 interface only; no WasmEngineTransport impl this plan |
| Incremental later slices | deferred (new plan) |

## Out of this plan

- Sync / pads / sampler / performance / load track on the bus  
- `WasmEngineTransport`  
- MIDI subscriber  
- Deleting all JSON `engine://event` usage  

---

## Execution

Plan saved to `docs/superpowers/plans/2026-07-26-engine-event-bus.md`.
