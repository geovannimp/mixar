# PlatterMotion — design

**Date:** 2026-10-13
**Status:** In progress
**Related:** deck-spec §5.11; PR #512 (jog flicker) is adjacent but separate

## Goal

Give Mixar a first-party **platter motion authority** so hardware jog, UI jog, and
future Pad FX scratch/backspin patterns all drive the same playback rate — without
vendoring Mixxx or depending on a nonexistent Rust vinyl crate.

## Problem (observed)

On a **paused** deck (DDJ-400 + UI):

1. Jog feels like ~1 s of **normal-speed** play, then stop — not hand-tracked scratch.
2. On-screen jog does not follow hardware motion.
3. Mixxx UI + hardware both look like variable-rate scratch on the waveform.

Root causes in today’s `Deck` jog:

- Outer default is `PitchBend` (`1.0 ± 8%`). Without a vinyl touch edge, paused scrub
  sounds like nearly-1× playback for the gesture duration.
- Vinyl velocity uses **audio-buffer dt**, not a fixed filter clock.
- UI `JogPlatter` **freezes** rotation while `jogTouching` is true.

## Research summary

| Source | Takeaway |
|--------|----------|
| xwax `pitch.h` | Classic alpha–beta pitch filter on fixed `dt` (GPL3 code; math is standard) |
| Mixxx `scratch2` | Same filter, 1 ms timer, rate **replaces** play while enabled |
| Mixxx UI | Separate position-chase PD → same rate bus |
| crates.io | No usable Rust vinyl/scratch crate |
| Rekordbox Pad FX | Vinyl-style pad FX = hold/release **rate automation**; pad-editor `SP.SCRATCH` is unrelated (sampler→deck load) |

## Decisions

| Topic | Choice |
|-------|--------|
| Ownership | New `PlatterMotion` in `engine-dsp` (pure); `Deck` delegates |
| Output | Absolute `playback_rate` while motion is active; transport play uses tempo × rate |
| Vinyl filter | Alpha–beta, **fixed 1 ms** integration inside `tick(frames)` |
| Constants | `intervals/rev = 720`, `rpm = 33⅓`, `α = 1/8`, `β = α/32` (controller-scratch defaults) |
| Paused + outer | Treat as **vinyl scrub** (absolute rate), not pitch-bend |
| Playing + outer | Keep pitch-bend spring |
| Touch (vinyl) | Enable scratch authority; rate → 0 until ticks; release → ramp to transport rate if playing else idle `1.0` without scrubbing |
| UI platter | Always follow playhead unless the user is mid local drag |
| Pad FX later | `MotionSource::Envelope` writes the same rate authority (stub API now) |
| Licensing | Reimplement filter from the standard algorithm; do **not** copy Mixxx/xwax sources |

## Architecture

```
jog ticks + touch     ┐
UI position chase     ├─► PlatterMotion ─► rate (+ active flag)
scripted envelope     ┘
        │
        ▼
Deck::effective_speed / process (rate replaces motor while active)
```

### `PlatterMotion` surface (v1)

```rust
pub enum MotionMode { Vinyl, PitchBend, Ignore }

pub struct PlatterMotion { /* filter + bend + envelope stub */ }

impl PlatterMotion {
    pub fn set_touch(&mut self, touching: bool, transport_playing: bool);
    pub fn set_modes(&mut self, top: MotionMode, outer: MotionMode);
    pub fn add_ticks(&mut self, delta: i32);
    /// Optional absolute music-rate override for Pad FX patterns (None = inactive).
    pub fn set_envelope_rate(&mut self, rate: Option<f32>);
    pub fn tick(&mut self, frames: usize, sample_rate: u32, transport_playing: bool);
    pub fn rate(&self) -> f32;
    pub fn touching(&self) -> bool;
    pub fn driving(&self) -> bool;
}
```

### Rate semantics

- **Vinyl (or paused outer):** `rate` is absolute music rate (`1.0` = track speed at
  current tempo ratio). Touch with no ticks → `0.0`.
- **Playing pitch-bend:** `rate = 1.0 + bend` (existing spring).
- **Envelope (future):** if `Some(r)`, overrides live filter until cleared.
- `Deck::effective_speed() = playback_ratio() * motion.rate()` unchanged.
- `jog_driving_audio()` ← `motion.driving()`.

### Fixed-dt vinyl step

Each `tick(frames)`:

1. Accumulate vinyl ticks since last tick.
2. Subdivide the buffer into `⌊frames * 1000 / sample_rate⌋` steps of 1 ms
   (plus remainder carried).
3. Each 1 ms step: `observation(dx)` where
   `dx = (pending_ticks as f64) * (1.0 / (rpm * intervals_per_rev / 60.0))`
   then clear the consumed pending share for that step (Mixxx-style: feed whole
   accumulator once per 1 ms timer fire — we feed once per 1 ms quantum with the
   ticks accumulated for that quantum).
4. `rate = clamp(predicted_velocity, -3, 3)`.

Practical v1 (matches Mixxx scratchProcess): on each 1 ms quantum, call
`observation(dx * tick_accumulator)` once and zero the accumulator — ticks that
arrived during the audio buffer are attributed to the quanta inside that buffer.

### UI

`JogPlatter._syncRotationFromPlayhead`: only skip playhead sync while **local**
`_dragging`. Hardware `jogTouching` must still rotate the disc from `positionMs`.

### Out of scope (this pass)

- Full UI position-PD controller (can keep tick-based UI jog; playhead follow is enough).
- Shipping Pad FX pattern banks (envelope API only).
- Per-mapping intervals/rpm config.
- DVS timecode.

## Success criteria

1. Paused vinyl/touch jog: waveform rate tracks hand speed (not ~1× bursts).
2. Paused outer (no touch) scrub moves playhead with variable rate, not pitch-bend crawl.
3. Playing outer bend still springs back.
4. UI platter rotates with hardware scratch.
5. `PlatterMotion::set_envelope_rate(Some(_))` can drive rate without ticks (unit test).
6. Existing slip / key-lock-while-jog tests still pass.
