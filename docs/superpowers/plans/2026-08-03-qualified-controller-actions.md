# Qualified Controller Actions + Library Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Qualify map actions as `OriginTemplate::leaf`, route engine cmds vs library navigation evts, and let the FE move track-table focus on `LibraryNavigation` events.

**Architecture:** Parse qualified action strings in `controller`; return `RoutedAction`. Host implements `ActionPublish` (engine cmd + library evt). `library-api` gains `Origin::LibraryNavigation` + `NavigateNext`/`NavigatePrev`. FE applies those evts to a focused row index in the library track table.

**Tech Stack:** Rust (`controller`, `library-api`, `library`), MessagePack wire, Tauri/probe hosts, React + Zustand library store.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-03-qualified-controller-actions-design.md`
- Wire stays typed enums — no string kinds on the bus
- Nav is library **evt** only; worker does not handle it
- Cargo: `cargo --manifest-path crates/Cargo.toml …`
- Short action names retired; migrate catalog + fixtures together
- YAGNI: only `navigate_next` / `navigate_prev` for nav kinds

## File structure

| Path | Responsibility |
|------|----------------|
| `crates/library-api/src/origin.rs` | Add `LibraryNavigation` |
| `crates/library-api/src/kind.rs` | Add `NavigateNext`, `NavigatePrev` |
| `crates/library-api/tests/msgpack_roundtrip.rs` | Roundtrip new origin/kinds |
| `crates/library/src/session.rs` | Already has `publish_evt` — use as UI publish path |
| `crates/library/tests/session_ui_nav_evt.rs` | Subscribe receives nav evt without worker cmd |
| `crates/controller/Cargo.toml` | Add `library-api` dep |
| `crates/controller/src/action_id.rs` | Parse/bind `OriginTemplate::leaf` |
| `crates/controller/src/action.rs` | `RoutedAction` + resolve leaves |
| `crates/controller/src/catalog.rs` | Qualified ACTIONS list |
| `crates/controller/src/session.rs` | `ActionPublish` trait; route publishes |
| `crates/controller/tests/**` | Fixtures + CaptureBus updates |
| `crates/midi-map-probe/src/main.rs` | Implement `ActionPublish` |
| `apps/gui-app/src/lib/library/wire.ts` | Origin/kind schemas |
| `apps/gui-app/src/stores/libraryStore.ts` | Focused row + nav handlers |
| `apps/gui-app/src/components/LibraryTrackTable.tsx` (or `library/…`) | Highlight focused row |

---

### Task 1: library-api Origin + Kind

**Files:**
- Modify: `crates/library-api/src/origin.rs`
- Modify: `crates/library-api/src/kind.rs`
- Modify: `crates/library-api/tests/msgpack_roundtrip.rs`

**Interfaces:**
- Produces: `Origin::LibraryNavigation`, `Kind::NavigateNext`, `Kind::NavigatePrev` (wire snake_case)

- [ ] **Step 1: Write failing roundtrip test**

```rust
#[test]
fn library_navigation_origin_and_navigate_kinds_roundtrip() {
    let origin = Origin::LibraryNavigation;
    let kind = Kind::NavigateNext;
    let wire = encode_wire(&WireMessage {
        origin: origin.clone(),
        kind: kind.clone(),
        revision: 1,
        action_timestamp_ms: 0,
        body: encode_evt_body(&EvtBody::Empty).unwrap().into(),
    })
    .unwrap();
    let decoded = decode_wire(&wire).unwrap();
    assert_eq!(decoded.origin, Origin::LibraryNavigation);
    assert_eq!(decoded.kind, Kind::NavigateNext);
}
```

- [ ] **Step 2: Run test — expect fail (variant missing)**

Run: `cargo --manifest-path crates/Cargo.toml test -p library-api library_navigation -- --nocapture`

- [ ] **Step 3: Add variants**

`origin.rs`:

```rust
pub enum Origin {
    Library,
    Track(String),
    LibraryNavigation,
}
```

`kind.rs`: add `NavigateNext`, `NavigatePrev` to `Kind`.

- [ ] **Step 4: Run test — expect pass**

- [ ] **Step 5: Commit**

```bash
git add crates/library-api
git commit -m "feat(library-api): LibraryNavigation origin and navigate kinds"
```

---

### Task 2: library session UI evt smoke test

**Files:**
- Create: `crates/library/tests/session_ui_nav_evt.rs`

**Interfaces:**
- Consumes: `LibrarySession::publish_evt`, `Origin::LibraryNavigation`, `Kind::NavigateNext`
- Produces: proof host can publish nav without `publish_cmd`

- [ ] **Step 1: Write test**

```rust
#[test]
fn publish_ui_nav_evt_reaches_subscriber() {
    let session = LibrarySession::open_in_memory(LibraryConfig::default()).unwrap();
    let mut rx = session.subscribe_evt_all().unwrap();
    session
        .publish_evt(Origin::LibraryNavigation, Kind::NavigateNext, EvtBody::Empty)
        .unwrap();
    let ev = rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap();
    assert_eq!(ev.origin, Origin::LibraryNavigation);
    assert_eq!(ev.kind, Kind::NavigateNext);
}
```

- [ ] **Step 2: Run — expect pass** (uses existing `publish_evt`)

Run: `cargo --manifest-path crates/Cargo.toml test -p library session_ui_nav -- --nocapture`

- [ ] **Step 3: Commit**

```bash
git add crates/library/tests/session_ui_nav_evt.rs
git commit -m "test(library): LibraryNavigation evt reaches subscribers"
```

---

### Task 3: Parse action ids + catalog migration

**Files:**
- Create: `crates/controller/src/action_id.rs`
- Modify: `crates/controller/src/lib.rs` (mod + re-exports)
- Modify: `crates/controller/src/catalog.rs`
- Modify: `crates/controller/tests/fixtures/**/map.toml`
- Create: `crates/controller/src/action_id.rs` unit tests in same file or `tests/action_id.rs`

**Interfaces:**
- Produces:
  - `enum OriginTemplate { Deck(Option<u16>) /* None = _ */, Mixer, Engine, LibraryNavigation }`
  - `fn parse_action_id(action: &str) -> Result<(OriginTemplate, &str), LoadError>`
  - `fn bind_origin(template: OriginTemplate, section: &str) -> Result<BoundOrigin, LoadError>`
  - `enum BoundOrigin { Engine(engine_api::Origin), LibraryNavigation }`
  - `is_known_action` accepts only qualified ids

- [ ] **Step 1: Failing tests for parse/bind**

```rust
#[test]
fn deck_wildcard_binds_from_section() {
    let (t, leaf) = parse_action_id("Deck(_)::set_volume").unwrap();
    assert_eq!(leaf, "set_volume");
    let BoundOrigin::Engine(Origin::Deck(0)) = bind_origin(t, "deck_1").unwrap() else { panic!() };
}

