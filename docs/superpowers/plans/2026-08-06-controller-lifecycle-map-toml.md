# Controller Lifecycle map.toml Implementation Plan

> **For agentic workers:** Inline execution. Checkbox steps for tracking.

**Goal:** Drive Rhai lifecycle hooks from `[lifecycle]` in `map.toml` (explicit only).

**Architecture:** Parse closed lifecycle table on `MapFile`; session resolves event → fn name; omit section ⇒ no hooks.

**Tech Stack:** Rust `serde`/`toml`, existing `ScriptRuntime::call_hook`, `map-check`.

## Global Constraints

- String values only (no interval tables)
- Closed keys: `on_init`, `on_shutdown`, `idle_heartbeat`
- No `[lifecycle]` ⇒ no hooks

---

### Task 1: Parse + session wiring + tests

**Files:** `map_file.rs`, `session.rs`, `check.rs` (if needed), fixtures, `ddj-400/map.toml`, `map.tosd`, tests

- [x] Add `LifecycleHooks` / field on `MapFile`
- [x] Session looks up names; skip if absent
- [x] Validate unknown keys
- [x] Migrate DDJ + with-script; add tests
- [x] Update `.tosd` + docs; commit; resolve PR thread
