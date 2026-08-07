# Control value normalization (0..1)

Date: 2026-08-07  
Issue: [#138](https://github.com/geovannimp/rust-dj-engine/issues/138)  
Status: implementing  
Depends on: soft-takeover owners ([#132](https://github.com/geovannimp/rust-dj-engine/pull/132) merged)

## Goal

Absolute deck/mixer controls use one **0..1 fader-position** language for cmds, events/status, and engine runtime strip state. Map to DSP units (dB, playback ratio) only at the engine→DSP edge. Analysis/DB stay physical (dB, LUFS).

## Decisions

| Topic | Choice |
|-------|--------|
| Scope | Cmds + events + runtime strip (hard break) |
| Tempo wire | Fader **position** `0..1` (not playback ratio) |
| Invert | `invert: bool` on `map.toml` input bindings (default false); `norm = 1 - norm` after MIDI normalize, before publish |
| Soft-takeover | Norm-only compare; delete `db_to_norm` / `speed_to_norm` |
| Not normalized | `auto_gain_db`, loudness/LUFS, BPM, ms positions |

## Wire renames

| Today | After |
|-------|--------|
| `volume` | unchanged |
| `eq.{low,mid,high}` dB | same fields, values `0..1` (center `0.5` = 0 dB) |
| `filter_db` | `filter` |
| `gain_trim_db` | `gain_trim` |
| `speed` ratio | `speed` position `0..1` |
| `SetEqBand.gain_db` | `gain` |
| `SetFilter.filter_db` | `filter` |
| `SetGainTrim.gain_db` | `gain_trim` |

Crossfader / cue mix already `0..1`.

## Mapping

```toml
tempo = { action = "Deck(_)::set_speed", soft_takeover = true, invert = true }  # DDJ-400
```

Controller: MIDI → `0..1` → optional invert → publish cmd with that norm (and `soft_takeover` flag). No dB/speed conversion in controller.

## Engine

- Store strip controls as `0..1`.
- Soft-takeover: `|incoming - current| ≤ 3/127` in that space; latch unchanged.
- On apply: convert once into DSP (`±24` dB strip; tempo `±16%` from position — same curves as today’s controller helpers, moved into engine-core).
- Sync that computes playback ratios stays internal (ratio), but `SetSpeed` / snapshot `speed` are positions. When sync changes tempo, update stored position from the new ratio via the inverse map.

## UI

- Zustand / wire mirror norms.
- Display: convert at component edge (`0..1` ↔ dB or pitch %).
- Pitch slider value **is** `speed` (position). Publishing `set_speed` sends the slider norm directly (no invert — invert is HW-only).

## Out of scope

Fractional beats (#137); pad routing; changing analysis DB schema.
