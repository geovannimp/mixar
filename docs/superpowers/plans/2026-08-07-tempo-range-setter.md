# Tempo Range Setter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Configurable per-deck `tempo_range` as pitch fraction (±6/10/16/25%, default ±6%), with GUI cycle control, DDJ-400 Shift+SYNC mapping, and percent pitch readout.

**Architecture:** Hard-break DSP/wire from BPM half-span to percent fraction; `SetTempoRange` cmd + field on deck snapshots/events; controller cycles shared step list; GUI mirrors engine and cycles the same steps.

**Tech Stack:** Rust (`engine-dsp`, `engine-api`, `engine-core`, `controller`), TypeScript/React (`gui-app`), MessagePack bus, DDJ-400 TOML mapping.

## Global Constraints

- Wire/DSP unit is fraction (`0.06` = ±6%), never BPM half-span.
- Default `0.06`; cycle steps `[0.06, 0.10, 0.16, 0.25]`.
- Range change keeps `speed`; does not clear `ratio_override` (remap `speed` if override set).
- Soft-takeover unchanged; no range LED work.
- Work in `.worktrees/feat-tempo-range-setter` on `feat/tempo-range-setter`.
- Run cargo via `cargo --manifest-path crates/Cargo.toml …`.

## File map

| File | Role |
|------|------|
| `crates/engine-dsp/src/tempo.rs` | Percent fader ↔ ratio math + step consts |
| `crates/engine-dsp/src/deck.rs` | Default range; `set_tempo_range` sync-safe |
| `crates/engine-api/src/kind.rs` | `SetTempoRange` |
| `crates/engine-api/src/payload.rs` | Cmd + snapshot/event field |
| `crates/engine-core/src/control.rs` | Dispatch + publish |
| `crates/engine-core/src/engine.rs` | `set_deck_tempo_range` + snapshot field |
| `crates/controller/src/catalog.rs` + `action.rs` | `cycle_tempo_range` |
| `mappings/ddj-400/device.toml` + `map.toml` | Note `0x60` bind |
| `apps/gui-app/src/lib/format.ts` + `wire.ts` + types + tempo panel | Percent UI + cycle |

---

### Task 1: DSP percent tempo math

**Files:**
- Modify: `crates/engine-dsp/src/tempo.rs`
- Modify: `crates/engine-dsp/src/deck.rs`

**Interfaces:**
- Produces: `DEFAULT_TEMPO_RANGE = 0.06`, `TEMPO_RANGE_STEPS: &[f32]`, `next_tempo_range(current) -> f32`, `norm_to_playback_ratio(norm, tempo_range)`, `playback_ratio_to_norm(ratio, tempo_range)` (BPM args removed from span math)

- [ ] **Step 1: Rewrite `tempo.rs` for percent**

```rust
//! Tempo fader `0..1` ↔ playback ratio using ±`tempo_range` (fraction of rate).

pub const DEFAULT_TEMPO_RANGE: f32 = 0.06;
pub const TEMPO_RANGE_STEPS: &[f32] = &[0.06, 0.10, 0.16, 0.25];

pub fn next_tempo_range(current: f32) -> f32 {
    let eps = 1e-4;
    if let Some(i) = TEMPO_RANGE_STEPS.iter().position(|s| (s - current).abs() < eps) {
        return TEMPO_RANGE_STEPS[(i + 1) % TEMPO_RANGE_STEPS.len()];
    }
    TEMPO_RANGE_STEPS[0]
}

pub fn norm_to_playback_ratio(norm: f32, tempo_range: f32) -> f32 {
    let range = f64::from(tempo_range.max(0.0));
    let n = f64::from(norm.clamp(0.0, 1.0));
    (1.0 + (0.5 - n) * 2.0 * range).max(0.01) as f32
}

pub fn playback_ratio_to_norm(ratio: f32, tempo_range: f32) -> f32 {
    let range = f64::from(tempo_range.max(1e-6));
    let n = 0.5 - (f64::from(ratio) - 1.0) / (2.0 * range);
    n.clamp(0.0, 1.0) as f32
}
```

