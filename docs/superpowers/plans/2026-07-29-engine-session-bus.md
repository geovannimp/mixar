# Engine Session on Bus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Host-handled `start_engine` on `Origin::Engine`, store-owned bus subscribe, delete `get_status` and `useEngineBootstrap`.

**Architecture:** Add `Kind::StartEngine` (empty body). Tauri `engine_publish` handles `Origin::Engine` + `StartEngine` via a shared Rust start helper (also used by `save_settings` restart). Frontend `EngineTransport.subscribe` becomes async-ready; the engine store awaits subscribe before publishing start. No hydrate cmd.

**Tech Stack:** `engine-api` MessagePack, Tauri `bus_bridge`, Zustand `engineStore`, Vitest.

**Spec:** `docs/superpowers/specs/2026-07-29-engine-session-bus-design.md`

## Global Constraints

- Origin for start is `engine` (`Origin::Engine`), not `mixer`.
- Do not add `get_status` on the bus; remove the Tauri command and bootstrap invoke.
- Keep settings/devices/`LibraryTransport` off this path.
- Start failure: `engine_publish` `Err` + existing toast.promise; no hard-exit in this slice.
- Prefer `cargo --manifest-path crates/Cargo.toml` and `cargo check --manifest-path apps/gui-app/src-tauri/Cargo.toml`.
- Follow ponytail: fewest files; reuse `publish_status` / existing start body.

## File map

| File | Role |
|------|------|
| `crates/engine-api/src/kind.rs` | Add `StartEngine` |
| `apps/gui-app/src/lib/engine/wire.ts` | Kind + `CmdKind` `start_engine` |
| `apps/gui-app/src-tauri/src/lib.rs` | Extract `start_engine_inner`; unregister `start_engine` / `get_status` cmds |
| `apps/gui-app/src-tauri/src/bus_bridge.rs` | Host-handle `Origin::Engine` + `StartEngine` |
| `apps/gui-app/src/lib/engine/transport.ts` | `subscribe` → `Promise<() => void>` |
| `apps/gui-app/src/lib/engine/tauriTransport.ts` | `await listen` before resolving |
| `apps/gui-app/src/lib/engine/memoryTransport.ts` | Async subscribe; optional publish→handlers for tests |
| `apps/gui-app/src/stores/engineStore.ts` | `ensureBusSubscribed`, start via `publishCmd`; drop `setStatus` / `invoke` |
| `apps/gui-app/src/stores/engineStore.test.ts` | Bus subscribe + start/status coverage |
| `apps/gui-app/src/hooks/useEngineBootstrap.ts` | Delete |
| `apps/gui-app/src/hooks/useEngine.tsx` | Drop bootstrap re-export |
| `apps/gui-app/src/layouts/AppLayout.tsx` | Remove bootstrap call |
| `docs/deck-spec.md` | §3 / §8 / §9 hydrate wording |
| `docs/superpowers/specs/2026-07-29-engine-session-bus-design.md` | Status → accepted |

---

### Task 1: Wire `StartEngine` kind

**Files:**
- Modify: `crates/engine-api/src/kind.rs`
- Modify: `apps/gui-app/src/lib/engine/wire.ts` (`KindSchema`, `CmdKind`)
- Test: existing wire/kind coverage if any; otherwise `cargo test -p engine-api` + FE wire parse

**Interfaces:**
- Produces: `Kind::StartEngine` / TS `"start_engine"`; empty `CmdBody` via existing empty-body path

- [ ] **Step 1: Add Rust kind**

In `kind.rs`, add `StartEngine` to the `Kind` enum (serde `start_engine`). Place near other session/host cmds (e.g. after `LoadLibraryTrack` or with host cmds).

- [ ] **Step 2: Add TS kind**

In `wire.ts`:
- Add `"start_engine"` to `KindSchema` enum
- Add `"start_engine"` to `CmdKind` union

Empty body already works: `cmdBodyForKind("start_engine", {})` → `{ type: "empty" }`.

- [ ] **Step 3: Verify API crate**

Run: `cargo test --manifest-path crates/Cargo.toml -p engine-api`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-api/src/kind.rs apps/gui-app/src/lib/engine/wire.ts
git commit -m "$(cat <<'EOF'
feat(engine-api): add StartEngine bus kind

EOF
)"
```

---

### Task 2: Host-handled start + remove Tauri cmds

**Files:**
- Modify: `apps/gui-app/src-tauri/src/lib.rs` (`start_engine` → `pub(crate) fn start_engine_inner`, remove `get_status`, unregister both)
- Modify: `apps/gui-app/src-tauri/src/bus_bridge.rs` (handle `Origin::Engine`)
- Consumes: `Kind::StartEngine`, `publish_status`, `install_session`, `EvtForwarder`, `ensure_sampler_ready`

**Interfaces:**
- Produces: `start_engine_inner(app: &AppHandle, state: &mut AppState, session_holder: &SharedSession) -> Result<(), String>` that performs today’s start body and calls `publish_status` (ignore returned status or map to `()`).
- `save_settings` restart path calls `start_engine_inner` instead of duplicating session create (or keep inline but share helper — prefer one helper).

- [ ] **Step 1: Extract helper from current `start_engine`**

Replace `#[tauri::command] fn start_engine(...)` with:

