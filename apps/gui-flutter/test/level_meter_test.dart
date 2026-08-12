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

  testWidgets('mono one ladder; stereo two; zeros use muted', (tester) async {
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

    final boxes = tester.renderObjectList<RenderBox>(find.byType(DecoratedBox));
    expect(boxes.every((b) => b.size.width >= 6), isTrue);
    expect(boxes.every((b) => b.size.height > 0), isTrue);

    final decorations = tester.widgetList<DecoratedBox>(find.byType(DecoratedBox));
    expect(
      decorations.every((d) => (d.decoration as BoxDecoration).color == theme.colors.muted),
      isTrue,
    );
  });
}
