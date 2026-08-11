# Flutter Rotary Knob Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port Tauri `RotaryKnob` to Flutter and replace MixerStrip Gain/Hi/Mid/Low circles with interactive local-state knobs.

**Architecture:** Pure Dart widget with CustomPaint for 270° travel arcs + face tick; vertical pointer drag matching Tauri math; MixerStrip holds per-band `0..1` state.

**Tech Stack:** Flutter, Forui theme colors, flutter_test.

## Global Constraints

- No new pub dependencies.
- No engine/FRB wiring.
- No keyboard handling in this slice.
- Value domain: min `0`, max `1`, center `0.5`, step `0.1/48`.
- Travel: −135° … +135°.
- Colors from Forui `context.theme`.

---

### Task 1: Pure math helpers + unit tests

**Files:**
- Create: `apps/gui-flutter/lib/mixer/rotary_knob.dart` (math + painter + widget)
- Create: `apps/gui-flutter/test/rotary_knob_test.dart`

**Interfaces:**
- Produces:
  - `const kControlNormMin = 0.0`
  - `const kControlNormMax = 1.0`
  - `const kControlNormCenter = 0.5`
  - `const kControlNormStep = 0.1 / 48.0`
  - `double valueToAngle(double value, double min, double max)`
  - `({double from, double to}) valueFillAngles(double value, double min, double max, {double? center})`
  - `double snapToStep(double value, double step)`
  - `double valueFromVerticalDrag({required double startValue, required double startY, required double clientY, required double min, required double max, required double step})`
  - `class RotaryKnob extends StatelessWidget` with props: `label`, `value`, `onValueChange`, optional `min`/`max`/`step`/`center`/`disabled`/`size`/`accentColor`/`ringColor`

- [ ] **Step 1: Write failing math tests**

```dart
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/rotary_knob.dart';

void main() {
  test('valueToAngle maps min/center/max to -135/0/135', () {
    expect(valueToAngle(0, 0, 1), -135);
    expect(valueToAngle(0.5, 0, 1), 0);
    expect(valueToAngle(1, 0, 1), 135);
  });

  test('valueFillAngles grows from center detent', () {
    final above = valueFillAngles(0.75, 0, 1, center: 0.5);
    expect(above.from, 0);
    expect(above.to, 67.5);

    final below = valueFillAngles(0.25, 0, 1, center: 0.5);
    expect(below.from, -67.5);
    expect(below.to, 0);
  });

  test('vertical drag up increases value', () {
    final next = valueFromVerticalDrag(
      startValue: 0.5,
      startY: 100,
      clientY: 100 - 36, // half of 72 → +0.5 * range
      min: 0,
      max: 1,
      step: kControlNormStep,
    );
    expect(next, closeTo(1.0, kControlNormStep));
  });
}
```

- [ ] **Step 2: Run tests — expect FAIL (library missing)**

Run: `cd apps/gui-flutter && flutter test test/rotary_knob_test.dart`

- [ ] **Step 3: Implement math + RotaryKnob widget**

Implement in `rotary_knob.dart`:
- Constants and pure functions above (mirror Tauri).
- `_RotaryKnobPainter` draws track arc (muted), value arc (`ringColor` or primary), circular face (secondary), tick (foreground) rotated to angle.
- Widget layout: column with uppercase label + square dial (`md` 36, `sm` 24).
- `Listener` / `GestureDetector` onVerticalDrag*: store startY/startValue on start; on update call `onValueChange(valueFromVerticalDrag(...))`.
- Honor `disabled`.

- [ ] **Step 4: Run math tests — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add apps/gui-flutter/lib/mixer/rotary_knob.dart apps/gui-flutter/test/rotary_knob_test.dart
git commit -m "feat(gui-flutter): add RotaryKnob with Tauri travel math"
```

---

### Task 2: Wire MixerStrip + drag widget test

**Files:**
- Modify: `apps/gui-flutter/lib/mixer/mixer_strip.dart`
- Modify: `apps/gui-flutter/test/rotary_knob_test.dart`

**Interfaces:**
- Consumes: `RotaryKnob`, `kControlNorm*`
- Produces: MixerStrip channels with local Gain/Hi/Mid/Low state defaulting to `0.5`

- [ ] **Step 1: Convert `_ChannelColumn` to StatefulWidget** holding `Map<String, double>` for `['Gain','Hi','Mid','Low']` at `0.5`; replace circle placeholders with:

```dart
RotaryKnob(
  label: name,
  value: values[name]!,
  min: kControlNormMin,
  max: kControlNormMax,
  step: kControlNormStep,
  center: kControlNormCenter,
  onValueChange: (v) => setState(() => values[name] = v),
)
```

Remove duplicate label Text above the circle — `RotaryKnob` already shows the label.

- [ ] **Step 2: Add widget test** that pumps `FTheme` + `RotaryKnob`, drags vertically, asserts `onValueChange` called with higher value.

- [ ] **Step 3: Run** `flutter test test/rotary_knob_test.dart test/widget_test.dart` — expect PASS

- [ ] **Step 4: Commit**

```bash
git add apps/gui-flutter/lib/mixer/mixer_strip.dart apps/gui-flutter/test/rotary_knob_test.dart
git commit -m "feat(gui-flutter): wire RotaryKnob into MixerStrip channels"
```

---

### Task 3: Spec/plan docs already on branch

**Files:**
- Create (if not already): `docs/superpowers/specs/2026-08-11-flutter-rotary-knob-design.md`
- Create: `docs/superpowers/plans/2026-08-11-flutter-rotary-knob.md` (this file)

- [ ] **Step 1: Ensure both docs committed**

```bash
git add docs/superpowers/specs/2026-08-11-flutter-rotary-knob-design.md docs/superpowers/plans/2026-08-11-flutter-rotary-knob.md
git commit -m "docs: Flutter rotary knob design and plan"
```
