import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';

void main() {
  test('snapTowardCenter soft-snaps near mid on 0–100 scale', () {
    expect(snapTowardCenter(50.5, 0, 100), 50);
    expect(snapTowardCenter(49.2, 0, 100), 50);
    expect(snapTowardCenter(52, 0, 100), 52);
  });

  test('snapTowardCenter scales threshold with range', () {
    expect(snapTowardCenter(0.505, 0, 1), closeTo(0.5, 1e-12));
    expect(snapTowardCenter(0.52, 0, 1), 0.52);
  });

  test('valueFromFaderPointer vertical: top is max', () {
    final maxAtTop = valueFromFaderPointer(
      local: const Offset(20, 0),
      size: const Size(40, 100),
      orientation: FaderOrientation.vertical,
      min: 0,
      max: 100,
      step: 1,
      centerNotch: false,
    );
    expect(maxAtTop, 100);

    final minAtBottom = valueFromFaderPointer(
      local: const Offset(20, 100),
      size: const Size(40, 100),
      orientation: FaderOrientation.vertical,
      min: 0,
      max: 100,
      step: 1,
      centerNotch: false,
    );
    expect(minAtBottom, 0);
  });

  test('valueFromFaderPointer horizontal: left is min', () {
    final left = valueFromFaderPointer(
      local: Offset.zero,
      size: const Size(100, 40),
      orientation: FaderOrientation.horizontal,
      min: 0,
      max: 100,
      step: 0.05,
      centerNotch: false,
    );
    expect(left, 0);

    final right = valueFromFaderPointer(
      local: const Offset(100, 20),
      size: const Size(100, 40),
      orientation: FaderOrientation.horizontal,
      min: 0,
      max: 100,
      step: 0.05,
      centerNotch: false,
    );
    expect(right, 100);
  });

  test('centerNotch snaps pointer near mid', () {
    final next = valueFromFaderPointer(
      local: const Offset(50.4, 20),
      size: const Size(100, 40),
      orientation: FaderOrientation.horizontal,
      min: 0,
      max: 100,
      step: 0.05,
      centerNotch: true,
    );
    expect(next, 50);
  });

  testWidgets('vertical drag down decreases value', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    var value = 80.0;

    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: (context, child) => MaterialUiCompatibilityBridge( // ignore: deprecated_member_use
          child: FTheme(data: theme, child: child!),
        ),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 40,
              height: 120,
              child: StatefulBuilder(
                builder: (context, setState) {
                  return FaderSlider(
                    value: value,
                    showMarkers: true,
                    accent: FaderAccent.a,
                    onValueChange: (next) => setState(() => value = next),
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );

    await tester.drag(find.byType(FaderSlider), const Offset(0, 40));
    await tester.pumpAndSettle();

    // Center (y=60) → 50; +40px down (y=100) → ~16.7, snapped to step 1.
    expect(value, closeTo(17, 1));
  });

  testWidgets('disabled fader ignores drag', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    var value = 80.0;
    var calls = 0;

    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        builder: (context, child) => MaterialUiCompatibilityBridge( // ignore: deprecated_member_use
          child: FTheme(data: theme, child: child!),
        ),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 40,
              height: 120,
              child: FaderSlider(
                value: value,
                disabled: true,
                accent: FaderAccent.a,
                onValueChange: (next) {
                  calls += 1;
                  value = next;
                },
              ),
            ),
          ),
        ),
      ),
    );

    await tester.drag(find.byType(FaderSlider), const Offset(0, 40));
    await tester.pumpAndSettle();

    expect(calls, 0);
    expect(value, 80);
  });
}
