# Functional jog wheel / scratch

Date: 2026-07-30  
Issue: [#43](https://github.com/geovannimp/rust-dj-engine/issues/43)  
Spec: `docs/deck-spec.md` §5.11 (J1, J2, J4, J6)  
Status: implemented (P2 slice)

## Goal

Make the deck jog platter drive audio (seek/scratch feel) via a MIDI-compatible bus API. GUI platter always acts as the **top** plate; outer-ring policy exists for controllers and settings.

## Scope (P2)

| ID | In scope |
|----|----------|
| J1 | Drag jog → engine cmds |
| J2 | Vinyl policy via `JogMode::Vinyl` on top |
| J4 | Tick → filtered rate override (Mixxx-shaped) |
| J6 | Platter animation stays BPM-synced when idle; tracks gesture while active |

**Deferred:** J3 CDJ feel refinements beyond `pitch_bend`/`ignore`, J5 sensitivity UI, slip (#38), MIDI mapper (#49).

## Bus API

### `JogMode`

Shared enum for top and outer slots:

| Variant | Behavior |
|---------|----------|
| `vinyl` | Ticks → filtered angular velocity → transient `jog_rate` (0 / reverse allowed). Near-zero velocity holds stopped. Leaving vinyl ramps `jog_rate` → `1.0`. |
| `pitch_bend` | Ticks add a decaying offset around `1.0`. |
| `ignore` | No effect. |

### Per deck

- `top_jog_mode: JogMode` — while `jog_touching`
- `outer_jog_mode: JogMode` — while not touching
- `jog_touching: bool`

**Defaults:** `top: vinyl`, `outer: pitch_bend`.

### Commands

| Kind | Body |
|------|------|
| `jog_touch` | `{ touching: bool }` |
| `jog_turn` | `{ delta: i32 }` relative ticks |
| `set_jog_mode` | `{ top: JogMode, outer: JogMode }` |

Resolution: if touching → `top_jog_mode`, else → `outer_jog_mode`.

### Settings

`AppSettings.default_top_jog_mode` / `default_outer_jog_mode`. Applied on engine/deck init; `set_jog_mode` overrides per deck. Emit modes + `jog_touching` on `DeckSnapshot` / `DeckUpdated`.

### Constants (P2, not bus)

`intervals_per_rev = 720`, `rpm = 33⅓`, Mixxx-like α/β filter, release ramp ~50–100 ms. GUI uses the same `intervals_per_rev` when converting pointer arc → ticks.

## Engine

- Layer `jog_rate` on base deck `speed` in playback stepping (`effective = speed * jog_rate`). Do not route vinyl stop through `set_deck_speed` (that path rejects ≤0).
- Control thread dispatches the three kinds; publish deck updated on mode/touch changes.
- No extra scratch PCM buffer in this slice; reuse fractional interpolator.

## GUI

- `JogPlatter`: always top — pointer down/up → `jog_touch`; drag → `jog_turn`. No outer hit zone.
- Disabled without track / when engine not running.
- Store helpers + wire kinds; settings selects for defaults.
- Visual: keep BPM tracker when idle; follow gesture/playhead while touching.

## Out of scope

- Slip mode, MIDI HID maps, configurable α/β/ticks UI, on-deck mode toggle widget (bus override is enough for MIDI/tests).

## Acceptance

- [x] Dragging the platter changes audio under `top: vinyl`
- [x] Touch hold can stop; release ramps back when playing
- [x] Untouched turns obey `outer_jog_mode` (MIDI-ready; GUI always touches)
- [x] Settings defaults + `set_jog_mode` work; status reflects modes/touch
- [x] Idle platter animation still tracks effective BPM
