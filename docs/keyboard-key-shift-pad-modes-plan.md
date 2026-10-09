# Keyboard & Key Shift Pad Modes Implementation Plan

> **Correction (#298, 2026-10-08):** The original plan (below, Tasks 2–6) assumed a
> per-scale Keyboard model (`major` / `minor` / `pentatonic` degree tables) and a
> fixed Key Shift layout. That model was wrong for Rekordbox. The shipped model is
> **chromatic and page-based**: five shared semitone pages, slot 0 = pad 1 = root,
> page 2 `[0..+7]` default, Keyboard root = the selected hot cue, and no scale
> state anywhere. Tasks 2, 3, and 5 below are corrected to that model; Task 1
> (realtime pitch) is unchanged. See `docs/keyboard-key-shift-pad-modes-design.md`
> for the authoritative page tables (DDJ-400 footnote *6; rekordbox 7 manual
> p164–165).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add realtime key-shift pitch to the engine and expose it as Keyboard and Key Shift performance-pad modes across the engine bus, DDJ-400 mapping, and Flutter UI.

**Architecture:** A realtime pitch factor is added to `crates/stretch` (keylock time-stretch at `tempo/pitch` plus a stateful `StreamingSincResampler` output stage). `engine-dsp` owns per-deck `key_shift_semitones`; `engine-core` exposes `SetKeyShift` / `SetPitchPage` / `SetKeyboardRoot` commands and Keyboard/KeyShift pad handlers with session-only state; the controller binds the DDJ-400 note banks; Flutter renders the page grids plus a root selector.

**Tech Stack:** Rust workspace (`cargo --manifest-path crates/Cargo.toml`), `timestretch` 0.14 (`StreamingSincResampler`), engine cmd/evt bus, `flutter_rust_bridge`, Flutter (Shad/Mixar shell).

**Spec:** `docs/keyboard-key-shift-pad-modes-design.md`

## Global Constraints

- Cargo: `cargo --manifest-path crates/Cargo.toml …` (or `cd crates`). `engine-core` default features include `backend-cpal`; tests use `backend: "null"`.
- `engine-dsp` stays pure: no I/O, no filesystem, no library access.
- Key shift, pitch page, and keyboard root are **session-only**; never write `track_*` rows or library key corrections.
- `P == 1.0` must bypass the pitch resampler (bit-identical to today).
- Engine pad slots are zero-based `0..7`; UI/DDJ-400 pads are `1..8`.
- Pitch is `2^(semitones/12)`; clamp semitones to `-16..=+16` and reject non-finite (treat as 0).
- Engine + controller are the source of truth; any input path (UI, MIDI, tests) publishes the same cmd.
- Pre-commit hook runs rustfmt/clippy for `.rs` and `dart format`/`flutter analyze` for `.dart`; do not bypass.

## Review Focus

The spec implies these inputs/failure modes that no single task's happy-path test catches; each is pinned by a test in the owning task:

1. Key shift 0 preserves today's output exactly (bypass) — Task 2.
2. Non-finite / out-of-range semitones must clamp, never panic or overflow the output buffer — Task 2.
3. Key shift with stems attached still plays stems (mixed feed), not silence — Task 2.
4. Page switching wraps `5 → 1` / `1 → 5`; empty page slots are no-ops — Task 3.
5. Unload/load resets key shift, page, and root so a new track never inherits stale state — Task 2.

---

### Task 1: Realtime pitch factor in `stretch`

**Files:**
- Modify: `crates/stretch/src/lib.rs`
- Modify: `crates/stretch/src/engine.rs`
- Test: `crates/stretch/tests/pitch_factor.rs`

**Interfaces:**
- Consumes: `timestretch::core::resample::{SincInterpTable, StreamingSincResampler}`.
- Produces:
  - `pub fn semitones_to_pitch(semitones: f32) -> f64` in `stretch`.
  - `trait TimeStretcher { …; fn set_pitch_factor(&mut self, factor: f64) {} }` (default no-op).
  - `TimestretchStretcher::set_pitch_factor(&mut self, factor: f64)` and pitch-aware `pull_interleaved`.

- [ ] **Step 1: Write the failing tests** (`crates/stretch/tests/pitch_factor.rs`):

```rust
use stretch::semitones_to_pitch;

#[test]
fn semitone_math_is_exact() {
    assert!((semitones_to_pitch(0.0) - 1.0).abs() < 1e-12);
    assert!((semitones_to_pitch(12.0) - 2.0).abs() < 1e-12);
    assert!((semitones_to_pitch(-12.0) - 0.5).abs() < 1e-12);
    assert!((semitones_to_pitch(1.0) - 1.059_463_094_359_295_4).abs() < 1e-9);
}
```

- [ ] **Step 2: Run test to verify it fails** — `cargo --manifest-path crates/Cargo.toml test -p stretch --test pitch_factor semitone_math_is_exact`. Expected: FAIL (unresolved import).

- [ ] **Step 3: Implement `semitones_to_pitch` and the trait method**:

```rust
/// Semitone offset → pitch multiplier (`2^(semitones/12)`).
pub fn semitones_to_pitch(semitones: f32) -> f64 {
    f64::from(semitones).mul_add(1.0 / 12.0, 0.0).exp2()
}
```

Add `fn set_pitch_factor(&mut self, _factor: f64) {}` to `trait TimeStretcher`.

- [ ] **Step 4: Implement pitch resampling in `TimestretchStretcher`.** Add `pitch`, two `StreamingSincResampler`s, and a `carry` buffer. `set_pitch_factor` clamps to `[0.25, 4.0]`, maps non-finite to `1.0`, and on change resets both resamplers and clears `carry`. When `(pitch - 1.0).abs() < 1e-6`, keep the current body; otherwise feed the stretcher at `tempo_rate / pitch` and resample by `pitch` into `out_frames`. `start_delay()` adds the resampler group delay; `reset()` clears carry and re-anchors.

- [ ] **Step 5: Run the tests** — `cargo --manifest-path crates/Cargo.toml test -p stretch`. Expected: PASS.

- [ ] **Step 6: Commit** — `feat(stretch): realtime pitch factor for key shift (#298)`.

---

### Task 2: `engine-api` surface + `engine-dsp` deck key shift

**Files:**
- Modify: `crates/engine-api/src/payload.rs`, `crates/engine-api/src/kind.rs`
- Modify: `crates/engine-dsp/src/deck.rs`
- Modify: `crates/engine-core/src/sync.rs` (`DeckControlState`)
- Modify: `crates/engine-core/src/engine.rs`, `crates/engine-core/src/control.rs`
- Test: `crates/engine-dsp/src/deck.rs` (`#[cfg(test)] mod tests`)
- Test: `crates/engine-core/tests/bus_key_shift.rs`

**Interfaces:**
- Consumes: `stretch::{semitones_to_pitch, TimeStretcher::set_pitch_factor}` (Task 1).
- Produces:
  - `CmdBody::SetKeyShift { semitones: f32 }`, `CmdBody::SetPitchPage { page: u8 }`, `CmdBody::SetKeyboardRoot { slot: u8 }`.
  - `Kind::SetKeyShift`, `Kind::SetPitchPage`, `Kind::SetKeyboardRoot`.
  - `EvtBody::DeckUpdated` + `DeckSnapshot` fields `key_shift: f32`, `#[serde(default = "default_pitch_page")] pitch_page: u8` (default 2), `#[serde(default)] keyboard_root_hot_cue: u8`.
  - `Deck::set_key_shift_semitones(f32)`, `Deck::key_shift_semitones() -> f32`, `Deck::pitch_factor() -> f64`.
  - `DeckControlState { key_shift_semitones: f32, pitch_page: u8, keyboard_root_hot_cue: u8 }`.
  - `Engine::set_deck_key_shift`, `set_deck_pitch_page`, `set_deck_keyboard_root`.

- [ ] **Step 1: Write the failing engine-dsp tests** (`key_shift_changes_pitch_not_tempo`, `key_shift_zero_matches_unset_output`, `key_shift_clamps_and_rejects_non_finite`, `key_shift_with_stems_keeps_stem_audio`, `key_shift_resets_on_unload`). Use a zero-crossing `estimate_frequency` helper.

- [ ] **Step 2: Run to verify failure** — `cargo --manifest-path crates/Cargo.toml test -p engine-dsp key_shift`. Expected: FAIL (`set_key_shift_semitones` not found).

- [ ] **Step 3: Implement `engine-dsp` deck key shift.** Add `key_shift_semitones: f32` (init `0.0`), `key_shift_semitones()`, `pitch_factor()`, `set_key_shift_semitones` (finite clamp to `-16..=16`). Change path selection to `want_stretch = self.key_shift_semitones != 0.0 || (self.key_lock && self.stems.is_none())`; in `play_stretched` set `stretcher.set_pitch_factor(self.pitch_factor())`; read through `mix_at_position` when stems are attached. `unload()` resets key shift to `0.0`.

- [ ] **Step 4: Run to verify pass** — `cargo --manifest-path crates/Cargo.toml test -p engine-dsp key_shift`. Expected: PASS.

- [ ] **Step 5: Write the failing engine-core bus test** (`crates/engine-core/tests/bus_key_shift.rs`): publish `SetKeyShift`, `SetPitchPage`, `SetKeyboardRoot` and assert the `DeckUpdated` fields; assert `unload` resets key shift to `0.0`, page to `2`, root to `0`.

- [ ] **Step 6: Run to verify failure** — `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_key_shift`. Expected: FAIL (new variants unresolved).

- [ ] **Step 7: Implement the `engine-api` surface.** In `payload.rs`: add `default_pitch_page()` (returns `2`); add the three `CmdBody` variants; add `#[serde(default)] key_shift: f32`, `#[serde(default = "default_pitch_page")] pitch_page: u8`, `#[serde(default)] keyboard_root_hot_cue: u8` to `DeckSnapshot` and `EvtBody::DeckUpdated`. In `kind.rs`: add the three kinds. Fix every literal `DeckSnapshot {` / `DeckUpdated {` construction site.

- [ ] **Step 8: Implement `engine-core` state, commands, snapshot.** `DeckControlState`: add `key_shift_semitones: f32`, `pitch_page: u8`, `keyboard_root_hot_cue: u8`; reset in `clear_loaded_track` (key shift `0`, page `DEFAULT_PITCH_PAGE`, root `0`). Add the three `Engine` setters (clamp page to `1..=PITCH_PAGE_COUNT`). `deck_snapshot_from_dsp` forwards the fields; `deck_snapshot_to_evt` forwards them; `control.rs` dispatch validates and handles the three kinds returning `CmdOutcome::DeckUpdated(deck_id)`.

- [ ] **Step 9: Run to verify pass** — `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_key_shift`, then `-p engine-dsp -p engine-api -p engine-core`. Expected: PASS.

- [ ] **Step 10: Commit** — `feat(engine): session key-shift, pitch page and keyboard root state (#298)`.

---

### Task 3: Keyboard & Key Shift pad modes

**Files:**
- Modify: `crates/engine-api/src/payload.rs` (`PadMode` variants), `crates/engine-api/src/kind.rs` (pad kinds)
- Modify: `crates/engine-core/src/pads.rs` (page tables), `engine.rs` (handlers + dispatch), `control.rs`
- Modify: `crates/controller/src/catalog.rs` (`PAD_MODES`, leaves), `action.rs` (leaf resolution)
- Test: `crates/engine-core/tests/bus_pad_modes_kb_keyshift.rs`
- Test: `crates/controller/src/action.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: Task 2 (`set_deck_key_shift`, `set_deck_pitch_page`, `set_deck_keyboard_root`, `DeckControlState`).
- Produces:
  - `PadMode::Keyboard`, `PadMode::KeyShift`.
  - `Kind::KeyboardPadPress/Release`, `Kind::KeyShiftPadPress/Release`.
  - `Engine::keyboard_pad_press(deck, slot, shift)`, `keyboard_pad_release(deck, slot)`, `key_shift_pad_press(deck, slot, shift)`, `key_shift_pad_release(deck, slot)`.
  - `PITCH_PAGE_COUNT`, `DEFAULT_PITCH_PAGE`, `PITCH_PAGES`, `PitchPadAction`, `pad_page_action(page, slot)`, `pitch_page_next/prev`.

- [ ] **Step 1: Write the failing engine-core test.** Build a null session with a loaded track and hot cues; then:
  - page 2 default: `KeyShiftPadPress slot 2 → key_shift == 2.0`; shift slot 6 → page 3; shift slot 7 → page 2.
  - page 5: `KeyShiftPadPress slot 0 → key_shift == 0.0` (RESET); slot 1 → `current - 1`.
  - Keyboard: select root hot cue, `KeyboardPadPress slot 4 (page 2) → key_shift == 4.0` and playhead seeks to the root; release restores the pre-press shift.
  - Keyboard shift bank slot 0 deletes hot cue 0.
  - `pitch_page_next(5) == 1`, `pitch_page_prev(1) == 5`.

- [ ] **Step 2: Run to verify failure** — `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_pad_modes_kb_keyshift`. Expected: FAIL.

- [ ] **Step 3: Implement pad modes and handlers.**
  - `pads.rs`: `PITCH_PAGE_COUNT = 5`, `DEFAULT_PITCH_PAGE = 2`, `PitchPadAction { Semitone(i8), KeyReset, SemitoneUp, SemitoneDown, KeySync, None }`, the five `PITCH_PAGES` tables, `pad_page_action`, `pitch_page_next/prev`.
  - `engine-core`: `pad_press` dispatches Keyboard/KeyShift; `key_shift_pad_press` applies the page action (shift bank: slot 6 next page, slot 7 prev); `keyboard_pad_press` uses the root hot cue, applies the page semitone (momentary), seeks to the root and plays (shift bank: slots 0–5 delete hot cue, 6/7 switch page); `keyboard_pad_release` gates off (fallback to another held pad, else restore pre-press shift and seek back to the root).
  - `control.rs`: dispatch the four kinds to the handlers (`CmdOutcome::DeckUpdated`).

- [ ] **Step 4: Run to verify pass** — `cargo --manifest-path crates/Cargo.toml test -p engine-core --test bus_pad_modes_kb_keyshift -p controller`. Expected: PASS.

- [ ] **Step 5: Add the controller action test** (page/shift leaf resolution) in `crates/controller/src/action.rs`.

- [ ] **Step 6: Run to verify pass** — `cargo --manifest-path crates/Cargo.toml test -p controller`. Expected: PASS.

- [ ] **Step 7: Commit** — `feat(deck): Keyboard and Key Shift pad modes (#298)`.

---

### Task 4: DDJ-400 mapping, session LEDs, host mirror

**Files:**
- Modify: `mappings/ddj-400/device.toml`, `mappings/ddj-400/map.toml`
- Modify: `crates/controller/src/session.rs`, `crates/controller/src/engine.rs`
- Modify: `crates/host-flutter/src/api/controller.rs` (mirror `key_shift` / `pitch_page` / `keyboard_root_hot_cue`)
- Modify: `docs/ddj-400-hardware-checklist.md`
- Test: `crates/controller/tests/ddj400_kb_keyshift.rs`

**Interfaces:**
- Consumes: Task 3 leaves; `EvtBody::DeckUpdated { key_shift, pitch_page, keyboard_root_hot_cue, .. }`.
- Produces: `MappingSession::set_deck_key_shift` / `set_deck_pitch_page` / `set_deck_keyboard_root`; matching `ControllerEngine` methods.

- [x] **Step 1: Write the failing controller test** (`ddj400_kb_keyshift.rs`): load the shipped bundle with a fake `MidiOut`; assert mode buttons dispatch `SetPadMode`, pad banks dispatch the named press kinds, and the shift bank switches pages.

- [x] **Step 2: Run to verify failure** — Expected: FAIL (bundle validation).

- [x] **Step 3: Add device + map bindings.** `catalog.rs`: register `pad_mode_keyboard`, `pad_mode_key_shift`, `keyboard_pad_1..8`, `key_shift_pad_1..8`, and the shift-bank page/delete aliases. `device.toml` / `map.toml`: bind the mode buttons (`0x69`, `0x6F`), the pad banks (`0x40–0x47`, `0x70–0x77`) and their shift banks (pad channel + 1), plus the `_led` aliases.

- [x] **Step 4: Implement session LED + host mirror.** `session.rs`: force-refresh the Keyboard / Key Shift banks on mode switch; add `set_deck_key_shift` / `set_deck_pitch_page` / `set_deck_keyboard_root` (mirror + pad LED via `apply_output_signal`); handle `CmdBody::SetKeyShift` / `SetPitchPage` / `SetKeyboardRoot` in the local-mirror match. `controller/src/engine.rs` exposes the three methods. `host-flutter/src/api/controller.rs` reads the three fields from `DeckUpdated` / `EngineStatus` and mirrors them.

- [x] **Step 5: Run to verify pass** — `cargo --manifest-path crates/Cargo.toml test -p controller`. Expected: PASS.

- [x] **Step 6: Commit** — `feat(controller): DDJ-400 Keyboard and Key Shift banks + LEDs (#298)`.

---

### Task 5: Flutter pad modes, providers, FRB

**Files:**
- Modify: `crates/host-flutter/src/api/engine.rs` (new transport methods + `EngineEvt` fields)
- Modify: `apps/gui-flutter/lib/mixer/pad_modes.dart`
- Modify: `apps/gui-flutter/lib/mixer/deck_pads_panel.dart`, `deck_pads_host.dart`
- Modify: `apps/gui-flutter/lib/mixer/pads/keyboard_pads.dart`, `pads/key_shift_pads.dart`
- Modify: `apps/gui-flutter/lib/mixer/engine_ui.dart`, `engine_providers.dart`
- Regenerate: `apps/gui-flutter/lib/src/rust/**` via `moon run gui-flutter:generate`
- Test: `apps/gui-flutter/test/pad_modes_test.dart`, `engine_ui_test.dart`, `keyboard_key_shift_pads_test.dart`

**Interfaces:**
- Consumes: Task 3/4 engine cmds/evts.
- Produces (Dart): `PadMode.keyboard`, `PadMode.keyShift`; `EngineEvt.keyShift` (`double?`), `EngineEvt.pitchPage` (`int?`), `EngineEvt.keyboardRootHotCue` (`int?`); `EngineUiSnapshot.pitchPageFor(int)`, `keyboardRootFor(int)`; providers `deckKeyShiftProvider`, `deckPitchPageProvider`, `deckKeyboardRootProvider`; engine methods `setKeyShift`, `setPitchPage`, `setKeyboardRoot`, `keyboardPadPress`, `keyboardPadRelease`, `keyShiftPadPress`, `keyShiftPadRelease`.

- [ ] **Step 1: Write the failing Dart tests** — page-table equality and `pitchPadLabel` cases (`pad_modes_test`); `pitchPage`/`keyboardRoot` apply from an updated evt (`engine_ui_test`); page-aware pad widget tests plus a root-selector test (`keyboard_key_shift_pads_test`).

- [ ] **Step 2: Run to verify failure** — `cd apps/gui-flutter && flutter test test/pad_modes_test.dart test/keyboard_key_shift_pads_test.dart`. Expected: FAIL (undefined `PitchPad` / widget params).

- [ ] **Step 3: Regenerate FRB and wire the Rust host.** `host-flutter/src/api/engine.rs`: add `set_keyboard_root` / `set_pitch_page` transport methods; extend `EngineEvt` and its `DeckUpdated` / `EngineStatus` mapping with `pitch_page` / `keyboard_root_hot_cue` (following the `key_lock` field). Run `moon run gui-flutter:generate`; confirm the generated Dart has `pitchPage`/`keyboardRootHotCue` and none of the removed scale bindings.

- [ ] **Step 4: Implement the Flutter surface.**
  - `pad_modes.dart`: `kPadModes` includes `keyboard`, `keyShift`; labels `Keys` / `Shift`; `kPitchPageCount` / `kDefaultPitchPage`; the `PitchPad` value type and `kPitchPages` tables; `pitchPage(page)` / `pitchPadLabel(pad)`.
  - `key_shift_pads.dart`: `KeyShiftPads({ required int page, required int activeSemitones, required ValueChanged<int> onPress, bool disabled })` — 8 pads labelled from `kPitchPages[page-1]`, active highlight on the matching absolute semitone (page-5 specials never highlight).
  - `keyboard_pads.dart`: `KeyboardPads({ required int page, required int rootHotCue, required List<DeckHotCue> hotCues, required ValueChanged<int> onSelectRoot, required ValueChanged<int> onPress, required ValueChanged<int> onRelease, bool disabled })` — 8 `HoldPadButton`s plus an 8-chip root selector (empty slots disabled).
  - `deck_pads_panel.dart`: constructor gains `pitchPage`, `keyboardRootHotCue`, `onSelectRoot`, `onCyclePage`; `_modeBody` renders the real grids; a tappable `PAGE n/5` indicator.
  - `deck_pads_host.dart`: `_toEnginePadMode` cases; watch `deckPitchPageProvider` / `deckKeyboardRootProvider`; wire `onSelectRoot` → `setKeyboardRoot` and `onCyclePage` → `setPitchPage`.
  - `engine_ui.dart` / `engine_providers.dart`: `pitchPages` / `keyboardRoots` maps + `copyWith` + providers + `applyEngineEvt` handling (mirror the `keyLocks` pattern).
  - Deck key display: show the resulting key read-only (analyzed key + `keyShiftSemitones`), never persisted.

- [ ] **Step 5: Run to verify pass** — `cd apps/gui-flutter && flutter test test/pad_modes_test.dart test/engine_ui_test.dart test/keyboard_key_shift_pads_test.dart`; then `flutter analyze` — no new issues.

- [ ] **Step 6: Commit** — `feat(gui): Keyboard and Key Shift pad modes in Flutter (#298)`.

---

### Task 6: Docs + full verification

**Files:**
- Modify: `docs/deck-spec.md` (§5.4 P4, §5.5 pad-mode table, §5.10, roadmap line)
- Modify: `docs/tech-spec.md` (remove semitone shift from MVP non-goals, or point to this design)
- Modify: `docs/ddj-400-hardware-checklist.md` if not done in Task 4
- Corrected: `docs/keyboard-key-shift-pad-modes-design.md`, `docs/keyboard-key-shift-pad-modes-plan.md`

- [ ] **Step 1: Update the specs.** deck-spec: mark P4 Key shift and the Keyboard / Key Shift pad modes as shipped, linking the design doc, and describe the hot-cue-rooted page model. tech-spec: note realtime key shift now exists via `stretch`'s pitch factor. checklist: reflect the bound mode buttons and page/shift banks.

- [ ] **Step 2: Run the full affected check.** `cargo --manifest-path crates/Cargo.toml test -p stretch -p engine-dsp -p engine-api -p engine-core -p controller`; `cd apps/gui-flutter && flutter test`; `npx moon ci`. Expected: PASS.

- [ ] **Step 3: Commit** — `docs(deck): ship Keyboard and Key Shift pad modes (#298)`.

## Spec coverage

| Spec item | Task |
|-----------|------|
| Realtime pitch factor | 1 |
| PadMode + engine cmds/events + snapshot | 2, 3 |
| Key Shift page layout + latch | 3 |
| Keyboard page layout + hot-cue root + momentary | 3 |
| Key lock / resulting-key interaction | 2, 3, 5 |
| No persistence as manual corrections | 2, 3, 6 |
| Pitch / pad / mode-switch / state-sync tests | 1–5 |
| DDJ-400 input, mode, LED | 4 |
| Flutter UI | 5 |
| Docs | 6 |

## Execution note

Owner preference (see `docs/stems-pad-mode-plan.md`): continue to PR without per-section review; prefer inline execution over subagent handoff unless blocked.