#[test]
fn deck_absolute_ignores_section_index() {
    let (t, _) = parse_action_id("Deck(1)::set_volume").unwrap();
    let BoundOrigin::Engine(Origin::Deck(1)) = bind_origin(t, "deck_1").unwrap() else { panic!() };
}

#[test]
fn library_navigation_parses() {
    let (t, leaf) = parse_action_id("LibraryNavigation::navigate_next").unwrap();
    assert!(matches!(t, OriginTemplate::LibraryNavigation));
    assert_eq!(leaf, "navigate_next");
}
```

- [ ] **Step 2: Run — fail**

- [ ] **Step 3: Implement `action_id.rs` + wire into `catalog` / `map_file` validation**

Qualified ACTIONS examples: `"Deck(_)::set_volume"`, `"Mixer::set_crossfader"`, `"LibraryNavigation::navigate_next"`, … (migrate every former short name). Absolute-action set uses same qualified forms (wildcards OK in catalog; soft-takeover keys use **bound** id at runtime).

Update fixtures:

```toml
play_pause = "Deck(_)::toggle_play"
volume = [
  { action = "Deck(_)::set_volume", soft_takeover = true },
  { action = "Deck(_)::set_filter", modifier = "custom.shift", soft_takeover = true },
]
```

- [ ] **Step 4: Run controller tests that only load maps — fix until green for load**

Run: `cargo --manifest-path crates/Cargo.toml test -p controller bundle_load -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add crates/controller
git commit -m "feat(controller): parse OriginTemplate::leaf action ids"
```

---

### Task 4: RoutedAction + ActionPublish + resolve_action

**Files:**
- Modify: `crates/controller/Cargo.toml` (depend on `library-api`)
- Modify: `crates/controller/src/action.rs`
- Modify: `crates/controller/src/session.rs`
- Modify: `crates/controller/tests/session_input.rs` (and other CaptureBus users)
- Modify: `crates/midi-map-probe/src/main.rs`
- Modify: `crates/controller/src/lib.rs` exports

**Interfaces:**
- Produces:

```rust
pub enum RoutedAction {
    EngineCmd { origin: engine_api::Origin, kind: engine_api::Kind, body: engine_api::CmdBody },
    LibraryEvt { origin: library_api::Origin, kind: library_api::Kind, body: library_api::EvtBody },
}

