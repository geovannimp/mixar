# PlatterMotion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace ad-hoc `Deck` jog math with a first-party `PlatterMotion` rate authority (fixed-dt vinyl filter, paused scrub, envelope stub) and make the UI platter follow the playhead during hardware scratch.

**Architecture:** New `crates/engine-dsp/src/platter_motion.rs` owns vinyl/bend/envelope state. `Deck` keeps `jog_*` public API but delegates. Flutter `JogPlatter` syncs from playhead whenever not locally dragging.

**Tech Stack:** Rust `engine-dsp` unit tests; Flutter widget tests optional for rotation sync.

**Spec:** `docs/platter-motion-design.md`

## Global Constraints

- Cargo: `cargo --manifest-path crates/Cargo.toml …`
- `engine-dsp` stays pure (no I/O).
- Do not copy Mixxx/xwax source files; reimplement the standard alpha–beta math.
- Keep public `Deck::{set_jog_touch,jog_turn,set_jog_mode,jog_rate,jog_touching,jog_driving_audio}` signatures.
- Owner preference: continue without per-section review; inline execution.

## File map

| File | Role |
|------|------|
| `crates/engine-dsp/src/platter_motion.rs` | Motion authority |
| `crates/engine-dsp/src/lib.rs` | `mod platter_motion` |
| `crates/engine-dsp/src/deck.rs` | Delegate jog fields/methods |
| `apps/gui-flutter/lib/mixer/deck_jog.dart` | Playhead follow while hardware touching |
| `docs/platter-motion-design.md` | Spec |
| `docs/deck-spec.md` | Point §5.11 at PlatterMotion |

---

### Task 1: `PlatterMotion` module + unit tests

**Files:**
- Create: `crates/engine-dsp/src/platter_motion.rs`
- Modify: `crates/engine-dsp/src/lib.rs`
- Test: unit tests inside `platter_motion.rs`

**Interfaces:**
- Produces: `MotionMode`, `PlatterMotion` with `set_touch`, `set_modes`, `add_ticks`, `set_envelope_rate`, `tick`, `rate`, `touching`, `driving`.

- [ ] **Step 1: Write failing tests** for: fixed-dt vinyl rate from steady ticks ≈ 1.0 at vinyl speed; paused touch idle → rate 0 and `driving`; envelope override; paused outer ticks produce absolute rate (not ~1.0 bend); playing pitch-bend decays toward 1.0.

- [ ] **Step 2: Implement `platter_motion.rs`** with α=1/8, β=α/32, 720 intervals/rev, 33⅓ rpm, 1 ms quantum, envelope override, paused-outer→vinyl scrub.

- [ ] **Step 3: Export module** from `lib.rs`.

- [ ] **Step 4: Run** `cargo --manifest-path crates/Cargo.toml test -p engine-dsp platter_motion`. Expected: PASS.

---

### Task 2: Wire `Deck` to `PlatterMotion`

**Files:**
- Modify: `crates/engine-dsp/src/deck.rs`

**Interfaces:**
- Consumes: `PlatterMotion`
- Keeps: existing public jog methods

- [ ] **Step 1: Replace jog fields** with `motion: PlatterMotion` (or keep thin wrappers). Map `JogMode` ↔ `MotionMode`.

- [ ] **Step 2: Update existing deck jog tests** that assumed buffer-dt vinyl / paused pitch-bend; add `paused_outer_jog_scrubs_not_pitch_bend`.

- [ ] **Step 3: Run** `cargo --manifest-path crates/Cargo.toml test -p engine-dsp`. Expected: PASS.

---

### Task 3: UI platter follows playhead during hardware touch

**Files:**
- Modify: `apps/gui-flutter/lib/mixer/deck_jog.dart`
- Test: extend `apps/gui-flutter/test/deck_jog_test.dart` if present, else add focused test

- [ ] **Step 1: Change `_syncRotationFromPlayhead`** so only `_dragging` skips sync (not `widget.jogTouching`).

- [ ] **Step 2: Run** `cd apps/gui-flutter && flutter test test/deck_jog_test.dart` (or create it). Expected: PASS.

---

### Task 4: Docs + verification

**Files:**
- Modify: `docs/deck-spec.md` §5.11
- Keep: `docs/platter-motion-design.md`

- [ ] **Step 1: Note PlatterMotion** in deck-spec.
- [ ] **Step 2: Full check** `cargo --manifest-path crates/Cargo.toml test -p engine-dsp` and Flutter jog tests.

## Spec coverage

| Spec item | Task |
|-----------|------|
| Fixed-dt vinyl filter | 1 |
| Paused outer scrub | 1–2 |
| Envelope stub | 1 |
| Deck delegation | 2 |
| UI follow playhead | 3 |
| Docs | 4 |
