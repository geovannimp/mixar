import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
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
      clientY: 100 - 36,
      min: 0,
      max: 1,
      step: kControlNormStep,
    );
    expect(next, closeTo(1.0, kControlNormStep));
  });

  test('snapToStep keeps non-zero min on the step grid', () {
    expect(snapToStep(0.25, 0.1, origin: 0.25), 0.25);
    expect(snapToStep(0.29, 0.1, origin: 0.25), 0.25);
    expect(snapToStep(0.31, 0.1, origin: 0.25), closeTo(0.35, 1e-12));
  });

  test('valueToAngle rejects invalid ranges', () {
    expect(() => valueToAngle(0, 1, 1), throwsArgumentError);
    expect(() => valueToAngle(0, double.nan, 1), throwsArgumentError);
  });

  testWidgets('drag updates value via onValueChange', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    var value = 0.5;

    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: Scaffold(
          body: Center(
            child: StatefulBuilder(
              builder: (context, setState) {
                return RotaryKnob(
                  label: 'Hi',
                  value: value,
                  center: kControlNormCenter,
                  onValueChange: (next) => setState(() => value = next),
                );
              },
            ),
          ),
        ),
      ),
    );

    final dial = find.byType(CustomPaint).last;
    await tester.drag(dial, const Offset(0, -36));
    await tester.pumpAndSettle();

    expect(value, greaterThan(0.5));
  });
}
