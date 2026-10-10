# Keyboard & Key Shift pad modes — design (#298)

**Date:** 2026-10-07
**Status:** Shipped (2026-10-07); corrected to the Rekordbox page model (2026-10-08)
**Issue:** [geovannimp/mixar#298](https://github.com/geovannimp/mixar/issues/298)
**Related:** deck-spec §5.4 (P4 key shift), §5.5/§5.10 (pad modes); `docs/stems-pad-mode-design.md` (same pad-mode pattern); #300 (DDJ-400 LED pages); DDJ-400 hardware diagram footnote *6; rekordbox 7 manual p164–165

## Goal

Add two pitch-oriented performance-pad modes, distinct from key lock:

- **Keyboard mode** — the selected hot cue is the root. Each of the 8 pads
  auditions that hot cue at one semitone of the current page (momentary/gate).
- **Key Shift mode** — pads latch a defined semitone key change for the whole
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
| Semitone pages | Keyboard is 4 pitch-only pages; Key Shift is 5 (pages 1–4 shared + a Reset/Up/Down/Sync utility page). Slot 0 = pad 1 = root; page 2 `[0..+7]` is the default in both (DDJ-400 footnote *6; rekordbox 7 manual p164–165) |
| Key Shift pads | Press applies the page action and latches it (absolute semitones, RESET, UP, DOWN); SYNC is a no-op |
| Key Shift page switch | Shift bank (`ch 9/11`) slot 7 → next page, slot 8 → previous page (wrapping `5 → 1`); other slots no-op |
| Keyboard root | The selected hot cue (`keyboard_root_hot_cue`, default 0). No scale state exists |
| Keyboard pads | Four pitch-only semitone pages relative to the root hot cue; press seeks to the root and applies the page semitone (momentary) |
| Keyboard release | Gate off: fall back to another held pad's semitone, else restore the pre-press key shift and seek back to the root |
| Keyboard shift bank | (`ch 9/11`) slots 1–6 delete that hot cue; slot 7 → next page, slot 8 → previous page (wrapping `4 → 1`) |
| Key lock interaction | Key shift is an additive session offset applied on top of key lock; it never forces key lock on/off and works with key lock at any tempo |
| Resulting key | UI may *display* the analyzed key transposed by the session shift; it is never persisted |
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

Semitone offsets are clamped to `-16..=+16`. Non-finite input maps to `0`.

## Pad layouts

Engine slots are zero-based `0..7`; UI/DDJ-400 pads are `1..8`.
Rekordbox convention: pad 1 top-left, pad 4 top-right, pad 5 bottom-left, pad 8
bottom-right. `slot 0 = pad 1 = MIDI offset 0 = root`.

Both modes are page-based (DDJ-400 hardware diagram footnote *6; rekordbox 7
manual p164–165). **Keyboard is pitch-only with four pages; Key Shift adds a
fifth utility page.** The page bar labels each page by its semitone range:

| Page | Slots (pads 1–8) | Label | Keyboard | Key Shift |
|------|------------------|-------|:--------:|:---------:|
| 1 | +8, +9, +10, +11, +12, —, —, — | `+8…+12` | ✓ | ✓ |
| 2 (default) | 0, +1, +2, +3, +4, +5, +6, +7 | `0…+7` | ✓ | ✓ |
| 3 | −8, −7, −6, −5, −4, −3, −2, −1 | `-1…-8` | ✓ | ✓ |
| 4 | —, —, —, —, −12, −11, −10, −9 | `-9…-12` | ✓ | ✓ |
| 5 | RESET, DOWN, −5, −12, SYNC, UP, +7, +12 | `UTIL` | — | ✓ |

`—` is an empty slot (no-op). Page 5 mixes absolute semitones (`−5`, `−12`,
`+7`, `+12`) with deck actions: RESET (`key shift = 0`), UP / DOWN (nudge ±1
semitone), SYNC (unimplemented → no-op). It exists only in Key Shift mode, so
Keyboard's page switch wraps `4 → 1`.

### Key Shift (`PadMode::KeyShift`)

Five pages. Press latches the page action (absolute semitones set the offset,
RESET zeroes it, UP/DOWN nudge). The shift bank switches pages: slot 6 (`pad 7`)
→ next page, slot 7 (`pad 8`) → previous page, wrapping `5 → 1` and `1 → 5`.

### Keyboard (`PadMode::Keyboard`)

Four pitch-only pages; the utility page does not exist here. The root is the
selected hot cue (`keyboard_root_hot_cue`, default `0`). Press seeks to that hot
cue, applies the page's semitone offset, and plays (momentary). Release gates
off: it falls back to another held pad's semitone, else restores the pre-press
key shift and seeks back to the root. Shift bank slots 0–5 delete that hot cue;
slots 6/7 switch the page, wrapping `4 → 1` and `1 → 4`.

There is **no scale state**: the former per-scale degree tables (major/minor/
pentatonic) were removed in favour of the chromatic page model above.

## Engine surface

### `crates/engine-api`

- `PadMode::{Keyboard, KeyShift}`.
- `CmdBody`:
  - `SetKeyShift { semitones: f32 }`
  - `SetKeyboardPage { page: u8 }`
  - `SetKeyShiftPage { page: u8 }`
  - `SetKeyboardRoot { slot: u8 }`
  - `KeyboardPadPress { slot, shift }` / `KeyboardPadRelease { slot }`
  - `KeyShiftPadPress { slot, shift }` / `KeyShiftPadRelease { slot }`
- `EvtBody::DeckUpdated` + `DeckSnapshot` gain `#[serde(default)] key_shift: f32`,
  `#[serde(default = "default_pitch_page")] keyboard_page: u8` and
  `#[serde(default = "default_pitch_page")] key_shift_page: u8` (both default 2),
  and `#[serde(default)] keyboard_root_hot_cue: u8`. Each pad mode keeps its own
  page so Keyboard and Key Shift never affect each other.

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

- `pads.rs`: `KEYBOARD_PAGE_COUNT = 4`, `KEY_SHIFT_PAGE_COUNT = 5`,
  `DEFAULT_PITCH_PAGE = 2`, the `KEYBOARD_PAGES` (4) / `KEY_SHIFT_PAGES` (5)
  tables, `keyboard_page_action` / `key_shift_page_action`, and the per-mode
  `keyboard_page_next|prev` (wrap `4 → 1`) / `key_shift_page_next|prev` (wrap
  `5 → 1`).
- `DeckControlState` gains `key_shift_semitones: f32`, `keyboard_page: u8`,
  `key_shift_page: u8`, `keyboard_root_hot_cue: u8`.
- `Engine::set_deck_key_shift`, `set_deck_keyboard_page`, `set_deck_key_shift_page`,
  `set_deck_keyboard_root`, `keyboard_pad_press/release`,
  `key_shift_pad_press/release`.
- `pad_press`/`pad_release` dispatch the two new modes.
- Snapshot maps the new fields; `clear_loaded_track` resets key shift to `0`,
  both pages to `DEFAULT_PITCH_PAGE`, and root to `0`. A shift-bank page switch
  advances only the page of the mode being operated.

## Controller (DDJ-400)

Note: the declarative `signal` LED routing and `DeckFeedback` struct from
#300 are **not on this branch** (that work is unmerged). This follows the
existing snapshot + `apply_output_signal` mechanism instead.

- `catalog.rs`: `PAD_MODES` += `keyboard`, `key_shift`; new leaves
  `keyboard_pad`, `key_shift_pad`, and the shift-bank page/delete actions.
- `action.rs`: mode buttons → `SetPadMode`; `keyboard_pad` → mode-specific
  press/release kinds; `key_shift_pad` likewise.
- `device.toml`: mode buttons `pad_mode_keyboard` (`0x69`),
  `pad_mode_key_shift` (`0x6F`) on `deck_1`/`deck_2`; keyboard pads
  `ch 8/10 notes 0x40–0x47` (press) and `ch 9/11` (delete/page); key-shift pads
  `ch 8/10 notes 0x70–0x77` (press) and `ch 9/11` (page); matching out-only
  `_led` aliases for the mode buttons and both banks.
- `map.toml`: bind the above actions and `[outputs.deck_N]` LED targets.
- `session.rs`: extend `set_deck_pad_mode` (force-refresh the new banks, like
  `refresh_hot_cue_leds`); add `set_deck_key_shift` / `set_deck_keyboard_page` /
  `set_deck_key_shift_page` / `set_deck_keyboard_root` that mirror the value and
  light the matching pad LED (the LED refresh uses the page of the current pad
  mode); handle `CmdBody::SetKeyShift` / `SetKeyboardPage` / `SetKeyShiftPage` /
  `SetKeyboardRoot` in the local-mirror match.
- `crates/controller/src/engine.rs` + `crates/host-flutter/src/api/controller.rs`:
  add `set_deck_keyboard_page` / `set_deck_key_shift_page` /
  `set_deck_keyboard_root` to `ControllerEngine`; in `apply_engine_mirror` read
  `key_shift` / `keyboard_page` / `key_shift_page` / `keyboard_root_hot_cue`
  from `DeckUpdated`/`EngineStatus` and mirror them (fresh-attach replay already
  runs through `EngineStatus`).
- Update `docs/ddj-400-hardware-checklist.md` (remove the two "waiting on"
  rows, document bindings).

## Flutter

- `pad_modes.dart`: `kPadModes` += `keyboard`, `keyShift`; short labels `Keys`
  / `Shift`; `kKeyboardPageCount` (4) / `kKeyShiftPageCount` (5) /
  `kDefaultPitchPage`; the `PitchPad` value type and `kKeyboardPages` /
  `kKeyShiftPages` tables; `keyboardPage(page)` / `keyShiftPage(page)` /
  `keyboardPageRangeLabel(page)` / `keyShiftPageRangeLabel(page)` /
  `pitchPadLabel(pad)` helpers.
- `deck_pads_host.dart`: `_toEnginePadMode` cases; wire press/release/root/page
  callbacks; pass real `shift` for the generic pad path; prev/next wrap uses the
  active mode's page count (Keyboard 4, Key Shift 5).
- `deck_pads_panel.dart`: two new `_modeBody` grids (active pad highlight for the
  latched Key Shift offset; disabled while no track); a range-labelled page bar
  (`0…+7`, `UTIL`, …) in the Keyboard / Key Shift body.
- `pads/keyboard_pads.dart`: page pads plus a compact root hot-cue selector
  (8 chips; empty slots disabled).
- `pads/key_shift_pads.dart`: page pads with the latched-semitone highlight
  (page-5 specials never highlight as absolute).
- `engine_ui.dart` + `engine_providers.dart`: `keyboardPages` / `keyShiftPages` /
  `keyboardRoots` maps, `deckKeyboardPageProvider` / `deckKeyShiftPageProvider` /
  `deckKeyboardRootProvider`, and `applyEngineEvt` handling; a resulting-key
  display that is read-only.
- FRB: regenerate `gui-flutter` bindings for the new commands/events.

## Testing

| Layer | Tests |
|-------|-------|
| `stretch` | pitch factor math; sine in → frequency × P; duration unchanged; P=1 bypass; group-delay accounting |
| `engine-dsp` | `set_key_shift_semitones` shifts output frequency; playhead/tempo independent of pitch; stems + key shift; reset on unload |
| `engine-core` | `bus_pad_modes_kb_keyshift.rs`: page tables, pad press latch/clear, keyboard momentary gate, page switching, per-mode page independence, `SetKeyShift`/`SetKeyboardPage`/`SetKeyShiftPage`/`SetKeyboardRoot`, snapshot state-sync, no library writes |
| `controller` | action resolution for new leaves; `ddj400_feedback` mode-gated LED banks; mode-button → `SetPadMode` |
| Flutter | `pad_modes_test` page tables/labels; `engine_ui_test` keyboardPage/keyShiftPage/keyboardRoot apply; keyboard/key-shift pad widget tests |

## Out of scope

- Harmonic/auto key sync (deck-spec P7).
- Persisting key shift, pitch page, or keyboard root to `library.db`.
- Pad FX / Beat FX (#40, #250).

## Success criteria

1. With key shift 0 and Hot Cue/Stems/Sampler/etc., behavior is unchanged.
2. Key Shift pads apply the page semitones without changing tempo; shift-bank
   slots 7/8 switch pages.
3. Keyboard pads play the root hot cue at the page semitones; release restores
   the pre-press shift and seeks back to the root.
4. Key shift layers on key lock without disturbing the analyzed/library key.
5. DDJ-400 mode buttons, both pad banks, and LEDs are wired.
6. The four required test classes pass.
