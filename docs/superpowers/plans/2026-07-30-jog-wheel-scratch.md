# Jog wheel / scratch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Functional top-plate jog/scratch on the bus + GUI platter, MIDI-shaped `jog_touch` / `jog_turn` / dual `JogMode` fields.

**Architecture:** Engine-owned `jog_rate` layered on deck `speed`. Shared `JogMode` (`vinyl` | `pitch_bend` | `ignore`) on `top` / `outer`. GUI platter always sends touch+turns (top policy).

**Tech Stack:** `engine-api` MessagePack, `engine-dsp` / `engine-core`, Tauri settings, Zustand, Vitest.

**Spec:** `docs/superpowers/specs/2026-07-30-jog-wheel-scratch-design.md`

## Global Constraints

- `intervals_per_rev = 720`, `rpm = 33.333…`; α/β ≈ Mixxx (`1/8`, `1/256`).
- Do not use `set_deck_speed` for vinyl stop (rejects ≤0); use `jog_rate`.
- GUI jog is always top (no outer hit zones).
- Prefer `cargo --manifest-path crates/Cargo.toml` and gui-app checks.
- Ponytail: fewest files; reuse `publish_deck_updated` / `publishCmd`.

## File map

| File | Role |
|------|------|
| `crates/engine-api/src/kind.rs` | `JogTouch`, `JogTurn`, `SetJogMode` |
| `crates/engine-api/src/payload.rs` | `JogMode`, cmd bodies, deck snapshot/evt fields |
| `crates/engine-dsp/src/deck.rs` | Jog state + `jog_rate` in step |
| `crates/engine-core/src/control.rs` | Dispatch |
| `crates/engine-core/src/engine.rs` | Engine helpers + defaults from config |
| `apps/gui-app/src/lib/engine/wire.ts` | Wire mirror |
| `apps/gui-app/src/stores/engineStore.ts` | Actions |
| `apps/gui-app/src/types.ts` / `defaultDeck.ts` | Status + settings fields |
| `apps/gui-app/src/components/DeckTransport.tsx` | Pointer → cmds |
| `apps/gui-app/src-tauri/src/lib.rs` | AppSettings defaults |
| Settings UI + `busSettings.ts` | Default mode selects |
| `docs/deck-spec.md` | Note functional jog |

---

### Task 1: API types

**Files:** `crates/engine-api/src/{kind,payload,lib}.rs`, tests

- [ ] Add `JogMode` enum + `JogTouch` / `JogTurn` / `SetJogMode` kinds and bodies
- [ ] Add `top_jog_mode`, `outer_jog_mode`, `jog_touching` to `DeckSnapshot` and `DeckUpdated`
- [ ] Roundtrip test; `cargo test -p engine-api`
- [ ] Commit

### Task 2: DSP jog_rate

**Files:** `crates/engine-dsp/src/deck.rs`

- [ ] Add jog fields + `set_jog_touch` / `jog_turn` / `set_jog_mode` / `tick_jog` (ramp/bend decay per process block)
- [ ] `effective_speed = speed * jog_rate` in `play_interpolated` (and any other step path)
- [ ] Unit tests: vinyl fwd/back/hold, bend decay, ignore, top vs outer via touching
- [ ] Commit

### Task 3: Core dispatch + status

**Files:** `crates/engine-core/src/{control,engine}.rs` (+ snapshot mapping)

- [ ] Dispatch three cmds; include jog fields in snapshots/events
- [ ] Init modes from engine config / defaults
- [ ] Bus smoke test optional; `cargo test -p engine-dsp -p engine-core`
- [ ] Commit

### Task 4: Settings defaults

**Files:** Tauri `AppSettings`, TS `AppSettings` / `normalizeAppSettings`, settings panel

- [ ] `default_top_jog_mode` / `default_outer_jog_mode`; wire into engine start if config path exists, else apply when creating engine session
- [ ] Commit

### Task 5: GUI wire + store + platter

**Files:** `wire.ts`, `engineStore.ts`, `types.ts`, `defaultDeck.ts`, `DeckTransport.tsx`, `DeckPanel.tsx`

- [ ] Wire kinds/bodies; store `jogTouch` / `jogTurn` / `setJogMode`
- [ ] Pointer handlers on `JogPlatter` (always touch); tick from angular delta
- [ ] Vitest for tick helper and/or store publish
- [ ] Commit

### Task 6: Docs + PR

- [ ] Update `deck-spec.md` §5.11 current-state note
- [ ] Mark design status implemented bits; open PR linking #43
