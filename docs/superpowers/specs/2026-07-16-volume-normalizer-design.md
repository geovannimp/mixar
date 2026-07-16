# Volume Normalizer (Auto Gain / ReplayGain) Design

**Issue:** [#67](https://github.com/geovannimp/rust-dj-engine/issues/67)  
**Spec refs:** existing `gain_trim_db` on decks; `track_analysis` persistence; offline `analyzer` pipeline  
**Date:** 2026-07-16

## Goal

Automatically level tracks to a consistent perceived loudness using offline analysis (ReplayGain tags when present, otherwise ITU-R BS.1770 integrated LUFS), applied as a **separate auto-gain layer** under the user’s gain trim knob.

## Requirements

| Decision | Choice |
|----------|--------|
| Gain model | Separate layers: `effective_db = auto_gain_db + gain_trim_db` (knob = user offset only) |
| Measurement | Prefer file ReplayGain tags; else compute integrated LUFS (BS.1770) |
| Default target | **−18 LUFS** (Settings-editable) |
| Default enabled | **true** |
| Unanalyzed / missing loudness | `auto_gain_db = 0` (no surprise boost) |
| Clamp | Auto gain clamped to **±12 dB** |
| Persistence | `loudness_lufs` on `track_analysis` (nullable) |
| Settings apply | Recompute auto gain for currently loaded decks when enable/target change |

Out of scope: baking gain into files, real-time AGC, master limiter, VDJ-style “knob includes auto” mapping, proprietary Serato/Rekordbox parity.

## Architecture

```text
Analyze
  decode mono PCM (existing)
  → ReplayGain track tag? → derive loudness_lufs
  → else ebur128 / BS.1770 integrated LUFS
  → upsert track_analysis.loudness_lufs

Load / settings change (normalizer on + loudness present)
  → auto_gain_db = clamp(target_lufs - loudness_lufs, -12, +12)
  → Deck applies db_to_linear(auto_gain_db + gain_trim_db)

Normalizer off or no loudness
  → auto_gain_db = 0
```

## Components

### Analyzer / tags

- After existing decode path, compute or import loudness before/with other analysis outputs.
- Lofty: read ReplayGain track gain when available; convert to a stored `loudness_lufs` consistent with the target math (document conversion: typically `loudness ≈ reference − gain_db` for ReplayGain 2.0-style tags — pick one reference and keep apply-path inverse consistent).
- Otherwise measure integrated loudness on analyzer mono PCM (same analysis duration window as BPM when limited).
- Extend `TrackAnalysis` / analyzer-core result with `loudness_lufs: Option<f64>`.

### Library

- Add nullable `loudness_lufs` column on `track_analysis` (Sea-ORM entity; schema sync on open).
- Upsert includes the new field on analyze.
- Expose loudness to GUI load path (track summary or analysis fetch used on load).

### DSP (`engine-dsp`)

- `Deck`: `auto_gain_db: f32` (default `0.0`), getters/setters.
- In `process`, replace trim-only scale with `db_to_linear(auto_gain_db + gain_trim_db)`.
- VU / pre-fader tap remains after this combined trim (unchanged order relative to EQ/filter).

### Engine / Tauri

- `Engine::set_deck_auto_gain_db(deck_id, db)`.
- On track load: if settings.normalizer enabled and loudness known → set auto; else 0.
- Settings: `volume_normalizer_enabled`, `target_lufs` (−18 default).
- On settings save: recompute auto for decks that have a loaded track with known loudness.
- UI: Settings toggle + target control; trim knob unchanged (still `gain_trim_db` only). Optional later: show effective/auto readout — not required for MVP.

## Behavior matrix

| Normalizer | Loudness | Trim | Effective |
|------------|----------|------|-----------|
| off | any | T | T |
| on | missing | T | T |
| on | L | T | clamp(target−L) + T |

## Testing

| Layer | Cases |
|-------|--------|
| Analyzer / library | Tag import path; LUFS on fixture; upsert `loudness_lufs` |
| DSP | `auto + trim` linear gain; defaults; setters |
| Apply math | Unit test `auto_gain_db(target, measured)` clamp |
| Integration / GUI | Load with normalizer on/off; settings change updates loaded decks |

## Acceptance

1. Analyzed tracks store `loudness_lufs` (from tag or compute).
2. With normalizer on, differently mastered tracks sit closer in level at equal faders and trim-center.
3. With normalizer off, behavior matches today’s trim-only path.
4. Manual trim remains an additive user offset.
5. Missing loudness never applies a large boost.
6. Settings expose enable + target (−18 default).
7. Tests cover gain math, deck apply, and analysis persistence.
