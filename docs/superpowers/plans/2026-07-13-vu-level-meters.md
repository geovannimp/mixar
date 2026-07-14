# VU / Level Meters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship per-deck pre-fader stereo peak + peak-hold meters between the mixer volume faders, with UI mono/stereo display config.

**Architecture:** Measure peak_l/peak_r in `Deck::process` after filter / before volume; expose via `Engine::deck_level_snapshot()` (same mutex pattern as playback snapshot); `EngineNotifier` applies peak-hold and emits `EngineEvent::Levels` at ~30 Hz; React store + `LevelMeter` ladders in the mixer center strip.

**Tech Stack:** Rust (`engine-dsp`, `engine-core`), Tauri event bus, React/Zustand (`gui-app`).

**Spec:** `docs/superpowers/specs/2026-07-13-vu-level-meters-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `engine-dsp/src/level_meter.rs` | Peak detector helper + unit tests |
| `engine-dsp/src/lib.rs` | Module export |
| `engine-dsp/src/deck.rs` | Call meter pre-volume; store/expose last peaks |
| `engine-core/src/engine.rs` | `deck_level_snapshot()` |
| `gui-app/src-tauri/src/engine_events.rs` | `EngineEvent::Levels` + emit + serde test |
| `gui-app/src-tauri/src/engine_notifier.rs` | Read levels, peak-hold decay, emit |
| `gui-app/src/types.ts` | Level fields + `LevelMeterMode` |
| `gui-app/src/lib/engineEvents.ts` | Handle `levels` event |
| `gui-app/src/stores/defaultDeck.ts` | Zero default levels |
| `gui-app/src/stores/engineStore.ts` | Patch levels + mode preference |
| `gui-app/src/components/LevelMeter.tsx` | LED ladder UI |
| `gui-app/src/components/DeckMixer.tsx` | Place meters between faders |

---

### Task 1: DSP level meter helper (TDD)

**Files:**
- Create: `engine-dsp/src/level_meter.rs`
- Modify: `engine-dsp/src/lib.rs`

- [ ] **Step 1: Write failing tests in `level_meter.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_from_interleaved_stereo_buffer() {
        // L R L R … → peak_l=0.5, peak_r=0.8
        let buf = [0.1, -0.8, 0.5, 0.2, -0.3, 0.4];
        let (peak_l, peak_r) = measure_stereo_peaks(&buf);
        assert!((peak_l - 0.5).abs() < 1e-6);
        assert!((peak_r - 0.8).abs() < 1e-6);
    }

    #[test]
    fn empty_buffer_is_zero() {
        assert_eq!(measure_stereo_peaks(&[]), (0.0, 0.0));
    }
}
```

- [ ] **Step 2: Run tests — expect compile/link failure**

Run: `cargo test -p engine-dsp measure_stereo_peaks -- --nocapture`  
Expected: FAIL (module/`measure_stereo_peaks` missing)

- [ ] **Step 3: Implement helper**

```rust
//! Pre-fader peak detection for deck VU meters.

