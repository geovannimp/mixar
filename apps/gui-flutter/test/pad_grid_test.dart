import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_grid.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

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
    // The test surface is 800 x 600 by default; a wider pane needs a wider one.
    tester.view.physicalSize = const Size(1200, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    final theme = MixarThemeData.dark();
    final keys = List<Key>.generate(8, (i) => Key('pad-$i'));
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: (context, child) => MixarTheme(data: theme, child: child!),
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

  testWidgets('pads fill a wide pane', (tester) async {
    final rects = await pumpPads(tester, size: const Size(400, 300));
    // Inner pane is 384 x 284 after the 8 px inset: 90 wide, 138 tall.
    for (final rect in rects) {
      expect(rect.width, closeTo(90, 0.5));
      expect(rect.height, closeTo(138, 0.5));
    }
    expect(rects[0].left, closeTo(8, 0.5));
    expect(rects[0].top, closeTo(8, 0.5));
    expect(rects[3].right, closeTo(392, 0.5));
    expect(rects[7].bottom, closeTo(292, 0.5));
    expect(rects[4].top, closeTo(8 + 138 + 8, 0.5));
  });

  testWidgets('pads cap at 16:10 and centre in a very wide pane', (
    tester,
  ) async {
    final rects = await pumpPads(tester, size: const Size(1000, 300));
    // 984 x 284 inner: rows take 138, the cap holds width at 1.6 x 138.
    for (final rect in rects) {
      expect(rect.width, closeTo(220.8, 0.5));
      expect(rect.height, closeTo(138, 0.5));
    }
    const blockWidth = 4 * 220.8 + 3 * 8;
    expect(rects[0].left, closeTo(8 + (984 - blockWidth) / 2, 0.5));
    expect(rects[3].right, closeTo(8 + (984 + blockWidth) / 2, 0.5));
  });

  testWidgets('pads stretch tall in a narrow pane', (tester) async {
    final rects = await pumpPads(tester, size: const Size(200, 400));
    for (final rect in rects) {
      expect(rect.width, closeTo(40, 0.5));
      expect(rect.height, closeTo(188, 0.5));
    }
  });

  testWidgets('a short pane keeps a 24 px row and scrolls', (tester) async {
    final rects = await pumpPads(tester, size: const Size(400, 30));
    expect(rects[0].height, closeTo(24, 0.5));
    expect(rects[0].width, closeTo(38.4, 0.5));
    expect(find.byType(Scrollable), findsWidgets);
  });

  testWidgets('a pane barely wider than the gaps keeps four columns', (
    tester,
  ) async {
    final rects = await pumpPads(tester, size: const Size(60, 200));
    expect(tester.takeException(), isNull);
    for (final rect in rects) {
      expect(rect.width, closeTo(5, 0.5));
      expect(rect.height, closeTo(88, 0.5));
    }
  });
}
