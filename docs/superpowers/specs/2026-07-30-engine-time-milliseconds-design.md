# Engine time: seconds → milliseconds

**Issue:** [#96](https://github.com/geovannimp/rust-dj-engine/issues/96)  
**Date:** 2026-07-30  
**Status:** Approved for planning

## Goal

Convert playhead, cue, loop, seek, duration, waveform viewport, and analyzer duration fields from floating-point **seconds** (`*_secs`) to integer **milliseconds** (`*_ms`) end-to-end so wire payloads, engine APIs, library persistence, and UI share one wall-clock-style time unit and avoid float drift on transport/quantize paths.

## Decisions

| Decision | Choice |
|----------|--------|
| Approach | Integer ms end-to-end (one migration PR) |
| Public / wire / DB / FE / analyzer / waveform viewport type | `i32` (`*_ms`) |
| DSP playhead / active loop internals | Source frames; playhead must be **signed** so negative seeks work (today’s `u64 position` is insufficient) |
| Beat snap / sync math | Local `f64` seconds only inside helpers; enter/exit as ms |
| Seek / playhead clamping | **None** — negative and past-end positions allowed |
| Negative cues | Allowed; recall seeks to the exact negative `ms` |
| Dual wire fields / compat shims | None |
| Existing user DB rows | Discard / recreate OK (app not public); SeaORM schema sync |

## Units & conversion

| Layer | Unit | Type |
|-------|------|------|
| Wire / `engine-api` / FE Zustand / library DB / analyzer results / waveform viewport | milliseconds | `i32` |
| DSP playhead | source frames, signed | e.g. `position_frac: f64` (may be negative); integer cursor `i64` or derive from frac |
| DSP active loop region | source frames | sample bounds (typically ≥ 0) |
| Beat snap / sync helpers | local seconds | `f64` at helper boundary only |

Shared helpers (place where both engine and library can reuse, or duplicate one-liners if a shared crate is overkill):

```text
secs_to_ms(secs: f64) -> i32   // round
ms_to_secs(ms: i32) -> f64     // ms as f64 / 1000.0
```

### Semantics

- **Seek / playhead:** do not clamp to `[0, duration]`. Negative and past-end positions are valid so pre-start cues work.
- **Signed playhead in DSP:** migrate away from unsigned-only playhead storage so `seek_ms(-500)` is representable. Audio readout for `position < 0` yields silence until the playhead crosses 0; past EOF yields silence / end behavior (no forced snap to 0).
- **Cue storage:** `Option<i32>` may be negative; stop clamping cue storage to ≥0.
- **Cue recall:** seek to the stored `ms` (may be negative).
- **Loop set:** error if `out_ms <= in_ms` (unchanged rule).
- **Unloaded deck:** `position_ms` / `duration_ms` remain `None` on snapshots.
- **Rounding:** `round` when converting residual float seconds → ms (e.g. import paths).

## Architecture

```text
FE (Zustand / wire.ts)  --msgpack *_ms:i32-->  engine-api
                                              |
                                         engine-core control
                                              |
                                         Engine::*(_ms)
                                              |
                                    engine-dsp Deck::*(_ms)
                                         |            |
                                    frames seek    cue: Option<i32>
                                    loop: frames   snap_ms ↔ local secs
```

Library hot-cue / loop / track duration persistence uses `*_ms` in SeaORM entities and `deck_data`. Hosts that pass cue/loop positions into bus commands use ms only.

### Rename map (representative)

| Today | After |
|-------|--------|
| `position_secs`, `duration_secs`, `cue_point_secs` | `position_ms`, `duration_ms`, `cue_point_ms` |
| `in_secs` / `out_secs` | `in_ms` / `out_ms` |
| `seek_secs` / `deck_playback_secs` / `snap_secs` | `seek_ms` / `deck_playback_ms` / `snap_ms` |
| DSP `position_seconds()` | `position_ms()` → `Option<i32>` |
| Sampler `duration_secs` | `duration_ms` |
| Waveform `visible_secs`, `center_secs`, `cover_*_secs`, … | `*_ms` |
| Analyzer `duration_analyzed_secs`, `max_duration_secs`, … | `*_ms` |
| Track / cue / loop DB columns | `duration_ms`, `position_ms`, `in_ms`, `out_ms` |

## File scope (one PR)

1. `crates/engine-api` — payloads + msgpack / postcard / golden tests  
2. `crates/engine-dsp` — Deck time APIs + callers; remove seek/cue ≥0 clamps that block negative positions  
3. `crates/engine-core` — `engine`, `control`, `sync` + bus behavior tests  
4. `crates/library` + `library-core` — duration/cue/loop types + entities (schema sync)  
5. `crates/analyzer` / `analyzer-core` — duration fields and tests  
6. `apps/gui-app` — wire codecs, stores, pads, waveform, Tauri status / waveform / deck_performance  
7. Docs that name wire `*_secs` fields (e.g. `docs/deck-spec.md` snippets) — update in the same change set where they would otherwise lie

### Out of scope

- Changing beat/BPM algorithms beyond converting at helper boundaries  
- Compatibility shims or dual `*_secs` + `*_ms` on the wire  
- Preserving old SQLite column data across the rename

## Testing

- `engine-api` roundtrip / golden fixtures use `*_ms`  
- `engine-core` bus tests: seek, cue, auto-loop, hot-cue recall, pause-preserves-position, unload, pads/saved loops  
- FE `wire` + `applyBusEvent` tests use ms literals (e.g. `12250` not `12.25`)  
- DSP/unit: conversion helpers round-trip; negative cue stored and recalled seeks to that negative position (playhead not forced to 0)  
- Library tests after column rename  
- Analyzer tests for renamed duration fields  

## Acceptance criteria

- [ ] Public engine cmd/evt and snapshot fields that represent media position use `i32` milliseconds  
- [ ] No remaining playhead/cue/loop/seek APIs named `*_secs` on `engine-api`, engine-core transport, or public DSP Deck time surface  
- [ ] Library track/cue/loop duration and position columns are `*_ms`  
- [ ] Waveform viewport and analyzer duration fields are `*_ms`  
- [ ] Bus/golden/msgpack fixtures and FE wire codecs updated in the same change set  
- [ ] Existing bus behavior tests still pass  
- [ ] Seek and playhead do not clamp; negative cue recall works  