/// Measure absolute peak L/R from interleaved stereo samples.
pub fn measure_stereo_peaks(interleaved: &[f32]) -> (f32, f32) {
    let mut peak_l = 0.0f32;
    let mut peak_r = 0.0f32;
    let mut i = 0;
    while i + 1 < interleaved.len() {
        peak_l = peak_l.max(interleaved[i].abs());
        peak_r = peak_r.max(interleaved[i + 1].abs());
        i += 2;
    }
    (peak_l, peak_r)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LevelPeaks {
    pub peak_l: f32,
    pub peak_r: f32,
}

impl LevelPeaks {
    pub fn from_buffer(interleaved: &[f32]) -> Self {
        let (peak_l, peak_r) = measure_stereo_peaks(interleaved);
        Self { peak_l, peak_r }
    }
}
```

Add to `engine-dsp/src/lib.rs`:

```rust
pub mod level_meter;
pub use level_meter::{measure_stereo_peaks, LevelPeaks};
```

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p engine-dsp measure_stereo_peaks -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine-dsp/src/level_meter.rs engine-dsp/src/lib.rs
git commit -m "Add stereo peak helper for deck VU meters."
```

---

### Task 2: Wire meter into `Deck::process` (pre-volume)

**Files:**
- Modify: `engine-dsp/src/deck.rs`
- Test: add tests in `engine-dsp/src/deck.rs` or `level_meter.rs`

- [ ] **Step 1: Write failing deck-level test**

Add to `engine-dsp/src/deck.rs` `#[cfg(test)]` module. Reuse the same `LoadedAudio` construction as existing deck tests (constant stereo frames). Shape:

```rust
#[test]
fn levels_measure_pre_volume() {
    let mut deck = Deck::new(0, 48_000, 64, "balanced");
    // build Arc<LoadedAudio> with interleaved L=0.5, R=-0.25 for ≥64 frames
    // deck.load(audio).unwrap();
    deck.set_volume(0.0).unwrap();
    deck.play().unwrap();
    let _ = deck.process(64).unwrap();
    let peaks = deck.level_peaks();
    assert!(
        peaks.peak_l > 0.4,
        "pre-fader peak_l should ignore volume=0, got {}",
        peaks.peak_l
    );
    assert!(
        peaks.peak_r > 0.2,
        "pre-fader peak_r should ignore volume=0, got {}",
        peaks.peak_r
    );
}
```

Fill in `load` using the same helper/`LoadedAudio { samples, sample_rate, .. }` pattern already in this file’s tests.

- [ ] **Step 2: Run test — expect fail (no `level_peaks`)**

Run: `cargo test -p engine-dsp levels_measure_pre_volume -- --nocapture`  
Expected: FAIL

- [ ] **Step 3: Implement on `Deck`**

In `Deck` struct add:

```rust
level_peaks: LevelPeaks,
```

In `new`: `level_peaks: LevelPeaks::default()`

Accessor:

```rust
pub fn level_peaks(&self) -> LevelPeaks {
    self.level_peaks
}
```

In `process`, after filter and **before** volume multiply:

```rust
self.eq.process_buffer(&mut self.buffer);
self.filter.process_buffer(&mut self.buffer);
self.level_peaks = LevelPeaks::from_buffer(&self.buffer);
for sample in &mut self.buffer {
    *sample *= self.volume;
}
```

When returning early silence (not playing), also set `self.level_peaks = LevelPeaks::default()`.

Import `LevelPeaks` at top of `deck.rs`.

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p engine-dsp levels_measure_pre_volume -- --nocapture`  
Also: `cargo test -p engine-dsp -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine-dsp/src/deck.rs
git commit -m "Measure pre-fader stereo peaks on each deck process."
```

---

### Task 3: Engine level snapshot API

**Files:**
- Modify: `engine-core/src/engine.rs`

- [ ] **Step 1: Add method next to `deck_playback_snapshot`**

```rust
/// Snapshot pre-fader stereo peaks for all decks.
pub fn deck_level_snapshot(&self) -> Vec<(usize, f32, f32)> {
    let Some(dsp_engine) = self.dsp_engine.as_ref() else {
        return Vec::new();
    };
    let dsp = match dsp_engine.lock() {
        Ok(dsp) => dsp,
        Err(_) => return Vec::new(),
    };

    let mut snapshot = Vec::with_capacity(dsp.num_decks());
    for deck_id in 0..dsp.num_decks() {
        let Some(deck) = dsp.deck(deck_id) else {
            continue;
        };
        let peaks = deck.level_peaks();
        snapshot.push((deck_id, peaks.peak_l, peaks.peak_r));
    }
    snapshot
}
```

- [ ] **Step 2: Compile-check**

Run: `cargo check -p engine-core`  
Expected: success

- [ ] **Step 3: Commit**

```bash
git add engine-core/src/engine.rs
git commit -m "Expose deck_level_snapshot for VU metering."
```

---

### Task 4: Tauri `Levels` event + peak-hold in notifier

**Files:**
- Modify: `gui-app/src-tauri/src/engine_events.rs`
- Modify: `gui-app/src-tauri/src/engine_notifier.rs`

- [ ] **Step 1: Extend `EngineEvent` and add emit helper + serde test**

In `engine_events.rs` add variant:

```rust
Levels {
    deck_id: usize,
    peak_l: f32,
    peak_r: f32,
    peak_hold_l: f32,
    peak_hold_r: f32,
},
```

```rust
pub fn emit_levels(
    app: &AppHandle,
    deck_id: usize,
    peak_l: f32,
    peak_r: f32,
    peak_hold_l: f32,
    peak_hold_r: f32,
) {
    emit_event(
        app,
        EngineEvent::Levels {
            deck_id,
            peak_l,
            peak_r,
            peak_hold_l,
            peak_hold_r,
        },
    );
}
```

Add test mirroring `engine_event_status_serializes_with_type_tag` asserting `type == "levels"`.

- [ ] **Step 2: Run Rust tests for events**

Run: `cargo test -p gui-app engine_event -- --nocapture`  
(or the crate name used by the Tauri package — typically `gui-app` / `dj_engine_gui` from `Cargo.toml` in `src-tauri`)  
Expected: PASS for new serde test

- [ ] **Step 3: Peak-hold + emit in notifier**

In `engine_notifier.rs`:

```rust
use crate::engine_events::{emit_levels, emit_position};

const PEAK_HOLD_DECAY_PER_TICK: f32 = 0.04; // ~1.2s at 30 Hz (tweak to taste)

struct PeakHoldState {
    hold_l: [f32; NUM_DECKS],
    hold_r: [f32; NUM_DECKS],
}

impl PeakHoldState {
    fn new() -> Self {
        Self {
            hold_l: [0.0; NUM_DECKS],
            hold_r: [0.0; NUM_DECKS],
        }
    }

    fn update(&mut self, deck_id: usize, peak_l: f32, peak_r: f32) -> (f32, f32) {
        if deck_id >= NUM_DECKS {
            return (0.0, 0.0);
        }
        Self::ballistics(&mut self.hold_l[deck_id], peak_l);
        Self::ballistics(&mut self.hold_r[deck_id], peak_r);
        (self.hold_l[deck_id], self.hold_r[deck_id])
    }

    fn ballistics(hold: &mut f32, peak: f32) {
        if peak >= *hold {
            *hold = peak;
        } else {
            *hold = (*hold - PEAK_HOLD_DECAY_PER_TICK).max(0.0);
        }
    }
}
```

In `notifier_loop`, create `let mut peak_hold = PeakHoldState::new();` before the while loop. Each tick after locking:

```rust
let levels = {
    let engine = state.engine.as_mut().unwrap();
    engine.deck_level_snapshot()
};
```

Release lock, then:

```rust
for (deck_id, peak_l, peak_r) in levels {
    let (hold_l, hold_r) = peak_hold.update(deck_id, peak_l, peak_r);
    emit_levels(&app, deck_id, peak_l, peak_r, hold_l, hold_r);
}
```

Emit levels for all decks every tick (including zeros when paused) so holds can decay.

- [ ] **Step 4: Compile Tauri crate**

Run: `cargo check -p` *(use package name from `gui-app/src-tauri/Cargo.toml`)*  
Expected: success

- [ ] **Step 5: Commit**

```bash
git add gui-app/src-tauri/src/engine_events.rs gui-app/src-tauri/src/engine_notifier.rs
git commit -m "Emit Levels engine events with peak-hold ballistics."
```

---

### Task 5: Frontend event + store wiring

**Files:**
- Modify: `gui-app/src/types.ts`
- Modify: `gui-app/src/lib/engineEvents.ts`
- Modify: `gui-app/src/stores/defaultDeck.ts`
- Modify: `gui-app/src/stores/engineStore.ts`
- Optionally: `gui-app/src/hooks/useEngine.ts` selectors

- [ ] **Step 1: Extend types**

In `types.ts`:

```ts
export type LevelMeterMode = "mono" | "stereo";

export interface DeckLevels {
  peak_l: number;
  peak_r: number;
  peak_hold_l: number;
  peak_hold_r: number;
}

export const ZERO_DECK_LEVELS: DeckLevels = {
  peak_l: 0,
  peak_r: 0,
  peak_hold_l: 0,
  peak_hold_r: 0,
};
```

Add to `DeckStatus`:

```ts
levels: DeckLevels;
```

- [ ] **Step 2: Defaults + event apply**

In `defaultDeck.ts` add `levels: ZERO_DECK_LEVELS` (import it).

In `engineEvents.ts`:

```ts
| {
    type: "levels";
    deck_id: number;
    peak_l: number;
    peak_r: number;
    peak_hold_l: number;
    peak_hold_r: number;
  }
```

In `applyEngineEvent`, handle `levels` like `position` via `patchDeckLevels`.

```ts
export function patchDeckLevels(
  status: EngineStatus,
  deckId: number,
  levels: DeckLevels,
): EngineStatus {
  return {
    ...status,
    decks: status.decks.map((deck) =>
      deck.id === deckId ? { ...deck, levels } : deck,
    ),
  };
}
```

In store: add `levelMeterMode: LevelMeterMode` default `"mono"`, action `setLevelMeterMode(mode)`, and ensure `applyEvent` path already uses `applyEngineEvent`.

- [ ] **Step 3: Typecheck**

Run: `cd gui-app && npm run typecheck` (or the repo’s equivalent script)  
Expected: success (or only unrelated existing errors)

- [ ] **Step 4: Commit**

```bash
git add gui-app/src/types.ts gui-app/src/lib/engineEvents.ts gui-app/src/stores/defaultDeck.ts gui-app/src/stores/engineStore.ts
git commit -m "Wire Levels events into the engine store."
```

---

### Task 6: `LevelMeter` component + mixer layout

**Files:**
- Create: `gui-app/src/components/LevelMeter.tsx`
- Modify: `gui-app/src/components/DeckMixer.tsx`

- [ ] **Step 1: Create `LevelMeter.tsx`**

```tsx
import { cn } from "@/lib/utils";
import type { DeckLevels, LevelMeterMode } from "../types";

const SEGMENTS = 12;
const YELLOW_FROM = 8; // segment index 0 = bottom
const RED_FROM = 10;

function segmentOn(level: number, indexFromBottom: number): boolean {
  const threshold = (indexFromBottom + 1) / SEGMENTS;
  return level >= threshold - 1e-6;
}

function holdSegment(hold: number): number | null {
  if (hold <= 0) return null;
  return Math.min(SEGMENTS - 1, Math.max(0, Math.ceil(hold * SEGMENTS) - 1));
}

function Ladder({
  peak,
  hold,
  className,
}: {
  peak: number;
  hold: number;
  className?: string;
}) {
  const holdIdx = holdSegment(hold);
  return (
    <div
      className={cn(
        "flex h-full w-1.5 flex-col-reverse gap-px",
        className,
      )}
      aria-hidden
    >
      {Array.from({ length: SEGMENTS }, (_, fromBottom) => {
        const on = segmentOn(peak, fromBottom);
        const isHold = holdIdx === fromBottom;
        let color = "bg-zinc-800";
        if (on || isHold) {
          if (fromBottom >= RED_FROM) color = "bg-red-500";
          else if (fromBottom >= YELLOW_FROM) color = "bg-amber-400";
          else color = "bg-emerald-500";
        }
        return (
          <div
            key={fromBottom}
            className={cn(
              "min-h-0 flex-1 rounded-[1px]",
              color,
              isHold && !on && "opacity-100 ring-1 ring-white/70",
            )}
          />
        );
      })}
    </div>
  );
}

export function LevelMeter({
  levels,
  mode,
}: {
  levels: DeckLevels;
  mode: LevelMeterMode;
}) {
  if (mode === "mono") {
    const peak = Math.max(levels.peak_l, levels.peak_r);
    const hold = Math.max(levels.peak_hold_l, levels.peak_hold_r);
    return <Ladder peak={peak} hold={hold} />;
  }
  return (
    <div className="flex h-full gap-px">
      <Ladder peak={levels.peak_l} hold={levels.peak_hold_l} />
      <Ladder peak={levels.peak_r} hold={levels.peak_hold_r} />
    </div>
  );
}
```

- [ ] **Step 2: Place meters between volume faders in `DeckMixer.tsx`**

Change the center fader row from:

```tsx
<div className="flex min-h-0 shrink-0 items-stretch gap-0.5 px-0.5">
  <DeckVolumeFader … deck 0 />
  <DeckVolumeFader … deck 1 />
</div>
```

to:

```tsx
<div className="flex min-h-0 shrink-0 items-stretch gap-0.5 px-0.5">
  <DeckVolumeFader … deck 0 />
  <div className="flex h-full items-stretch gap-0.5 px-0.5">
    <LevelMeter levels={decks[0]?.levels ?? ZERO_DECK_LEVELS} mode={levelMeterMode} />
    <LevelMeter levels={decks[1]?.levels ?? ZERO_DECK_LEVELS} mode={levelMeterMode} />
  </div>
  <DeckVolumeFader … deck 1 />
</div>
```

Pass `levels` through `useDeckMixerChannel` / deck props from store. Read `levelMeterMode` from store.

Optional small control (hidden for now or a tiny M/S toggle in mixer header): call `setLevelMeterMode`.

- [ ] **Step 3: Manual check**

Run: `cd gui-app && npm run tauri:dev` (or project’s usual command)  
Expected: play a track → ladders animate between faders; volume fader does not change meter height; gain/EQ do.

- [ ] **Step 4: Commit**

```bash
git add gui-app/src/components/LevelMeter.tsx gui-app/src/components/DeckMixer.tsx gui-app/src/hooks/useEngine.ts
git commit -m "Show VU ladders between mixer volume faders."
```

---

### Task 7: Close-out

- [ ] **Step 1: Run full DSP + event tests**

```bash
cargo test -p engine-dsp -- --nocapture
cargo test -p engine-core -- --nocapture
```

Expected: PASS

- [ ] **Step 2: Update issue #42**

Comment on https://github.com/geovannimp/rust-dj-engine/issues/42 with short summary + mark done / close with `completed`. Set project Status to Done on Phase 4 board if tracking.

- [ ] **Step 3: Final commit only if stray fixes remain**

---

## Spec coverage checklist

| Spec requirement | Task |
|------------------|------|
| Pre-fader measure | 2 |
| Stereo peak_l/r always | 1–4 |
| Peak-hold decay | 4 |
| `EngineEvent::Levels` ~30 Hz | 4 |
| Meters between faders | 6 |
| Mono/stereo UI config | 5–6 |
| Volume ignored by meter | 2 test |
| DSP + serde tests | 1, 4 |

## Notes for implementers

- Prefer DSP mutex snapshot (like `deck_playback_snapshot`) over extra atomics — same 30 Hz path as position.
- When paused, still emit levels so peak-hold can fall to zero.
- Keep `DeckStatus` free of high-rate churn: levels ride the dedicated event (store patches like position).
