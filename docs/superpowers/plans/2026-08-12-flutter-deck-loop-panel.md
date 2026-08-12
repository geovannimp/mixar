# Flutter Deck Loop Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Forui-themed Tauri-parity loop panel shell behind an exclusive Pads/Loop performance rail (local state only).

**Architecture:** Extract beat-step helpers + `DeckLoopPanel`; host it in `DeckPerformancePanel` (left rail + `IndexedStack`); mount that from `DeckPanel`. No engine wiring.

**Tech Stack:** Flutter, Forui (`FButton`, `context.theme`), flutter_test.

## Global Constraints

- Forui palette/components only — no hardcoded emerald / zinc colors for active chrome
- Local state shell — no FRB / engine / library calls
- No shift-save / shift-delete in this slice
- Fewest files: panel + mount + beat-step test
- Match existing deck shell patterns (`DeckTempoPanel`, `DeckPadsPanel`)

---

### Task 1: Beat-step helpers + test

**Files:**
- Create: `apps/gui-flutter/lib/mixer/deck_loop_panel.dart` (helpers first; widget in Task 2)
- Test: `apps/gui-flutter/test/deck_loop_beats_test.dart`

**Interfaces:**
- Produces:
  - `const kAutoLoopBeats = [1, 2, 4, 8, 16, 32];`
  - `int autoLoopBeatIndex(int beats)` — index of `beats` in list, else index of `4`
  - `int stepAutoLoopBeats(int beats, int delta)` — clamp index after `delta` (±1)

- [ ] **Step 1: Write the failing test**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/deck_loop_panel.dart';

void main() {
  test('kAutoLoopBeats matches Tauri list', () {
    expect(kAutoLoopBeats, [1, 2, 4, 8, 16, 32]);
  });

  test('autoLoopBeatIndex falls back to 4', () {
    expect(autoLoopBeatIndex(4), 2);
    expect(autoLoopBeatIndex(99), 2);
  });

  test('stepAutoLoopBeats clamps at ends', () {
    expect(stepAutoLoopBeats(1, -1), 1);
    expect(stepAutoLoopBeats(1, 1), 2);
    expect(stepAutoLoopBeats(32, 1), 32);
    expect(stepAutoLoopBeats(32, -1), 16);
  });
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd apps/gui-flutter && flutter test test/deck_loop_beats_test.dart`  
Expected: FAIL (library / symbols missing)

- [ ] **Step 3: Write minimal helpers in `deck_loop_panel.dart`**

```dart
const kAutoLoopBeats = [1, 2, 4, 8, 16, 32];

int autoLoopBeatIndex(int beats) {
  final i = kAutoLoopBeats.indexOf(beats);
  return i >= 0 ? i : kAutoLoopBeats.indexOf(4);
}

int stepAutoLoopBeats(int beats, int delta) {
  final next = (autoLoopBeatIndex(beats) + delta)
      .clamp(0, kAutoLoopBeats.length - 1)
      .toInt();
  return kAutoLoopBeats[next];
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd apps/gui-flutter && flutter test test/deck_loop_beats_test.dart`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add apps/gui-flutter/lib/mixer/deck_loop_panel.dart apps/gui-flutter/test/deck_loop_beats_test.dart
git commit -m "feat(gui-flutter): add auto-loop beat step helpers"
```

---

### Task 2: `DeckLoopPanel` + `DeckPerformancePanel` mount

**Files:**
- Modify: `apps/gui-flutter/lib/mixer/deck_loop_panel.dart`
- Create: `apps/gui-flutter/lib/mixer/performance_modes.dart`
- Create: `apps/gui-flutter/lib/mixer/deck_performance_panel.dart`
- Modify: `apps/gui-flutter/lib/mixer/deck_pads_panel.dart` (`bordered` flag)
- Modify: `apps/gui-flutter/lib/mixer/deck_panel.dart`

**Interfaces:**
- Consumes: helpers from Task 1
- Produces:
  - `enum DeckPerformanceMode { pads, loop }` + `kDeckPerformanceModes`
  - `class DeckPerformancePanel` — left rail + `IndexedStack` of Pads / Loop
  - `class DeckLoopPanel extends StatefulWidget { final bool hasTrack; final bool disabled; final bool bordered; }`
  - Local state: `bool _loopActive`, `int _loopBeats` (default 4)

- [ ] **Step 1: Implement loop panel UI**

Centered vertical stack:
1. `FButton` "Loop" — toggles `_loopActive`
2. Row: `‹`, beats text, `›` — step via `stepAutoLoopBeats`
3. Row: `IN` / `OUT` — set `_loopActive = true`

Chrome (when `bordered`):
- Forui border / radius / background; active tint via `theme.colors.primary`
- `controlsDisabled = disabled || !hasTrack`
- Active buttons: `variant: .secondary`; inactive: `.outline`

- [ ] **Step 2: Implement performance panel + mount in `DeckPanel`**

```dart
const Expanded(child: DeckPerformancePanel(hasTrack: false)),
const SizedBox(width: 8),
const Expanded(child: _PlaceholderBox(...jog...)),
```

`DeckPerformancePanel` owns the Pads/Loop rail (`FButton` items for keyboard focus) and embeds `DeckPadsPanel(bordered: false)` / `DeckLoopPanel(bordered: false)`.

- [ ] **Step 3: Analyze**

Run: `cd apps/gui-flutter && dart analyze lib/mixer/deck_loop_panel.dart lib/mixer/deck_performance_panel.dart lib/mixer/deck_panel.dart`  
Expected: no issues

- [ ] **Step 4: Commit**

```bash
git add apps/gui-flutter/lib/mixer/deck_loop_panel.dart \
  apps/gui-flutter/lib/mixer/deck_performance_panel.dart \
  apps/gui-flutter/lib/mixer/performance_modes.dart \
  apps/gui-flutter/lib/mixer/deck_pads_panel.dart \
  apps/gui-flutter/lib/mixer/deck_panel.dart
git commit -m "feat(gui-flutter): add deck loop panel shell"
```

---

### Task 3: Verify + PR

- [ ] **Step 1: Run tests**

Run: `cd apps/gui-flutter && flutter test test/deck_loop_beats_test.dart test/tempo_format_test.dart`  
Expected: PASS

- [ ] **Step 2: Push branch and open PR**

Title: `feat(gui-flutter): deck loop panel shell (Tauri parity)`  
Body: summary of shell scope, Forui-only chrome, non-goals (engine, shift-save), test plan checklist.

---

## Spec coverage

| Spec item | Task |
|-----------|------|
| Pads/Loop performance rail | 2 |
| Forui chrome / active tint | 2 |
| Controls + local state | 2 |
| No shift-save/delete | 2 (omitted) |
| Beat-step unit test | 1 |
| Non-goals (engine, etc.) | omitted by design |