pub trait ActionPublish {
    fn publish_engine(&mut self, origin: engine_api::Origin, kind: engine_api::Kind, body: engine_api::CmdBody);
    fn publish_library_evt(&mut self, origin: library_api::Origin, kind: library_api::Kind, body: library_api::EvtBody);
}
```

- `resolve_action(action, section, norm, active, snap) -> Option<RoutedAction>` (section required for `_`)
- Deprecate/remove old `BusPublish` or make it a blanket helper that only handles engine (prefer replace call sites)

- [ ] **Step 1: Failing test — navigate_next routes to LibraryEvt**

```rust
#[test]
fn resolve_library_navigate_next() {
    let snap = ControlSnapshot::default();
    let routed = resolve_action("LibraryNavigation::navigate_next", "master", 0.0, true, &snap).unwrap();
    match routed {
        RoutedAction::LibraryEvt { origin, kind, body } => {
            assert_eq!(origin, library_api::Origin::LibraryNavigation);
            assert_eq!(kind, library_api::Kind::NavigateNext);
            assert_eq!(body, library_api::EvtBody::Empty);
        }
        _ => panic!("expected LibraryEvt"),
    }
}
```

- [ ] **Step 2: Run — fail**

- [ ] **Step 3: Implement RoutedAction; change resolve_action to parse id then match on leaf (engine cases unchanged); session publishes via ActionPublish**

Soft-takeover lookup key: after bind, format bound qualified id e.g. `Deck(0)::set_volume`.

- [ ] **Step 4: Update CaptureBus / midi-map-probe; run `cargo test -p controller`**

- [ ] **Step 5: Commit**

```bash
git add crates/controller crates/midi-map-probe
git commit -m "feat(controller): RoutedAction and ActionPublish for engine vs library nav"
```

---

### Task 5: Frontend wire + focused row navigation

**Files:**
- Modify: `apps/gui-app/src/lib/library/wire.ts`
- Modify: `apps/gui-app/src/stores/libraryStore.ts`
- Modify: `apps/gui-app/src/stores/libraryStore.test.ts`
- Modify: track table component path that exists on branch (`LibraryTrackTable.tsx` or `library/LibraryTrackTable.tsx`)
- Modify: parent that owns rows (`LibraryPanel.tsx`) to pass focus props if needed

**Interfaces:**
- Produces: store fields `focusedTrackRowIndex: number` (or track id); actions `navigateTrackFocus(delta: 1 | -1)`
- On bus evt `library_navigation` + `navigate_next`/`navigate_prev`, call navigate helper clamped to visible row count

- [ ] **Step 1: Failing store test**

```ts
it("navigate_next advances focusedTrackRowIndex", () => {
  useLibraryStore.setState({ focusedTrackRowIndex: 0, trackRowCount: 3 });
  useLibraryStore.getState().applyNavKind("navigate_next");
  expect(useLibraryStore.getState().focusedTrackRowIndex).toBe(1);
});
```

- [ ] **Step 2: Run — fail**

- [ ] **Step 3: Implement wire origin/kind + store apply path from existing bus subscribe + highlight row in table**

- [ ] **Step 4: `cd apps/gui-app && npm test`**

- [ ] **Step 5: Commit**

```bash
git add apps/gui-app
git commit -m "feat(gui): apply LibraryNavigation evts to track table focus"
```

---

### Task 6: Spec status + acceptance smoke

**Files:**
- Modify: `docs/superpowers/specs/2026-08-03-qualified-controller-actions-design.md` status → accepted

- [ ] **Step 1: Mark acceptance checkboxes that are done; status accepted**

- [ ] **Step 2: Run full relevant tests**

```bash
cargo --manifest-path crates/Cargo.toml test -p library-api -p library -p controller
cd apps/gui-app && npm test
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-08-03-qualified-controller-actions-design.md
git commit -m "docs: accept qualified controller actions design"
```

---

## Spec coverage checklist

| Spec item | Task |
|-----------|------|
| Qualified action syntax + `_` bind | 3 |
| RoutedAction / ActionPublish | 4 |
| LibraryNavigation origin + navigate kinds | 1 |
| library evt publish path | 2, 4 |
| FE consume nav | 5 |
| Catalog/fixture migration | 3 |
| Wire stays enums | 1 (no string kinds) |
| Worker unchanged for nav | 2 (evt only) |
