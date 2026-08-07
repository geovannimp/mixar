# Controller Load Focused to Deck Implementation Plan

> **For agentic workers:** Implement task-by-task. Steps use checkbox syntax.

**Goal:** Wire DDJ LOAD buttons so the focused library/filesystem row loads onto deck 1 or 2.

**Architecture:** Controller publishes `LoadFocusedToDeck` library evt; FE resolves focus and reuses existing engine load helpers.

**Tech Stack:** library-api, controller, DDJ mappings, gui-app Zustand/TS.

## Global Constraints

- Ponytail: minimal diff; reuse `handleLoadRow` semantics.
- Deck ids are 0-based on the wire.
- After mapping changes: Controllers → Update in the app.

---

### Task 1: library-api + worker

- [ ] Add `Kind::LoadFocusedToDeck` and `EvtBody::LoadFocusedToDeck { deck }`
- [ ] Ignore kind on library worker cmd bus
- [ ] Msgpack roundtrip test

### Task 2: controller + DDJ map

- [ ] Catalog leaves + MASTER aliases `load_deck_1` / `load_deck_2`
- [ ] Resolve press → LibraryEvt with body deck 0/1
- [ ] Action unit test
- [ ] `device.toml` / `map.toml` bindings

### Task 3: FE

- [ ] Wire Kind + EvtBody schema
- [ ] `focusedLoad` + `setFocusedLoad`; handle evt → engineActions
- [ ] Panel syncs focusedLoad from tableRows + focus
- [ ] Vitest for trackId and path load paths
