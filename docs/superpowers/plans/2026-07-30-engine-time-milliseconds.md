# Engine Time → Milliseconds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert media/viewport/analyzer times from `f64` seconds (`*_secs`) to `i32` milliseconds (`*_ms`) end-to-end per issue #96 and `docs/superpowers/specs/2026-07-30-engine-time-milliseconds-design.md`.

**Architecture:** Public wire, library DB, FE, analyzer, and waveform viewport use `i32` ms. DSP keeps a **signed** source-frame playhead (`position_frac: f64`, integer cursor `i64`) with **no** seek clamp. Beat snap/sync convert to local `f64` seconds only inside helpers. Shared Rust conversion helpers live in `audio-core`.

**Tech Stack:** Rust workspace (`audio-core`, `engine-api`, `engine-dsp`, `engine-core`, `library*`, `analyzer*`), MessagePack wire, Tauri + React/Zustand, SeaORM sync SQLite.

**Spec:** `docs/superpowers/specs/2026-07-30-engine-time-milliseconds-design.md`

## Global Constraints

- Media times on the wire / public APIs / DB / FE / analyzer / waveform viewport: `i32` named `*_ms`.
- No dual `*_secs` + `*_ms` fields; no compat shims.
- Seek and playhead: **do not clamp**; negative and past-end allowed.
- Negative cue storage and recall seek to the exact `ms`.
- Loop set still errors if `out_ms <= in_ms`.
- Old SQLite rows: discard OK (schema sync / recreate).
- Run Cargo via `cargo --manifest-path crates/Cargo.toml …` (or `cd crates`). Prefer `cargo test -p <crate>` while dependents are mid-rename; full workspace / FE checks after Task 7.
- Ponytail: smallest working diff; one runnable check per non-trivial unit.
- Frontend: keep imports at top; exhaustive `switch`/`never` where applicable.

---

## File map

| File | Responsibility |
|------|----------------|
| `crates/audio-core/src/time.rs` (new) | `secs_to_ms` / `ms_to_secs` |
| `crates/audio-core/src/waveform/mod.rs` | `waveform_buckets_for_duration` takes ms |
| `crates/engine-api/src/payload.rs` | Rename all media time fields to `*_ms: i32` |
| `crates/engine-api/tests/*` | Roundtrip / goldens use ms |
| `crates/engine-dsp/src/deck.rs` | Signed playhead; `seek_ms` / `position_ms` / cue/loop ms APIs; silence when `position_frac < 0` |
| `crates/engine-core/src/sync.rs` | `snap_ms`; beat-align in/out ms |
| `crates/engine-core/src/engine.rs` | Public `*_ms` methods; snapshot fields |
| `crates/engine-core/src/control.rs` | Decode/encode ms payloads |
| `crates/engine-core/tests/bus_*.rs` | Assert ms |
| `crates/library-core/src/types.rs` | `TrackMetadata.duration_ms` |
| `crates/library/src/entity/{tracks,track_hot_cue,track_loop}.rs` | Column rename to `*_ms` |
| `crates/library/src/deck_data.rs` (+ store/tags/lib) | Persist/load ms |
| `crates/analyzer-core/src/{config,result}.rs` | Duration limits / analyzed duration in ms |
| `apps/gui-app/src/lib/engine/wire.ts` | Zod + codecs `*_ms` |
| `apps/gui-app/src/{types,stores,components}/**` | Rename consumers |
| `apps/gui-app/src-tauri/src/**` | Status / waveform / performance / cache use ms |
| `docs/deck-spec.md` | Document `*_ms` |

---

### Task 1: `audio-core` time helpers + waveform buckets

**Files:**
- Create: `crates/audio-core/src/time.rs`
- Modify: `crates/audio-core/src/lib.rs` (mod + re-export)
- Modify: `crates/audio-core/src/waveform/mod.rs`
- Test: unit tests in `time.rs`

