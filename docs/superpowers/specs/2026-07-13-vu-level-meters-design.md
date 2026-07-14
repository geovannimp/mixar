# VU / Level Meters Design

**Issue:** [#42](https://github.com/geovannimp/rust-dj-engine/issues/42)  
**Spec refs:** `docs/deck-spec.md` §5.8 X5 / Phase 4  
**Date:** 2026-07-13

## Goal

Per-deck pre-fader level meters in the center mixer strip, with peak + peak-hold ballistics. Engine always reports stereo L/R; the UI chooses mono or stereo display via config.

## Requirements

| Decision | Choice |
|----------|--------|
| Signal tap | Pre-fader: after gain trim → EQ → filter, **before** volume |
| Ballistics | Peak (per buffer) + peak-hold (~1–1.5 s decay) |
| Layout | Meters **between** the two volume faders |
| Channels | Engine always emits stereo `peak_l` / `peak_r` (+ holds) |
| Display | Frontend config `mono` \| `stereo` (mono = max of L/R and holds) |
| Transport coupling | Dedicated `EngineEvent::Levels` (~30 Hz), not on `DeckStatus` |

Out of scope for this issue: master bus meters, PFL metering, numeric dB readout, persistence of display mode beyond a simple UI preference.

## Architecture

```
Deck::process
  gain trim → EQ → filter
       │
       ├─► LevelMeter (per-buffer peak_l / peak_r)
       └─► volume → mixer

Producer writes latest peaks to lock-free atomics (per deck)
EngineNotifier (~33 ms) reads atomics, applies peak-hold decay
  → EngineEvent::Levels { deck_id, peak_l, peak_r, peak_hold_l, peak_hold_r }
React store → DeckMixer center strip (mono or stereo ladders)
```

### DSP (`engine-dsp`)

- Small `LevelMeter` helper on `Deck` (or module next to filter/EQ).
- After filter processing and **before** multiplying by `volume`, scan the stereo buffer:
  - `peak_l = max(|sample_l|)` for the buffer
  - `peak_r = max(|sample_r|)` for the buffer
- Store as atomics (`AtomicU32` of `f32` bits) or fields readable via a snapshot API from the producer / engine — **no allocation / lock on the audio hot path beyond atomics**.
- When not playing / silence: peaks report `0.0`.

### Engine (`engine-core`)

- Expose `deck_level_snapshot() -> Vec<(deck_id, peak_l, peak_r)>` (or equivalent) for the notifier.
- Peak-hold **not** required on the audio thread; decay can live in the notifier or GUI layer. Prefer notifier so all UIs get consistent holds.

### Tauri event bus (`gui-app`)

Extend `EngineEvent`:

```text
Levels {
  deck_id: usize,
  peak_l: f32,       // 0.0..=1.0 linear
  peak_r: f32,
  peak_hold_l: f32,
  peak_hold_r: f32,
}
```

- Emit from existing `EngineNotifier` loop alongside `Position`.
- Frontend handles `type: "levels"` in the existing `engine://event` listener.

### Frontend

- Store per-deck level fields (`peak_l/r`, `peak_hold_l/r`).
- Preference: `levelMeterMode: "mono" | "stereo"` (local default: `"mono"`; toggle later if desired).
- **`LevelMeter` component:** vertical LED ladder (~10–12 segments), green → yellow → red; peak-hold as a bright tick.
- **Mixer layout:**

```text
[EQ A] [Fader A]  [meter(s) A] [meter(s) B]  [Fader B] [EQ B]
                      └──── between volume faders ────┘
```

  - Mono: one ladder per deck in that center gap.
  - Stereo: L/R pair per deck in that center gap.

## Error handling

- Lock/poison on app state: skip the tick (same as position notifier).
- Emit failures: log warning, continue.
- No track / paused: levels stay at zero; holds decay to zero.

## Testing

- `engine-dsp`: unit test that a synthetic stereo buffer yields expected `peak_l` / `peak_r` when measured pre-volume, and that volume does not change the reported meter peaks.
- `engine_events`: serde tag test for `Levels` (same pattern as `Status`).
- Manual: play a track, confirm ladders animate between faders; switch mono/stereo if UI toggle is present.

## Acceptance

1. Playing deck shows live meters between the channel volume faders.
2. Levels respond to gain/EQ/filter but **not** to channel volume fader.
3. Peak-hold marker decays after peaks.
4. Engine payload is always stereo; UI can render mono or stereo from config.
5. No regressions to position streaming or mixer controls.
