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

  testWidgets('pads tile a fractional pane without gaps', (tester) async {
    final rects = await pumpPads(tester, size: const Size(290, 260));
    for (var column = 1; column < 4; column++) {
      expect(rects[column].left, closeTo(rects[column - 1].right, 0.01));
      expect(rects[column].width, anyOf(72.0, 73.0));
    }
    expect(rects[4].left, closeTo(rects[0].left, 0.01));
    expect(rects[4].top, closeTo(rects[0].bottom, 0.01));
    expect(rects[7].right, closeTo(290, 0.01));
    expect(rects[7].bottom, closeTo(260, 0.01));
  });

  test('seams split the block into pixel-aligned cells', () {
    expect(padTableSeams(290, 4, 1), [0.0, 73.0, 145.0, 218.0, 290.0]);
    expect(padTableSeams(100, 4, 1), [0.0, 25.0, 50.0, 75.0, 100.0]);

    // At a fractional ratio every seam still lands on a device pixel and the
    // block never grows past the space it was given.
    final seams = padTableSeams(123, 4, 1.25);
    expect(seams.first, 0);
    expect(seams.last, lessThanOrEqualTo(123));
    for (final seam in seams) {
      final device = seam * 1.25;
      expect(device, closeTo(device.roundToDouble(), 1e-9));
    }
  });

  testWidgets('a flush grid leaves its outer frames to the chrome', (
    tester,
  ) async {
    await pumpGrid(tester, size: const Size(400, 300));
    final painter = tester
        .widgetList<CustomPaint>(
          find.descendant(
            of: find.byType(PadGrid),
            matching: find.byType(CustomPaint),
          ),
        )
        .map((paint) => paint.painter)
        .whereType<PadTablePainter>()
        .single;
    // The deck chrome already draws the left and right hairlines on a flush
    // block; painting ours beside them would read as a two-pixel border.
    expect(painter.leftFrame, isFalse);
    expect(painter.rightFrame, isFalse);
  });

  testWidgets('a capped grid frames the sides it leaves empty', (tester) async {
    // 1000 x 300: the 16:10 cap narrows the block to 960 and centres it.
    await pumpGrid(tester, size: const Size(1000, 300));
    final painter = tester
        .widgetList<CustomPaint>(
          find.descendant(
            of: find.byType(PadGrid),
            matching: find.byType(CustomPaint),
          ),
        )
        .map((paint) => paint.painter)
        .whereType<PadTablePainter>()
        .single;
    expect(painter.leftFrame, isTrue);
    expect(painter.rightFrame, isTrue);
  });

  test('the table seams frame the block and divide it evenly', () {
    final lines = padTableLines(const Size(400, 300), 1, 1);
    // Left and right frames, three column seams, one row seam.
    expect(lines, hasLength(6));
    expect(lines[0], const Rect.fromLTWH(0, 0, 1, 300));
    expect(lines[1], const Rect.fromLTWH(399, 0, 1, 300));
    expect([for (var i = 2; i < 5; i++) lines[i].left], [100.0, 200.0, 300.0]);
    expect(lines[5], const Rect.fromLTWH(0, 150, 400, 1));
    // No top seam (the header divider draws it) and no bottom seam (the
    // transport divider does), so the frame never doubles.
    final horizontals = lines.where((line) => line.height == 1);
    expect(horizontals, hasLength(1));

    // A flush block drops both frames and keeps the interior seams.
    final bare = padTableLines(
      const Size(400, 300),
      1,
      1,
      leftFrame: false,
      rightFrame: false,
    );
    expect(bare, hasLength(4));
  });

  test('fractional pad widths snap their seams to device pixels', () {
    // A 290 px pane gives 72.5 px pads; an unsnapped seam would blur across
    // two pixels and read as a double line.
    final lines = padTableLines(const Size(290, 260), 1, 1);
    final verticals = [
      for (final line in lines)
        if (line.width == 1 && line.height == 260) line.left,
    ]..sort();
    expect(verticals, [0.0, 73.0, 145.0, 218.0, 289.0]);
    for (final x in verticals) {
      expect(x, x.roundToDouble());
    }
  });
}