**Interfaces:**
- Produces:
  - `pub fn secs_to_ms(secs: f64) -> i32` — `(secs * 1000.0).round() as i32` (non-finite → `0`)
  - `pub fn ms_to_secs(ms: i32) -> f64` — `f64::from(ms) / 1000.0`
  - `pub fn waveform_buckets_for_duration(duration_ms: i32) -> usize`

- [ ] **Step 1: Write failing tests for conversion helpers**

Add to a new `time.rs` (tests first via `#[cfg(test)]`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secs_to_ms_rounds_and_negatives() {
        assert_eq!(secs_to_ms(12.5), 12_500);
        assert_eq!(secs_to_ms(-0.5), -500);
        assert_eq!(secs_to_ms(0.0004), 0);
        assert_eq!(secs_to_ms(0.0006), 1);
    }

    #[test]
    fn ms_to_secs_roundtrip_center() {
        assert!((ms_to_secs(12_500) - 12.5).abs() < f64::EPSILON);
        assert!((ms_to_secs(-500) + 0.5).abs() < f64::EPSILON);
    }
}
```

- [ ] **Step 2: Implement helpers and export**

```rust
//! Media time conversions (seconds ↔ milliseconds).

/// Convert floating seconds to integer milliseconds (rounded).
pub fn secs_to_ms(secs: f64) -> i32 {
    if !secs.is_finite() {
        return 0;
    }
    (secs * 1000.0).round() as i32
}

/// Convert integer milliseconds to floating seconds.
pub fn ms_to_secs(ms: i32) -> f64 {
    f64::from(ms) / 1000.0
}
```

In `lib.rs`: `pub mod time;` and `pub use time::{ms_to_secs, secs_to_ms};`.

- [ ] **Step 3: Rename `waveform_buckets_for_duration` to take ms**

```rust
pub fn waveform_buckets_for_duration(duration_ms: i32) -> usize {
    if duration_ms <= 0 {
        return 0;
    }
    let buckets = (duration_ms as f64 / WAVEFORM_MS_PER_BUCKET as f64).ceil() as usize;
    buckets.max(1)
}
```

Update all Rust call sites that passed seconds (grep `waveform_buckets_for_duration`) to pass `secs_to_ms(...)` or an existing ms value.

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p audio-core time::tests -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/audio-core
git commit -m "$(cat <<'EOF'
feat(audio-core): add secs↔ms helpers and duration_ms waveform buckets

EOF
)"
```

---

### Task 2: `engine-api` payloads → `*_ms: i32`

**Files:**
- Modify: `crates/engine-api/src/payload.rs`
- Modify: `crates/engine-api/tests/msgpack_roundtrip.rs`
- Modify: `crates/engine-api/tests/postcard_roundtrip.rs` (if present and uses secs)
- Note: dependents will not compile until Tasks 3–4; verify with `-p engine-api` only

**Interfaces:**
- Produces (field renames; all times `i32` unless `Option`):
  - `LoopRegion { in_ms, out_ms, active }`
  - `DeckHotCue { position_ms, ... }`
  - `DeckSavedLoop { in_ms, out_ms, ... }`
  - `SamplerSlotInfo { duration_ms: Option<i32>, ... }`
  - `DeckSnapshot` / `EvtBody::DeckUpdated`: `cue_point_ms`, `position_ms`, `duration_ms`
  - `CmdBody::Seek { position_ms }`
  - `CmdBody::TriggerHotCue { position_ms }`
  - `CmdBody::RecallSavedLoop { in_ms, out_ms }`
  - `EvtBody::Position { position_ms }`

- [ ] **Step 1: Update roundtrip test to expect ms (failing compile/assert)**

```rust
let seek = CmdBody::Seek {
    position_ms: 12_500,
};
```

- [ ] **Step 2: Rename every media time field in `payload.rs`**

