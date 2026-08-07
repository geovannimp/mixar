# Control value normalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Absolute controls speak `0..1` on cmds/events/runtime; DSP conversion only at the engine edge; mapping `invert` for HW (DDJ-400 tempo).

**Architecture:** Hard-break `engine-api` field names/semantics; engine stores norms and maps on set/get to DSP; controller drops unit conversion and applies binding `invert`; FE store mirrors norms and converts at display.

**Tech Stack:** Rust `engine-api` / `engine-core` / `controller`; Tauri host; Zod wire in `gui-app`.

**Spec:** `docs/superpowers/specs/2026-08-07-control-value-normalization-design.md`  
**Issue:** [#138](https://github.com/geovannimp/rust-dj-engine/issues/138)

## Global Constraints

- Wire/runtime absolute controls are `0..1` fader position (center `0.5` = 0 dB / 0% pitch).
- `auto_gain_db` / LUFS / BPM / ms stay physical.
- Soft-takeover compares norms only (threshold `3/127`).
- Strip map: `db = (norm - 0.5) * 48` clamped ±24; inverse for snapshot.
- Tempo map: position `0..1` → speed ratio `1.16 - norm * 0.32` (same as former controller curve; HW invert is mapping-side only).
- Cargo: `cargo --manifest-path crates/Cargo.toml …`

---

### Task 1: engine-api wire rename + semantics

**Files:**
- Modify: `crates/engine-api/src/payload.rs`
- Modify: FE/tests that encode golden bodies as needed later
- Test: `crates/engine-api/tests/msgpack_roundtrip.rs` (extend absolute body if present)

- [ ] Rename `SetFilter.filter_db` → `filter`, `SetGainTrim.gain_db` → `gain_trim`, `SetEqBand.gain_db` → `gain`
- [ ] Rename snapshot/`DeckUpdated` `filter_db` → `filter`, `gain_trim_db` → `gain_trim`
- [ ] Document in comments that `eq.*`, `speed`, `filter`, `gain_trim` are `0..1`
- [ ] `cargo test -p engine-api` green (update fixtures)

### Task 2: engine-core store norms + DSP edge + soft-takeover

**Files:**
- Modify: `crates/engine-core/src/soft_takeover.rs` (delete unit helpers)
- Modify: `crates/engine-core/src/control.rs`, `engine.rs`
- Add helpers: `norm_to_strip_db` / `strip_db_to_norm` / `norm_to_speed` / `speed_to_norm` in one engine module (DSP edge only)

- [ ] Strip norms stored; setters from cmds take `0..1` and convert when calling DSP
- [ ] Snapshots emit norms
- [ ] Soft-takeover compares without unit conversion
- [ ] Sync paths that set ratio update stored speed position via inverse map
- [ ] `cargo test -p engine-core --no-default-features` (or full) for affected bus tests

### Task 3: controller invert + norm-only publish

**Files:**
- Modify: `crates/controller/src/map_file.rs` (`invert: Option<bool>`)
- Modify: `crates/controller/src/action.rs` (publish norms; delete norm→db/speed)
- Modify: `crates/controller/src/session.rs` (apply invert before resolve)
- Modify: `mappings/ddj-400/map.toml` (tempo `invert = true`)
- Update schemas/`.tosd` if bindings document soft_takeover

- [ ] Binding `invert` default false; applied to CC norm before `resolve_action`
- [ ] Actions pass `norm` straight into cmd bodies
- [ ] Tests for invert + eq/filter/speed cmds
- [ ] `cargo test -p controller`

### Task 4: FE wire + store + UI display

**Files:**
- Modify: `apps/gui-app/src/lib/engine/wire.ts`, types, apply-bus-event, engine-store
- Modify: mixer/tempo components + `format.ts` helpers for norm↔display
- Tests: wire / apply-bus-event / format

- [ ] Zod schemas match Rust renames; values treated as `0..1`
- [ ] Pitch slider uses position directly; format pitch % via norm→ratio for labels
- [ ] EQ/filter/gain knobs: UI may keep dB labels but publish norms
- [ ] `npx vitest run` for affected tests

### Task 5: Verify + PR

- [ ] Full affected tests
- [ ] Push branch; open PR linked to #138
