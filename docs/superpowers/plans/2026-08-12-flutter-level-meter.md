# Flutter Level Meter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port Tauri `LevelMeter` visuals into Flutter as a props-driven widget and wire it into `MixerStrip` (zeros until engine bus).

**Architecture:** Pure Dart ladder math + `LevelMeter` widget in `level_meter.dart`; `MixerStrip` drops `_IdleLevelMeter` and passes `zeroDeckLevels` + local mono/stereo mode.

**Tech Stack:** Flutter, existing Forui mixer shell (no new deps).

## Global Constraints

- Visual only — no engine / FRB / store for levels
- Match Tauri: 12 segments, YELLOW_FROM=8, RED_FROM=10, hold epsilon math
- Controlled API: `LevelMeter({ levels, mode })`
- Spec: `docs/superpowers/specs/2026-08-12-flutter-level-meter-design.md`
- Do not commit unless the user asks

---

### Task 1: Ladder math + LevelMeter widget

**Files:**
- Create: `apps/gui-flutter/lib/mixer/level_meter.dart`
- Test: `apps/gui-flutter/test/level_meter_test.dart`

**Interfaces:**
- Produces: `LevelMeterMode`, `DeckLevels`, `zeroDeckLevels`, `segmentOn`, `holdSegment`, `LevelMeter`

- [ ] **Step 1: Write failing tests**

```dart
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/level_meter.dart';

void main() {
  test('segmentOn lights from bottom by threshold', () {
    expect(segmentOn(0, 0), isFalse);
    expect(segmentOn(1 / 12, 0), isTrue);
    expect(segmentOn(0.5, 5), isTrue);
    expect(segmentOn(0.5, 6), isFalse);
  });

  test('holdSegment ignores sub-threshold residual', () {
    expect(holdSegment(0), isNull);
    expect(holdSegment(1e-5), isNull);
    expect(holdSegment(1 / 12), 0);
    expect(holdSegment(1.0), 11);
  });

  testWidgets('mono one ladder; stereo two; zeros all off', (tester) async {
    final theme = FTheme.neutral.dark.desktop;

    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: const Scaffold(
          body: SizedBox(
            height: 200,
            child: Row(
              children: [
                LevelMeter(levels: zeroDeckLevels, mode: LevelMeterMode.mono),
                LevelMeter(levels: zeroDeckLevels, mode: LevelMeterMode.stereo),
              ],
            ),
          ),
        ),
      ),
    );
    expect(find.byType(LevelMeter), findsNWidgets(2));
    // mono: 12 segments; stereo: 24
    expect(find.byType(DecoratedBox), findsNWidgets(12 + 24));
  });
}
```

- [ ] **Step 2: Run tests — expect FAIL (library missing)**

Run: `cd apps/gui-flutter && flutter test test/level_meter_test.dart`

- [ ] **Step 3: Implement `level_meter.dart`**

Port Tauri `level-meter.tsx`:

```dart
enum LevelMeterMode { mono, stereo }

class DeckLevels {
  const DeckLevels({
    required this.peakL,
    required this.peakR,
    required this.peakHoldL,
    required this.peakHoldR,
  });
  final double peakL, peakR, peakHoldL, peakHoldR;
}

const zeroDeckLevels = DeckLevels(
  peakL: 0, peakR: 0, peakHoldL: 0, peakHoldR: 0,
);

const kLevelMeterSegments = 12;
const kLevelMeterYellowFrom = 8;
const kLevelMeterRedFrom = 10;

bool segmentOn(double level, int indexFromBottom) { ... }
int? holdSegment(double hold) { ... }

class LevelMeter extends StatelessWidget {
  const LevelMeter({required this.levels, required this.mode, super.key});
  final DeckLevels levels;
  final LevelMeterMode mode;
  // mono: max L/R; stereo: Row of two ladders
}
```

Colors: off `0xff27272a`; emerald `0xff10b981` @ 0.45; amber `0xfffbbf24` @ 0.45; red `0xffef4444` @ 0.50. Ladder width 6, gap 1px, radius 1. Column builds bottom→top by iterating `fromBottom` descending (or `Column` + reverse children) so index 0 is at the bottom.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cd apps/gui-flutter && flutter test test/level_meter_test.dart`

---

### Task 2: Wire MixerStrip

**Files:**
- Modify: `apps/gui-flutter/lib/mixer/mixer_strip.dart`
- Test: `apps/gui-flutter/test/widget_test.dart` (existing; run only)

**Interfaces:**
- Consumes: `LevelMeter`, `LevelMeterMode`, `zeroDeckLevels` from Task 1

- [ ] **Step 1: Replace idle meters**

- Import `level_meter.dart`
- Change `_LevelMetersColumn` to take `LevelMeterMode mode` (or map from `mono`)
- Replace both `_IdleLevelMeter` with:

```dart
LevelMeter(levels: zeroDeckLevels, mode: mode),
```

- Delete `_IdleLevelMeter`
- Keep GAIN/cue spacers; keep M/S toggle driving mode

- [ ] **Step 2: Verify shell still passes**

Run: `cd apps/gui-flutter && flutter test test/widget_test.dart test/level_meter_test.dart`

---

## Spec coverage

| Spec item | Task |
|-----------|------|
| `LevelMeter` + types + math | 1 |
| MixerStrip zeros + delete idle | 2 |
| M/S toggle | 2 (existing) |
| Unit + widget tests | 1 |
| No engine/FRB | global |
