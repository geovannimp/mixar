# Master Bus Cue (Headphones) Design

**Issue:** [#68](https://github.com/geovannimp/rust-dj-engine/issues/68)  
**Spec refs:** `docs/deck-spec.md` §5.15 Headphone cue / PFL (H2 Cue mix; Master Cue); builds on multi-bus routing + H1 PFL  
**Date:** 2026-07-16

## Goal

Let the DJ monitor the **master mix** in headphones (preview/cue bus) via a Pioneer-style **Master Cue** toggle and a **Cue/Master mix** knob, without changing what the room hears on the master bus.

Volume normalizer (#67) is explicitly out of scope for this work.

## Requirements

| Decision | Choice |
|----------|--------|
| Controls | Both: **Master Cue** button + **Cue/Master** mix knob |
| Interaction | Pioneer-style: Master Cue gates master as a headphone *source*; mix blends PFL ↔ (gated) master |
| Master tap | Post-fader / post-crossfader `mix_buffer`, **before** `master_volume` |
| Defaults | `cue_mix = 0` (Cue), `master_cue = false` |
| DSP structure | Dedicated **HeadphoneMonitor** unit (graph-oriented responsibility); not ad-hoc lerp in the master route |
| Persistence | Session only (re-apply on engine restart like headphone cue) |
| Preview off | Controls disabled / no-op; no cue bus |

Out of scope: split cue (H3), headphone/master delay compensation, booth bus, volume normalizer (#67).

## Architecture

```text
Decks → (channel volume + crossfader) → Sum → mix_buffer
                                              ├─► master bus × master_volume
                                              └─► HeadphoneMonitor
                                                    PFL = Σ pre_fader (headphone_cue && Playing)
                                                    master_tap = mix_buffer if master_cue else 0
                                                    out = (1 - cue_mix) * PFL + cue_mix * master_tap
                                                    └─► cue bus × cue_volume (+ clamp)
```

Bus ids unchanged: `master` and `cue` (UI “Preview”).

## Components

### DSP (`engine-dsp`)

- **`Mixer`:** add `cue_mix: f32` (0.0..=1.0) and `master_cue: bool` with getters/setters (reject out-of-range mix).
- **`HeadphoneMonitor`:** single-responsibility DSP unit used for the cue path. Prefer a dasp_graph node when practical; if PFL buffers remain outside the graph (as today), invoke the same unit from the cue branch of `route_to_buses` so logic stays one place.
- **Master bus:** unchanged — `mix_buffer × master_volume`.
- **PFL:** unchanged source rules — sum `pre_fader_buffer` for decks with `headphone_cue` and `Playing`.

### Engine (`engine-core`)

- `Engine::set_cue_mix(f32)` / `cue_mix()`
- `Engine::set_master_cue(bool)` / `master_cue()`
- Forward to mixer; surface in status/events if mixer-level fields are already exposed to the UI.

### GUI (`gui-app`)

- Tauri commands: `set_cue_mix`, `set_master_cue`.
- Session state defaults `0` / `false`; re-apply on `start_engine` / settings restart alongside headphone cue.
- Mixer strip: compact **Master Cue** toggle + **Cue ↔ Master** knob (near crossfader / headphone area).
- Disable when preview/cue bus is not configured.

## Behavior matrix

| Master Cue | Cue/Master | Deck PFL | Cue bus |
|------------|------------|----------|---------|
| off | 0 (Cue) | none | silence |
| off | 0 | one+ playing | PFL only (today) |
| off | >0 | any | no master bleed; mix toward Master only attenuates PFL |
| on | 0 | none | silence |
| on | 1 | none | master tap × `cue_volume` |
| on | 0.5 | one+ | equal blend of PFL + master tap |
| any | any | — | master **room** bus unchanged |

## Testing

| Layer | Cases |
|-------|--------|
| Unit (`engine-dsp`) | Defaults; `cue_mix` range validation; silence with Master Cue off / no PFL; PFL-only matches current cue; Master Cue on + mix=1 ≈ `mix_buffer` (independent of `master_volume`); mid blend; `cue_volume` scales cue only |
| Engine / Tauri | Setters round-trip; rehydrate after engine restart |
| GUI | Controls wired; disabled when preview off |

## Acceptance

1. With preview on, Master Cue on, mix at Master, no deck PFL → hear master in headphones; room master unchanged.
2. Master Cue off, mix at Cue, deck PFL on → same as today’s PFL-only cue.
3. Master Cue off, mix toward Master → no master in headphones.
4. Mid mix with Master Cue on blends PFL and master.
5. `cue_volume` scales the cue bus; `master_volume` does not change the headphone master tap level.