Mechanical rename: `*_secs: f64` / `Option<f64>` → `*_ms: i32` / `Option<i32>` for the fields listed above. Do **not** leave serde aliases.

- [ ] **Step 3: Run `engine-api` tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p engine-api`  
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/engine-api
git commit -m "$(cat <<'EOF'
feat(engine-api): represent media times as i32 milliseconds

EOF
)"
```

---

### Task 3: `engine-dsp` signed playhead + ms Deck API

**Files:**
- Modify: `crates/engine-dsp/src/deck.rs`
- Test: `#[cfg(test)]` in `deck.rs`

**Interfaces:**
- Consumes: `audio_core::{secs_to_ms, ms_to_secs}`
- Produces:
  - `position: i64` (floor of `position_frac`; may be negative)
  - `position_frac: f64` (signed source frames; source of truth)
  - `cue_point_ms: Option<i32>`
  - `cue_hold_return: Option<(i32, bool)>` (return position ms, was_playing)
  - `pub fn position_ms(&self) -> Option<i32>`
  - `pub fn cue_point_ms(&self) -> Option<i32>`
  - `pub fn loop_region_ms(&self) -> Option<(i32, i32)>`
  - `pub fn seek_ms(&mut self, ms: i32) -> Result<()>` — **no clamp**
  - `pub fn set_cue_point_ms(&mut self, ms: i32) -> Result<()>` — **no ≥0 clamp on storage**
  - `pub fn set_loop_region_ms(&mut self, in_ms: i32, out_ms: i32) -> Result<()>`
  - Remove public `*_secs` / `position_seconds` Deck time APIs

- [ ] **Step 1: Write failing tests for negative seek and cue**

```rust
#[test]
fn seek_ms_allows_negative_playhead() {
    let mut deck = Deck::new(0, ENGINE_RATE);
    // load short fixture stereo silence (reuse existing load helpers in this module)
    deck.seek_ms(-500).unwrap();
    assert_eq!(deck.position_ms(), Some(-500));
}

#[test]
fn negative_cue_recall_keeps_negative_position() {
    let mut deck = Deck::new(0, ENGINE_RATE);
    // load…
    deck.set_cue_point_ms(-250).unwrap();
    assert_eq!(deck.cue_point_ms(), Some(-250));
    deck.seek_ms(deck.cue_point_ms().unwrap()).unwrap();
    assert_eq!(deck.position_ms(), Some(-250));
}
```

(Adapt load boilerplate to the existing deck test helpers in the same file.)

- [ ] **Step 2: Change playhead storage to signed**

- Replace `position: u64` with `position: i64`.
- Update `seek(position: u64)` callers if still needed, or replace with frame seek that accepts `i64`.
- In `play_interpolated`, before sample read:

```rust
if self.position_frac < 0.0 {
    self.buffer[out * 2] = 0.0;
    self.buffer[out * 2 + 1] = 0.0;
    self.position_frac += step;
    continue;
}
```

- In `play_loaded_audio`, if `self.position_frac < 0.0`, output silence for this chunk (or advance until ≥0); never cast a negative frac to `usize` for indexing.
- Sync `self.position = self.position_frac.floor() as i64` after advances.

- [ ] **Step 3: Implement ms API (no clamp on seek)**

```rust
pub fn seek_ms(&mut self, ms: i32) -> Result<()> {
    let audio = self
        .loaded
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No track loaded"))?;
    let frames = ms_to_secs(ms) * f64::from(audio.sample_rate);
    self.position_frac = frames;
    self.position = frames.floor() as i64;
    self.reset_resampler_state();
    Ok(())
}

pub fn position_ms(&self) -> Option<i32> {
    let audio = self.loaded.as_ref()?;
    Some(secs_to_ms(self.position_frac / f64::from(audio.sample_rate)))
}

pub fn set_cue_point_ms(&mut self, ms: i32) -> Result<()> {
    self.cue_point_ms = Some(ms);
    Ok(())
}

pub fn set_loop_region_ms(&mut self, in_ms: i32, out_ms: i32) -> Result<()> {
    let audio = self
        .loaded
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No track loaded"))?;
    if out_ms <= in_ms {
        return Err(anyhow::anyhow!("Loop out must be after loop in"));
    }
    let rate = f64::from(audio.sample_rate);
    self.loop_region = Some((ms_to_secs(in_ms) * rate, ms_to_secs(out_ms) * rate));
    Ok(())
}
```

