# Keyboard & Key Shift pad modes — design (#298)

**Date:** 2026-10-07
**Status:** Awaiting owner review
**Issue:** [geovannimp/mixar#298](https://github.com/geovannimp/mixar/issues/298)
**Related:** deck-spec §5.4 (P4 key shift), §5.5/§5.10 (pad modes); `docs/stems-pad-mode-design.md` (same pad-mode pattern); #300 (DDJ-400 LED pages)

## Goal

Add two pitch-oriented performance-pad modes, distinct from key lock:

- **Keyboard mode** — pads trigger pitch-relative cue playback: the 8 pads are
  ascending scale degrees of the loaded track's analyzed key; pressing a pad
  plays the deck's cue at that pitch.
- **Key Shift mode** — pads apply defined semitone key changes to the whole
  deck, independent of tempo.

Both are **session-only** performance state. They never mutate the analyzed key
or write "manual correction" rows to `library.db`.

This pass also introduces the realtime pitch capability the modes need, because
none exists today (`tech-spec.md` currently lists semitone shift as an MVP
non-goal). Key lock only *holds* pitch while tempo changes.

## Decisions

| Topic | Choice |
|-------|--------|
| Pitch DSP | Realtime pitch factor in `crates/stretch`: keylock-stretch at `tempo/pitch` + `StreamingSincResampler` output stage (see math below) |
| Key Shift pads | 8 pads = semitones `[0, +1, +2, +3, −4, −3, −2, −1]` (Rekordbox/DDJ-FLX4 octave span) |
| Key Shift latch | Press latches the offset; re-press the active pad clears to 0. Shift bank (`ch 9/11`) → any pad resets to 0 |
| Keyboard pads | 8 pads = ascending scale degrees from the analyzed root: `[0, 2, 4, 5, 7, 9, 11, 12]` (major default) |
| Keyboard behavior | Press = seek to cue, apply the pad's pitch offset, play (momentary). Release = clear offset to 0 |
| Keyboard scale select | Shift bank (`ch 9/11`) pads 1/2/3 → `major` / `minor` / `pentatonic`; pads 4–8 no-op |
| Key lock interaction | Key shift is an additive session offset applied on top of key lock; it never forces key lock on/off and works with key lock at any tempo |
| Analyzed key | Root comes from `DeckControlState.key`; missing/unknown key defaults to C major. UI may *display* the resulting key but never persists it |
| Persistence | Per-deck runtime state only (`Deck` + `DeckControlState`); no `track_*` / `save_hot_cue` side effects |
| Stems | When key shift ≠ 0 the stretch path is used and the source feed is stem-aware, so stems pitch-shift too |

## Realtime pitch architecture

We want output **tempo factor `T`** and **pitch factor `P`** (semitone `s` →
`P = 2^(s/12)`). The timestretch WideKeylock engine changes tempo while holding
pitch; a downstream resample by `P` then shifts pitch and scales duration by `P`:

```
stretcher tempo_rate      = T / P        (keylock holds pitch)
output resample step      = P            (consume P engine frames per output frame)
=> net tempo = (T/P) · P = T,  net pitch = P
```

`timestretch::core::resample::StreamingSincResampler` is stateful, bounded
latency, takes a `step` per output sample, and exposes `process_into_capped`
(emit ≤ N) plus `next_output_source_pos` / `group_delay_samples` for playhead
bookkeeping. Two instances (L/R) share the same step.

Source frames consumed per final output frame is `T` (independent of `P`), so
the existing playhead advance (`position_frac += src_step` per fed frame) stays
correct; only the *number* of source frames fed per callback changes.

`P == 1.0` bypasses the resampler entirely (bit-identical to today).

## Pad layouts

Engine slots are zero-based `0..7`; UI/DDJ-400 pads are `1..8`.
Rekordbox convention: pad 1 top-left, pad 4 top-right, pad 5 bottom-left, pad 8
bottom-right.

### Key Shift (`PadMode::KeyShift`)

| Slot | Pad | Semitones | Label |
|------|-----|-----------|-------|
| 0 | 1 | 0 | 0 |
| 1 | 2 | +1 | +1 |
| 2 | 3 | +2 | +2 |
| 3 | 4 | +3 | +3 |
| 4 | 5 | −4 | −4 |
| 5 | 6 | −3 | −3 |
| 6 | 7 | −2 | −2 |
| 7 | 8 | −1 | −1 |

Press latches the offset; re-pressing the pad whose offset is active sets 0.
Shift-bank pads (`ch 9/11`, notes `0x70–0x77`) reset to 0.

### Keyboard (`PadMode::Keyboard`)

Scale degree tables (semitones from the analyzed root):

| Scale | Degrees (8) |
|-------|-------------|
| major | 0, 2, 4, 5, 7, 9, 11, 12 |
| minor | 0, 2, 3, 5, 7, 8, 10, 12 |
| pentatonic | 0, 2, 4, 7, 9, 12, 14, 16 |

Press: snap the playhead to the deck cue (active hot cue if set, else the cue
point, else no seek), apply the pad's semitone offset, and play. Release: clear
the pitch offset (playback continues). Shift-bank pads 1/2/3 select the scale;
the selection is UI-visible and persists for the deck session only.

## Engine surface

### `crates/engine-api`

- `PadMode::{Keyboard, KeyShift}`.
- `KeyboardScale { Major, Minor, Pentatonic }` (`snake_case`).
- `CmdBody`:
  - `SetKeyShift { semitones: f32 }`
  - `SetKeyboardScale { scale: KeyboardScale }`
  - `KeyboardPadPress { slot, shift }` / `KeyboardPadRelease { slot }`
  - `KeyShiftPadPress { slot, shift }` / `KeyShiftPadRelease { slot }`
- `EvtBody::DeckUpdated` + `DeckSnapshot` gain `#[serde(default)] key_shift: f32`
  and `#[serde(default)] keyboard_scale: KeyboardScale`.

### `crates/engine-dsp`

- `Deck` gains `key_shift_semitones: f32`; `set_key_shift_semitones`,
  `key_shift_semitones`, `pitch_factor()`.
- Stretch path selection: today `want_stretch = key_lock && stems.is_none()`.
  It becomes `key_shift != 0 || (key_lock && stems.is_none())`, i.e. key shift
  always engages the pitch path, and key lock still skips stems unless key
  shift is active.
- `play_stretched` sets `stretcher.set_pitch_factor(P)`; the source feed reads
  through `mix_at_position` when stems are attached (otherwise
  `interpolate_stereo`), so stems pitch-shift with the same playhead.

### `crates/stretch`

- `TimeStretcher::set_pitch_factor(f64)` (default no-op for other impls).
- `TimestretchStretcher` owns two `StreamingSincResampler`s; `pull_interleaved`
  computes `engine_out = ceil(out_frames · P) + pad`, tops up, processes, then
  resamples into `out_frames`. `start_delay`/`queued_source_frames` account for
  the resampler group delay.

### `crates/engine-core`

- `DeckControlState` gains `key_shift_semitones: f32`, `keyboard_scale:
  KeyboardScale`.
- `Engine::set_deck_key_shift`, `set_deck_keyboard_scale`,
  `keyboard_pad_press/release`, `key_shift_pad_press/release`.
- `pad_press`/`pad_release` dispatch the two new modes.
- Snapshot maps the new fields; `clear_loaded_track` resets both to 0 / Major.

## Controller (DDJ-400)

Note: the declarative `signal` LED routing and `DeckFeedback` struct from
#300 are **not on this branch** (that work is unmerged). This follows the
existing snapshot + `apply_output_signal` mechanism instead.

- `catalog.rs`: `PAD_MODES` += `keyboard`, `key_shift`; new leaves
  `keyboard_pad`, `keyboard_scale`, `key_shift_pad`, `key_shift_reset`.
- `action.rs`: mode buttons → `SetPadMode`; `keyboard_pad` → mode-specific
  press/release kinds; `key_shift_pad` likewise; `keyboard_scale` →
  `SetKeyboardScale`; `key_shift_reset` → `SetKeyShift { semitones: 0.0 }`.
- `device.toml`: mode buttons `pad_mode_keyboard` (`0x69`),
  `pad_mode_key_shift` (`0x6F`) on `deck_1`/`deck_2`; keyboard pads
  `ch 8/10 notes 0x40–0x47` (press) and `ch 9/11` (scale); key-shift pads
  `ch 8/10 notes 0x70–0x77` (press) and `ch 9/11` (reset); matching out-only
  `_led` aliases for the mode buttons and both banks.
- `map.toml`: bind the above actions and `[outputs.deck_N]` LED targets.
- `session.rs`: extend `set_deck_pad_mode` (force-refresh the new banks, like
  `refresh_hot_cue_leds`); add `set_deck_key_shift` / `set_deck_keyboard_scale`
  that mirror the value and light the matching pad LED; handle
  `CmdBody::SetKeyShift` / `SetKeyboardScale` in the local-mirror match.
- `crates/controller/src/engine.rs` + `crates/host-flutter/src/api/controller.rs`:
  add `set_deck_key_shift` to `ControllerEngine`; in `apply_engine_mirror`
  read `key_shift` / `keyboard_scale` from `DeckUpdated`/`EngineStatus` and
  mirror them (fresh-attach replay already runs through `EngineStatus`).
- Update `docs/ddj-400-hardware-checklist.md` (remove the two "waiting on"
  rows, document bindings).

## Flutter

- `pad_modes.dart`: `kPadModes` += `keyboard`, `keyShift`; short labels `Keys`
  / `Shift`; scale-degree + semitone tables; helpers.
- `deck_pads_host.dart`: `_toEnginePadMode` cases; wire press/release/scale
  callbacks; pass real `shift` for the generic pad path.
- `deck_pads_panel.dart`: two new `_modeBody` grids (offset labels; active pad
  highlight for the latched Key Shift offset; disabled while no track).
- `engine_ui.dart` + `engine_providers.dart`: `keyShift` / `keyboardScale`
  maps, providers, and `applyEngineEvt` handling; a resulting-key display that
  is read-only.
- FRB: regenerate `gui-flutter` bindings for the new commands/events.

## Testing

| Layer | Tests |
|-------|-------|
| `stretch` | pitch factor math; sine in → frequency × P; duration unchanged; P=1 bypass; group-delay accounting |
| `engine-dsp` | `set_key_shift_semitones` shifts output frequency; playhead/tempo independent of pitch; stems + key shift; reset on unload |
| `engine-core` | `bus_key_shift.rs`: pad press latch/clear, keyboard press/release, mode switch, `SetKeyShift`/`SetKeyboardScale`, snapshot state-sync, no library writes |
| `controller` | action resolution for new leaves; `ddj400_feedback` mode-gated LED banks; mode-button → `SetPadMode` |
| Flutter | `pad_modes_test` cycle/labels; `engine_ui_test` keyShift apply; keyboard/key-shift pad widget test |

## Out of scope

- Harmonic/auto key sync (deck-spec P7).
- Persisting key shift or keyboard scale to `library.db`.
- Scale root transposition beyond the analyzed key (no per-deck root override).
- Pad FX / Beat FX (#40, #250).

## Success criteria

1. With key shift 0 and Hot Cue/Stems/Sampler/etc., behavior is unchanged.
2. Key Shift pads shift the deck by the assigned semitones without changing
   tempo; re-press clears; shift bank resets.
3. Keyboard pads play the cue at ascending scale degrees for the selected
   scale; release clears the pitch offset.
4. Key shift layers on key lock without disturbing the analyzed/library key.
5. DDJ-400 mode buttons, both pad banks, and LEDs are wired.
6. The four required test classes pass.
