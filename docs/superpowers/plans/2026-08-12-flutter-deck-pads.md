# Flutter Deck Pads Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Flutter deck Pads placeholder with Tauri-parity UI chrome (mode tabs + four mode grids + sampler bank toolbar), local state only.

**Architecture:** Mirror Tauri’s `DeckPadsPanel` + `pads/*` split. `DeckPadsPanel` owns local `PadMode`, demo hot cues, and demo sampler banks/slots. Mode widgets are presentational. No FRB/engine.

**Tech Stack:** Flutter/Dart, Forui (`FTheme`, `FButton`, `FDialog`), existing `apps/gui-flutter` mixer shell.

## Global Constraints

- UI chrome only — no engine bus / FRB pad APIs.
- Match Tauri short labels: Cue / Roll / Jump / Sample.
- Beat tables identical to `apps/gui-app/src/lib/pad-modes.ts`.
- Hot-cue accents: red→pink 8-slot palette (Tauri `HOT_CUE_ACCENTS`).
- `hasTrack: false` from `DeckPanel` disables pad actions; mode tabs remain enabled.
- No new pub dependencies.
- Prefer Forui theme colors for neutrals; hardcode hot-cue accents to match Tauri.

---

### Task 1: `pad_modes` + format helpers + tests

**Files:**
- Create: `apps/gui-flutter/lib/mixer/pad_modes.dart`
- Create: `apps/gui-flutter/lib/mixer/pad_format.dart`
- Create: `apps/gui-flutter/test/pad_modes_test.dart`

**Interfaces:**
- Produces:
  - `enum PadMode { hotCue, loopRoll, beatJump, sampler }`
  - `const kPadModes`, `padModeShortLabel(PadMode)`, `cyclePadMode(PadMode, int direction)`
  - `const kLoopRollBeats`, `const kBeatJumpForward`, `const kBeatJumpBack`
  - `String formatDeckTimeTenth(int? positionMs)`

- [ ] **Step 1: Write failing test**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/pad_modes.dart';
import 'package:gui_flutter/mixer/pad_format.dart';