Update `begin_cue_hold` / `end_cue_hold` / `loop_region_ms` / unload resets accordingly. Delete old `*_secs` methods.

- [ ] **Step 4: Run DSP tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p engine-dsp`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/engine-dsp
git commit -m "$(cat <<'EOF'
feat(engine-dsp): signed playhead and millisecond Deck time APIs

EOF
)"
```

---

### Task 4: `engine-core` control path + bus tests

**Files:**
- Modify: `crates/engine-core/src/sync.rs`
- Modify: `crates/engine-core/src/engine.rs`
- Modify: `crates/engine-core/src/control.rs`
- Modify: `crates/engine-core/tests/bus_*.rs` (all that reference `*_secs`)

**Interfaces:**
- Consumes: DSP `*_ms` APIs; `engine_api` ms fields; `audio_core::{secs_to_ms, ms_to_secs}` for snap math
- Produces:
  - `snap_ms(ms: i32, bpm: Option<f64>, quantize: bool) -> i32`
  - `beat_align_target(...) -> i32` (positions/duration in ms)
  - `Engine::deck_playback_ms`, `seek_deck(..., position_ms: i32)`, cue/loop/hot-cue methods in ms
  - Snapshots / evt publish use `*_ms`

- [ ] **Step 1: Update one bus test to ms (example)**

In `bus_hot_cue_recall.rs`:

```rust
encode_cmd_body(&CmdBody::TriggerHotCue { position_ms: 500 }).unwrap(),
// …
assert!((pos - 500).abs() <= 1); // allow 1ms rounding if any
```

And saved loop:

```rust
in_ms: 0,
out_ms: 1000,
```

- [ ] **Step 2: Implement `snap_ms` / beat align in ms**

```rust
pub(crate) fn snap_ms(ms: i32, bpm: Option<f64>, quantize: bool) -> i32 {
    let secs = snap_secs_local(ms_to_secs(ms), bpm, quantize);
    secs_to_ms(secs)
}

fn snap_secs_local(secs: f64, bpm: Option<f64>, quantize: bool) -> f64 {
    // move existing snap_secs body here (private)
    …
}
```

Convert `beat_align_target` inputs/outputs to ms the same way (local secs inside).

- [ ] **Step 3: Wire `engine.rs` + `control.rs`**

- Rename all `position_secs` / `seek_secs` / `deck_playback_secs` / cue/loop helpers to ms.
- `LoopRegion` construction uses `in_ms` / `out_ms`.
- Position tick publishes `EvtBody::Position { position_ms }`.
- Grep `crates/engine-core` for `_secs` related to media time and clear them (leave `Duration::from_secs` alone).

- [ ] **Step 4: Fix all `bus_*.rs` tests**

Replace seconds literals with ms (`12.25` → `12250`, `0.5` → `500`, etc.). Update field names in pattern matches.

- [ ] **Step 5: Run engine-core tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p engine-core`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add crates/engine-core
git commit -m "$(cat <<'EOF'
feat(engine-core): drive transport and bus payloads in milliseconds

EOF
)"
```

---

### Task 5: Library + `library-core` persist `*_ms`

