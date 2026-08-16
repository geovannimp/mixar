import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';

void main() {
  test('PadGrid rejects wrong child counts', () {
    expect(
      () => PadGrid(children: const [SizedBox(), SizedBox()]),
      throwsArgumentError,
    );
    expect(
      () =>
          PadGrid(children: List<Widget>.generate(8, (_) => const SizedBox())),
      returnsNormally,
    );
  });

  Future<List<Rect>> pumpPads(WidgetTester tester, {required Size size}) async {
    final theme = FTheme.neutral.dark.desktop;
    final keys = List<Key>.generate(8, (i) => Key('pad-$i'));
    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: Scaffold(
          body: SizedBox(
            width: size.width,
            height: size.height,
            child: PadGrid(
              children: [
                for (final key in keys)
                  ColoredBox(key: key, color: const Color(0xFF111111)),
              ],
            ),
          ),
        ),
      ),
    );
    return [for (final key in keys) tester.getRect(find.byKey(key))];
  }

  testWidgets('pads stay square when the pane is wide', (tester) async {
    final rects = await pumpPads(tester, size: const Size(400, 300));
    for (final rect in rects) {
      expect(rect.width, closeTo(rect.height, 0.5));
    }
    expect(rects[0].width, closeTo(90, 0.5));
    expect(rects[4].top, greaterThan(rects[0].bottom));
    expect(rects[3].left, greaterThan(rects[0].right));
  });

  testWidgets('pads stay square when the pane is tall', (tester) async {
    final rects = await pumpPads(tester, size: const Size(200, 400));
    for (final rect in rects) {
      expect(rect.width, closeTo(rect.height, 0.5));
    }
    expect(rects[0].width, closeTo(40, 0.5));
  });
}
