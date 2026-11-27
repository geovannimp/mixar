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

  Future<void> pumpGrid(WidgetTester tester, {required Size size}) async {
    // The test surface is 800 x 600 by default; a wider pane needs a wider one.
    tester.view.physicalSize = const Size(1200, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    final theme = MixarThemeData.dark();
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
                for (var i = 0; i < 8; i++)
                  ColoredBox(
                    key: Key('pad-$i'),
                    color: const Color(0xFF111111),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Future<List<Rect>> pumpPads(WidgetTester tester, {required Size size}) async {
    await pumpGrid(tester, size: size);
    return [
      for (var i = 0; i < 8; i++) tester.getRect(find.byKey(Key('pad-$i'))),
    ];
  }

  testWidgets('pads tile the pane edge to edge', (tester) async {
    final rects = await pumpPads(tester, size: const Size(400, 300));
    // Four columns and two rows with no gaps: 100 wide, 150 tall.
    for (final rect in rects) {
      expect(rect.width, closeTo(100, 0.5));
      expect(rect.height, closeTo(150, 0.5));
    }
    expect(rects[0].left, closeTo(0, 0.5));
    expect(rects[0].top, closeTo(0, 0.5));
    expect(rects[3].right, closeTo(400, 0.5));
    expect(rects[7].bottom, closeTo(300, 0.5));
    expect(rects[4].top, closeTo(150, 0.5));
  });

  testWidgets('pads cap at 16:10 and centre in a very wide pane', (
    tester,
  ) async {
    final rects = await pumpPads(tester, size: const Size(1000, 300));
    // Rows take 150, so the cap holds width at 1.6 x 150.
    for (final rect in rects) {
      expect(rect.width, closeTo(240, 0.5));
      expect(rect.height, closeTo(150, 0.5));
    }
    const blockWidth = 4 * 240.0;
    expect(rects[0].left, closeTo((1000 - blockWidth) / 2, 0.5));
    expect(rects[3].right, closeTo((1000 + blockWidth) / 2, 0.5));
  });

  testWidgets('pads stretch tall in a narrow pane', (tester) async {
    final rects = await pumpPads(tester, size: const Size(200, 400));
    for (final rect in rects) {
      expect(rect.width, closeTo(50, 0.5));
      expect(rect.height, closeTo(200, 0.5));
    }
  });

  testWidgets('a short pane keeps a 24 px row and scrolls', (tester) async {
    final rects = await pumpPads(tester, size: const Size(400, 30));
    expect(rects[0].height, closeTo(24, 0.5));
    expect(rects[0].width, closeTo(38.4, 0.5));
    expect(find.byType(Scrollable), findsWidgets);
  });

  testWidgets('a pane narrower than four pads still tiles it', (tester) async {
    final rects = await pumpPads(tester, size: const Size(28, 200));
    expect(tester.takeException(), isNull);
    for (final rect in rects) {
      expect(rect.width, closeTo(7, 0.5));
      expect(rect.height, closeTo(100, 0.5));
    }
  });
}