**Files:**
- Modify: `crates/library-core/src/types.rs` (`duration_ms: Option<i32>`)
- Modify: `crates/library/src/entity/tracks.rs`
- Modify: `crates/library/src/entity/track_hot_cue.rs` (`position_ms: i32`)
- Modify: `crates/library/src/entity/track_loop.rs` (`in_ms`, `out_ms`)
- Modify: `crates/library/src/deck_data.rs`
- Modify: `crates/library/src/lib.rs`, `store.rs`, `tags.rs`, `model.rs` as needed
- Grep callers of `duration_secs` / `save_hot_cue` / `save_loop` across crates and apps

**Interfaces:**
- Produces: SeaORM columns `duration_ms`, `position_ms`, `in_ms`, `out_ms` (`i32` / `Option<i32>`)
- `save_hot_cue(..., position_ms: i32, ...)`
- `save_loop(..., in_ms: i32, out_ms: i32, ...)`
- Tag import: `secs_to_ms` when lofty/analysis still yields seconds

- [ ] **Step 1: Rename entities + `TrackMetadata.duration_ms`**

Change field types to `i32` / `Option<i32>`. Update `deck_data` mapping 1:1 (no float).

- [ ] **Step 2: Fix compile errors from metadata consumers**

Where code still has seconds from decode/analysis, convert once at the boundary with `secs_to_ms`.

- [ ] **Step 3: Run library tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p library -p library-core`  
Expected: PASS (schema sync creates new columns; no data migration)

- [ ] **Step 4: Commit**

```bash
git add crates/library-core crates/library
git commit -m "$(cat <<'EOF'
feat(library): store track/cue/loop times as milliseconds