void main() {
  test('pad mode order and short labels match Tauri', () {
    expect(kPadModes, [
      PadMode.hotCue,
      PadMode.loopRoll,
      PadMode.beatJump,
      PadMode.sampler,
    ]);
    expect(padModeShortLabel(PadMode.hotCue), 'Cue');
    expect(padModeShortLabel(PadMode.loopRoll), 'Roll');
    expect(padModeShortLabel(PadMode.beatJump), 'Jump');
    expect(padModeShortLabel(PadMode.sampler), 'Sample');
  });

  test('cyclePadMode wraps', () {
    expect(cyclePadMode(PadMode.hotCue, 1), PadMode.loopRoll);
    expect(cyclePadMode(PadMode.sampler, 1), PadMode.hotCue);
    expect(cyclePadMode(PadMode.hotCue, -1), PadMode.sampler);
  });

  test('beat tables match Tauri', () {
    expect(kLoopRollBeats, [1, 2, 4, 8, 16, 32, 64, 128]);
    expect(kBeatJumpForward, [1, 2, 4, 8, 16, 32, 64, 128]);
    expect(kBeatJumpBack, [-1, -2, -4, -8, -16, -32, -64, -128]);
  });

  test('formatDeckTimeTenth', () {
    expect(formatDeckTimeTenth(null), '0:00.0');
    expect(formatDeckTimeTenth(6500), '0:06.5');
    expect(formatDeckTimeTenth(125100), '2:05.1');
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/gui-flutter && mise exec -- flutter test test/pad_modes_test.dart`  
Expected: FAIL (library not found)

- [ ] **Step 3: Implement helpers**

`pad_modes.dart`: enum + consts + cycle + short labels.  
`pad_format.dart`: `formatDeckTimeTenth` — minutes, seconds, one decimal tenth from ms (floor).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/gui-flutter && mise exec -- flutter test test/pad_modes_test.dart`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/gui-flutter/lib/mixer/pad_modes.dart apps/gui-flutter/lib/mixer/pad_format.dart apps/gui-flutter/test/pad_modes_test.dart
git commit -m "feat(gui-flutter): add pad mode constants and time format"
```

---

### Task 2: Pad button + grid shell

**Files:**
- Create: `apps/gui-flutter/lib/mixer/pads/pad_button.dart`
- Create: `apps/gui-flutter/lib/mixer/pads/pad_grid.dart`

**Interfaces:**
- Consumes: Forui theme
- Produces:
  - `PadButton({ required Widget child, VoidCallback? onPress, VoidCallback? onPointerDown, VoidCallback? onPointerUp, VoidCallback? onPointerCancel, bool disabled, int? accentSlot, String? tooltip })`
  - `PadGrid({ required List<Widget> children })` — 4-column grid, 8 children

- [ ] **Step 1: Implement `PadButton`**

Neutral: muted border/bg from theme.  
When `accentSlot != null` (0–7): Tauri palette  
`red, orange, yellow, green, cyan, blue, violet, pink` at ~55% border / 20% fill.  
Min height ~44–48. Column center for child. Support pointer down/up/cancel for hold pads. Shift detection via `HardwareKeyboard.instance.logicalKeysPressed` containing shift left/right when calling optional `onPressWithShift` — simpler: pass `ValueChanged<bool shift>` or check shift inside parent click handlers.

Prefer: `onPress` receives no shift; parents call `HardwareKeyboard.instance.isLogicalKeyPressed(LogicalKeyboardKey.shiftLeft) || …shiftRight` themselves (matches Tauri event.shiftKey).

- [ ] **Step 2: Implement `PadGrid`**

`GridView.count(crossAxisCount: 4, …)` or `Wrap`/`Table` — non-scrollable, expand in parent. Gap ~6–8.

- [ ] **Step 3: Commit**

```bash
git add apps/gui-flutter/lib/mixer/pads/pad_button.dart apps/gui-flutter/lib/mixer/pads/pad_grid.dart
git commit -m "feat(gui-flutter): add pad button and grid shell"
```

---

### Task 3: Mode grid widgets

**Files:**
- Create: `apps/gui-flutter/lib/mixer/pads/hot_cue_pads.dart`
- Create: `apps/gui-flutter/lib/mixer/pads/loop_roll_pads.dart`
- Create: `apps/gui-flutter/lib/mixer/pads/beat_jump_pads.dart`
- Create: `apps/gui-flutter/lib/mixer/pads/sampler_pads.dart`

**Interfaces:**
- Consumes: `PadGrid`, `PadButton`, `pad_modes`, `pad_format`
- Produces:
  - `class DeckHotCue { int slot; String? label; int positionMs; }`
  - `HotCuePads({ required List<DeckHotCue> hotCues, bool disabled, required void Function(DeckHotCue) onTrigger, required void Function(int slot) onSave, required void Function(int slot) onDelete })`
  - `LoopRollPads({ bool disabled, required void Function(num beats) onBegin, required VoidCallback onEnd })`
  - `BeatJumpPads({ bool disabled, required void Function(num beats) onBeatJump })`
  - `class SamplerSlot { String? label; int? durationMs; bool get filled; }`
  - `class SamplerBank { String id; String name; String? playMode; }`
  - `SamplerPads({ required List<SamplerSlot> slots, required List<SamplerBank> banks, String? activeBankId, bool disabled, bool holdLike, String effectivePlayMode, required …callbacks })`

- [ ] **Step 1: Hot cue / loop roll / beat jump**

Match Tauri labels and pointer semantics. Shift+click delete/clear via `HardwareKeyboard`.

- [ ] **Step 2: Sampler pads**

Bank ◀/▶, truncated name, play-mode badge when not default `"one_shot"`, gear opens `FDialog` (or `showFDialog`) with name text + play-mode cycle, Save updates via callback. Eight pads; empty shows slot # + “sample”; filled shows accent + label/duration. No DnD.

- [ ] **Step 3: Commit**

```bash
git add apps/gui-flutter/lib/mixer/pads/
git commit -m "feat(gui-flutter): add hot cue, roll, jump, and sampler pad grids"
```

---

### Task 4: `DeckPadsPanel` + wire into `DeckPanel`

**Files:**
- Create: `apps/gui-flutter/lib/mixer/deck_pads_panel.dart`
- Modify: `apps/gui-flutter/lib/mixer/deck_panel.dart`

**Interfaces:**
- Produces: `DeckPadsPanel({ bool hasTrack = false })`
- Local state: `PadMode padMode`, `List<DeckHotCue> hotCues`, sampler banks/slots, `effectivePlayMode`
- Demo banks: two banks (“Bank 1”, “Bank 2”); slots start empty; hot cues start empty
- Save hot cue uses fake position `0` (or incrementing demo ms) since no playhead

- [ ] **Step 1: Implement `DeckPadsPanel`**

Tab row (`grid` 4 cols) + `switch (padMode)` to mode widget.  
`controlsDisabled = !hasTrack`. Sampler `disabled` uses panel-level `disabled` (false) OR `!hasTrack` — match Tauri: sampler uses `disabled` prop not `controlsDisabled`; for Flutter pass `disabled: !hasTrack` for all for simplicity unless matching Tauri exactly (sampler stays enabled without track in Tauri). **Match Tauri:** mode grids except sampler use `controlsDisabled`; sampler uses `disabled` only (tabs use `disabled`). With `hasTrack: false` and panel `disabled: false`, sampler would still be active — odd for stub. Spec: pad actions disabled when no track. Apply `controlsDisabled` to sampler too.

- [ ] **Step 2: Replace placeholder in `DeckPanel`**

Remove `_PlaceholderBox` pads grid; put `const DeckPadsPanel(hasTrack: false)` (or `Expanded(child: DeckPadsPanel(...))`) beside Jog.

- [ ] **Step 3: Run tests**

Run: `cd apps/gui-flutter && mise exec -- flutter test`  
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add apps/gui-flutter/lib/mixer/deck_pads_panel.dart apps/gui-flutter/lib/mixer/deck_panel.dart
git commit -m "feat(gui-flutter): wire Tauri-parity deck pads panel"
```

---

### Task 5: Verify + ship

- [ ] **Step 1:** `mise exec -- flutter analyze` on touched lib (or full package)
- [ ] **Step 2:** `mise exec -- flutter test`
- [ ] **Step 3:** Push branch and open PR summarizing UI-chrome port + non-goals