```rust
pub(crate) fn start_engine_inner(
    app: &AppHandle,
    state: &mut AppState,
    session_holder: &SharedSession,
) -> Result<(), String> {
    if state.session.is_some() {
        let _ = publish_status(app, state);
        return Ok(());
    }
    // … existing session create / start / install / forwarder / normalizer / sampler …
    let _ = publish_status(app, state);
    Ok(())
}
```

Point `save_settings` restart at this helper where it currently duplicates the block.

- [ ] **Step 2: Host-handle in `bus_bridge`**

Before omnibus forward, add:

```rust
if msg.origin == Origin::Engine {
    match msg.kind {
        Kind::StartEngine => {
            let mut state = app_state.lock().map_err(|e| e.to_string())?;
            crate::start_engine_inner(&app, &mut state, session.inner())?;
            return Ok(());
        }
        _ => {}
    }
}
```

Confirm `session: State<'_, SharedSession>` is already a parameter (it is). Use the same holder type as `install_session`.

Do **not** forward `StartEngine` to the omnibus.

- [ ] **Step 3: Unregister commands**

Remove `get_status` function and `start_engine` command registration from `run()`. Remove `invoke_handler` entries for both.

- [ ] **Step 4: Check**

Run: `cargo check --manifest-path apps/gui-app/src-tauri/Cargo.toml`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/gui-app/src-tauri/src/lib.rs apps/gui-app/src-tauri/src/bus_bridge.rs
git commit -m "$(cat <<'EOF'
feat(gui-app): host-handle start_engine on Origin::Engine

EOF
)"
```

---

### Task 3: Async-ready transport subscribe

**Files:**
- Modify: `apps/gui-app/src/lib/engine/transport.ts`
- Modify: `apps/gui-app/src/lib/engine/tauriTransport.ts`
- Modify: `apps/gui-app/src/lib/engine/memoryTransport.ts`

**Interfaces:**
- Produces: `subscribe(handler): Promise<() => void>`
- Memory: optionally make `publish` fan-out to handlers when tests need it; for store tests, injecting via a retained handler after subscribe is enough

- [ ] **Step 1: Change interface**

```ts
export interface EngineTransport {
  publish(origin: Origin, kind: CmdKind, fields?: Record<string, unknown>): Promise<void>;
  subscribe(handler: (message: Uint8Array) => void): Promise<() => void>;
}
```

- [ ] **Step 2: Tauri impl awaits listen**

```ts
subscribe: async (handler) => {
  const unlisten = await listen<number[] | Uint8Array>(ENGINE_BUS_EVENT, (event) => {
    const payload = event.payload;
    const bytes = payload instanceof Uint8Array ? payload : Uint8Array.from(payload ?? []);
    if (bytes.length === 0) return;
    handler(bytes);
  });
  return () => {
    unlisten();
  };
},
```

- [ ] **Step 3: Memory impl**

```ts
subscribe: async (handler) => {
  handlers.add(handler);
  return () => {
    handlers.delete(handler);
  };
},
```

- [ ] **Step 4: Commit**

```bash
git add apps/gui-app/src/lib/engine/transport.ts apps/gui-app/src/lib/engine/tauriTransport.ts apps/gui-app/src/lib/engine/memoryTransport.ts
git commit -m "$(cat <<'EOF'
feat(gui-app): await engine bus listen before subscribe resolves