EOF
)"
```

---

### Task 6: Analyzer duration fields → ms

**Files:**
- Modify: `crates/analyzer-core/src/config.rs` (`max_duration_ms`, `resolve_max_duration_ms`)
- Modify: `crates/analyzer-core/src/result.rs` (`duration_analyzed_ms`)
- Modify: `crates/analyzer-core/src/lib.rs`, `merge.rs` test fixtures
- Modify: `crates/analyzer/src/**` call sites (`loudness.rs`, `decode.rs`, `lib.rs`)

**Interfaces:**
- Produces: `AnalysisDurationMode::resolve_max_duration_ms(self, track_duration_ms: Option<i32>) -> Option<i32>`
- Precise mode: half track duration in ms, minimum `1000` ms (was `1.0` sec)

- [ ] **Step 1: Rename + fix Precise minimum**

```rust
Self::Precise => track_duration_ms.map(|duration| (duration / 2).max(1000)),
```

Update tests that used `240.0` seconds → `240_000` ms, etc.

- [ ] **Step 2: Run analyzer tests**

Run: `cargo test --manifest-path crates/Cargo.toml -p analyzer-core -p analyzer`  
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/analyzer-core crates/analyzer
git commit -m "$(cat <<'EOF'
feat(analyzer): express analysis duration limits in milliseconds

EOF
)"
```

---

### Task 7: FE + Tauri host → `*_ms`

**Files:**
- Modify: `apps/gui-app/src/lib/engine/wire.ts`
- Modify: `apps/gui-app/src/lib/engine/applyBusEvent.ts` (+ `.test.ts`)
- Modify: `apps/gui-app/src/types.ts`
- Modify: `apps/gui-app/src/stores/engineStore.ts`, `defaultDeck.ts`, `defaultSampler.ts`, `engineStore.test.ts`
- Modify: components that read `position_secs` / loop / cue / waveform window (`DeckPanel`, `DualDeckWaveform`, pads, markers, …)
- Modify: `apps/gui-app/src-tauri/src/{lib,engine_controller,deck_performance,waveform_render,audio_cache,deck_sampler}.rs`
- Optional tiny TS helpers: `secsToMs` / `msToSecs` next to wire if UI math still has seconds momentarily — prefer native ms in UI state

**Interfaces:**
- Produces: Zod schemas and Zustand fields `*_ms: number` (integers); waveform viewport invoke payloads use `*_ms`
- Tauri `deck_playback_ms`, status structs mirrored from `engine_api`

- [ ] **Step 1: Update `wire.ts` + `applyBusEvent` tests first**

Rename schema fields; change fixtures `12.25` → `12250`, etc.

- [ ] **Step 2: Rename TS types/stores/components**

Grep `apps/gui-app` for `_secs` media/viewport fields and convert. Display formatting: `ms → mm:ss` via integer math (`ms / 1000` for whole seconds display is fine).

- [ ] **Step 3: Update Tauri Rust host**

Same renames; call `engine.deck_playback_ms`; waveform cover/center/visible use `i32` ms. Remove seek clamps if any exist in host wrappers.

- [ ] **Step 4: Run FE unit tests + typecheck**

Run (from repo root / `apps/gui-app` per package scripts):

```bash
cd apps/gui-app && npm test -- --run src/lib/engine/applyBusEvent.test.ts src/lib/engine/wire.test.ts src/stores/engineStore.test.ts
```

Also: `cargo check --manifest-path apps/gui-app/src-tauri/Cargo.toml`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/gui-app
git commit -m "$(cat <<'EOF'
feat(gui): align engine wire and UI times to milliseconds

EOF
)"
```

---

### Task 8: Docs (`deck-spec` + related)

**Files:**
- Modify: `docs/deck-spec.md` (all `*_secs` media/wire/DB examples → `*_ms`)
- Modify: other docs only where they assert current wire field names (grep `position_secs` under `docs/`)

- [ ] **Step 1: Update deck-spec field tables and SQL examples**

Example SQL:

```sql
position_ms INTEGER NOT NULL,
in_ms INTEGER NOT NULL,
out_ms INTEGER NOT NULL,
```

UI-derived remaining: `remaining_ms = duration_ms - position_ms`.

- [ ] **Step 2: Grep docs for leftover media `*_secs`**

Run: `rg 'position_secs|duration_secs|cue_point_secs|in_secs|out_secs|visible_secs' docs/`  
Expected: only historical design docs that intentionally describe the old world, or update those too if they claim to be current.

- [ ] **Step 3: Commit**

```bash
git add docs
git commit -m "$(cat <<'EOF'
docs: describe engine and library times in milliseconds

EOF
)"
```

---

### Task 9: Final verification gate

- [ ] **Step 1: Workspace Rust tests**

Run: `cargo test --manifest-path crates/Cargo.toml`  
Expected: PASS

- [ ] **Step 2: Grep for forbidden transport names**

Run:

```bash
rg 'position_secs|duration_secs|cue_point_secs|seek_secs|deck_playback_secs|snap_secs|in_secs|out_secs' \
  crates/engine-api crates/engine-core crates/engine-dsp \
  apps/gui-app/src/lib/engine apps/gui-app/src/types.ts
```

Expected: no media-time hits (ignore `Duration::from_secs`, analyzer leftovers already migrated, comments in old specs if any).

- [ ] **Step 3: Confirm negative cue path**

Manual or unit: `seek_ms(-500)` → `position_ms == Some(-500)`; hot-cue recall with negative position does not snap to 0.

- [ ] **Step 4: Final commit only if Step 1–3 left fixes**

Otherwise done.

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Wire/API `i32` ms | 2, 4, 7 |
| No `*_secs` on engine-api / core transport / DSP Deck time API | 2, 3, 4, 9 |
| Library columns `*_ms` | 5 |
| Waveform + analyzer `*_ms` | 1, 6, 7 |
| Unclamped seek; negative cue | 3, 4, 9 |
| Bus/golden/FE fixtures updated | 2, 4, 7 |
| Bus behavior tests pass | 4, 9 |
| Signed DSP playhead | 3 |

## Self-review notes

- No TBD placeholders; conversion helpers and Deck signatures are explicit.
- Types consistent: `i32` ms everywhere public; DSP frames signed via `position_frac` + `i64 position`.
- `Duration::from_secs` (timeouts) intentionally untouched.