Update tests: center → 1.0; norm 0 with 0.06 → 1.06; next from 0.06 → 0.10 → … → 0.06.

- [ ] **Step 2: Update `Deck`**

- Default `tempo_range: DEFAULT_TEMPO_RANGE`.
- `playback_ratio` / `set_playback_ratio` call new signatures (drop track_bpm from tempo helpers).
- `set_tempo_range`: set range; if `ratio_override` is `Some(r)`, set `speed = playback_ratio_to_norm(r, tempo_range)`; else leave speed; **do not** clear override.

- [ ] **Step 3: Test**

```bash
cargo --manifest-path crates/Cargo.toml test -p engine-dsp tempo -- --nocapture
```

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-dsp/src/tempo.rs crates/engine-dsp/src/deck.rs
git commit -m "feat(engine-dsp): tempo_range as pitch percent fraction"
```

---

### Task 2: Wire + engine cmd

**Files:**
- Modify: `crates/engine-api/src/kind.rs`, `payload.rs`
- Modify: `crates/engine-core/src/control.rs`, `engine.rs`
- Test: add/extend `crates/engine-core/tests/bus_sync_speed.rs` or new `bus_tempo_range.rs`

- [ ] **Step 1: API**

Add `SetTempoRange` to `Kind`. Add to `CmdBody`:

```rust
SetTempoRange { tempo_range: f32 },
```

Add `tempo_range: f32` to `DeckSnapshot` and `EvtBody::DeckUpdated`.

- [ ] **Step 2: Engine**

`Engine::set_deck_tempo_range(deck_id, range) -> Result<Vec<usize>>` — validate `range.is_finite() && range > 0`, call deck setter, if this deck is master re-apply tempo sync to slaves (ratios may need remapped speed display), return updated deck ids.

Wire dispatch in `control.rs` like other discrete deck cmds; match arm for decode; publish via existing `DeckUpdated` path including new field in `deck_snapshot_to_evt` / `deck_snapshot_from_dsp`.

- [ ] **Step 3: Integration test**

Publish `SetTempoRange { tempo_range: 0.10 }`, expect `DeckUpdated` with `tempo_range ≈ 0.10` and unchanged `speed` when not synced.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(engine): SetTempoRange cmd and tempo_range on DeckUpdated"
```

---

### Task 3: Controller cycle + DDJ-400

**Files:**
- Modify: `crates/controller/src/catalog.rs`, `action.rs`, `action_id` if needed
- Modify: `mappings/ddj-400/device.toml`, `map.toml`
- Snapshot: store `tempo_range: [f32; 4]` on control snapshot if needed for cycle

- [ ] **Step 1:** Add deck alias `tempo_range` and action leaf `cycle_tempo_range`.

On press: `next = engine_dsp::tempo::next_tempo_range(snap.tempo_range[deck])` then `SetTempoRange`. Update snapshot from events.

- [ ] **Step 2:** DDJ-400 device note `0x60` on deck channels 1/2 as `tempo_range`; map to `Deck(_)::cycle_tempo_range`.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(controller): cycle_tempo_range + DDJ-400 Shift+SYNC"
```

---

### Task 4: GUI percent + cycle

**Files:**
- Modify: `apps/gui-app/src/lib/format.ts`, `wire.ts`, `types.ts`, `apply-bus-event.ts`, `deck-tempo-panel.tsx`, deck parent that sends cmds

- [ ] **Step 1:** `DEFAULT_TEMPO_RANGE = 0.06`, `TEMPO_RANGE_STEPS`, `nextTempoRange`, percent-based `normToSpeedRatio` / `effectiveBpm` / use `formatPitchPercent` in panel; remove BPM-offset display.

- [ ] **Step 2:** Wire schema + `DeckStatus.tempo_range`; cycle button publishes `set_tempo_range`.

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(gui): tempo range cycle and percent pitch readout"
```

---

### Task 5: Verify + PR

- [ ] Run `cargo --manifest-path crates/Cargo.toml test -p engine-dsp -p engine-core -p controller`
- [ ] Typecheck gui if feasible (`npx moon run gui-app:typecheck` or package script)
- [ ] Push branch and `gh pr create` linking #140