EOF
)"
```

---

### Task 4: Store owns bus + start; delete bootstrap

**Files:**
- Modify: `apps/gui-app/src/stores/engineStore.ts`
- Modify: `apps/gui-app/src/stores/engineStore.test.ts`
- Delete: `apps/gui-app/src/hooks/useEngineBootstrap.ts`
- Modify: `apps/gui-app/src/hooks/useEngine.tsx`
- Modify: `apps/gui-app/src/layouts/AppLayout.tsx`
- Test: `apps/gui-app` vitest store tests

**Interfaces:**
- Produces: module-level or store `ensureBusSubscribed(): Promise<void>` (one-shot); `ensureEngineRunning` awaits it then `publishCmd("engine", "start_engine")`
- Removes: `setStatus`, `invoke` for start/status, bootstrap hook

- [ ] **Step 1: Write failing store test**

In `engineStore.test.ts`, using `createMemoryEngineTransport` injected via resetting shared transport **or** testing `ensureBusSubscribed` + `applyBusBytes` path:

Minimal approach without rewriting transport singleton:

```ts
it("ensureBusSubscribed wires applyBusBytes", async () => {
  // Prefer: export setEngineTransportForTests from transport.ts for this slice, or
  // call applyBusBytes directly to prove status merge still works (existing tests).
  // Required new coverage: ensureEngineRunning publishes start_engine when not running.
});
```

Practical TDD for this codebase: add `setEngineTransportForTests(t: EngineTransport | null)` on `transport.ts` (test-only helper). Then:

```ts
it("ensureEngineRunning publishes start_engine after bus subscribe", async () => {
  const published: Array<{ origin: unknown; kind: string }> = [];
  const transport = {
    publish: async (origin, kind) => {
      published.push({ origin, kind });
    },
    subscribe: async (handler) => {
      // immediately ready
      return () => {};
    },
  };
  setEngineTransportForTests(transport as EngineTransport);
  // reset store module transport reference if store cached getEngineTransport at load —
  // if store holds `const engineTransport = getEngineTransport()` at module scope,
  // change store to call getEngineTransport() inside publishCmd / ensureBusSubscribed
  // so tests can swap the impl.
  useEngineStore.setState({ status: null, starting: false, revision: 0 });
  await useEngineStore.getState().ensureEngineRunning();
  expect(published.some((p) => p.kind === "start_engine" && p.origin === "engine")).toBe(true);
});
```

- [ ] **Step 2: Implement store changes**

- Change `publishCmd` / bus helpers to call `getEngineTransport()` each time (no module-cached transport), so tests can swap.
- Add:

```ts
let busUnlisten: (() => void) | null = null;
let busSubscribePromise: Promise<void> | null = null;

async function ensureBusSubscribed(): Promise<void> {
  if (busUnlisten) return;
  if (!busSubscribePromise) {
    busSubscribePromise = (async () => {
      const transport = getEngineTransport();
      busUnlisten = await transport.subscribe((bytes) => {
        useEngineStore.getState().applyBusBytes(bytes);
      });
    })();
  }
  await busSubscribePromise;
}
```

- `ensureEngineRunning`:

```ts
ensureEngineRunning: async () => {
  await ensureBusSubscribed();
  const { status, starting } = get();
  if (status?.running || starting) return;
  set({ starting: true });
  try {
    await toastManager.promise(
      (async () => {
        await publishCmd("engine", "start_engine");
      })(),
      { /* existing toast copy */ },
    );
  } finally {
    set({ starting: false });
  }
},
```

- Remove `invoke` import if unused; remove `setStatus` from interface/impl.
- Delete bootstrap hook; strip `AppLayout` / `useEngine` exports.

- [ ] **Step 3: Run FE tests**

Run: `cd apps/gui-app && npm test -- --run src/stores/engineStore.test.ts src/lib/engine/`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/gui-app/src/stores apps/gui-app/src/hooks apps/gui-app/src/layouts apps/gui-app/src/lib/engine/transport.ts
git commit -m "$(cat <<'EOF'
feat(gui-app): move engine bus subscribe and start into the store

EOF
)"
```

---

### Task 5: Docs + mark spec accepted

**Files:**
- Modify: `docs/deck-spec.md` (§3 engine start / §8 API / §9.7 bootstrap)
- Modify: `docs/superpowers/specs/2026-07-29-engine-session-bus-design.md` (`Status: accepted`)
- Optionally note in `docs/superpowers/specs/2026-07-27-bus-load-design.md` that `start_engine` is no longer a raw invoke (one-line fix)

- [ ] **Step 1: Update deck-spec**

Replace hydrate/`get_status` / `useEngineBootstrap` wording with store `ensureBusSubscribed` + `publishCmd("engine", "start_engine")`. Keep #109 link as done or drop “tracked issue” language.

- [ ] **Step 2: Spec status**

Set design doc `Status: accepted`.

- [ ] **Step 3: Commit**

```bash
git add docs/
git commit -m "$(cat <<'EOF'
docs: accept engine session bus spec and update deck-spec

EOF
)"
```

---

## Spec coverage check

| Spec requirement | Task |
|------------------|------|
| Delete `get_status` | 2, 4 |
| Delete `useEngineBootstrap` | 4 |
| Host-handled `start_engine` / `Origin::Engine` | 1, 2 |
| Store owns subscribe; await listen before start | 3, 4 |
| toast.promise on failure | 4 (keep existing) |
| No hard-exit | Global / out of scope |
| MemoryTransport / store test | 3, 4 |
| Docs | 5 |

## Placeholder / consistency scan

- Origin consistently `engine` / `Origin::Engine`.
- Kind name `StartEngine` / `start_engine`.
- Helper name `start_engine_inner` shared by bridge + settings restart.
- `subscribe` return type `Promise<() => void>` everywhere.
