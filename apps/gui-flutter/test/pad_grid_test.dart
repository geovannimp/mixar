import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
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

  Future<void> pumpGrid(
    WidgetTester tester, {
    required Size size,
    double devicePixelRatio = 1.0,
  }) async {
    // The test surface is 800 x 600 by default; a wider pane needs a wider one.
    tester.view.physicalSize = const Size(1200, 900);
    tester.view.devicePixelRatio = devicePixelRatio;
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

  Future<List<Rect>> pumpPads(
    WidgetTester tester, {
    required Size size,
    double devicePixelRatio = 1.0,
  }) async {
    await pumpGrid(tester, size: size, devicePixelRatio: devicePixelRatio);
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

  testWidgets('pads fill a very wide pane', (tester) async {
    final rects = await pumpPads(tester, size: const Size(1000, 300));
    // No aspect cap: a quarter of the width, half the height.
    for (final rect in rects) {
      expect(rect.width, closeTo(250, 0.5));
      expect(rect.height, closeTo(150, 0.5));
    }
    expect(rects[0].left, closeTo(0, 0.5));
    expect(rects[3].right, closeTo(1000, 0.5));
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
    expect(rects[0].width, closeTo(100, 0.5));
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

  testWidgets('a pane narrower than one device pixel paints nothing', (
    tester,
  ) async {
    for (final width in [0.0, 0.5]) {
      await pumpGrid(tester, size: Size(width, 200));
      expect(tester.takeException(), isNull, reason: 'width $width');
      expect(find.byKey(const Key('pad-0')), findsNothing);
    }
  });

  testWidgets('a half-pixel origin does not blur the seams', (tester) async {
    tester.view.physicalSize = const Size(400, 300);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    final theme = MixarThemeData.dark();
    final boundaryKey = GlobalKey();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: (context, child) => MixarTheme(data: theme, child: child!),
        home: Scaffold(
          body: RepaintBoundary(
            key: boundaryKey,
            child: Padding(
              // The compact transport column can put the grid on a half pixel.
              padding: const EdgeInsets.only(left: 0.5, top: 0.5),
              child: SizedBox(
                width: 290,
                height: 260,
                child: PadGrid(
                  children: [
                    for (var i = 0; i < 8; i++)
                      const ColoredBox(color: Color(0xFF111111)),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final boundary =
        boundaryKey.currentContext!.findRenderObject()!
            as RenderRepaintBoundary;
    final bytes = (await tester.runAsync(() async {
      final image = await boundary.toImage();
      final data = await image.toByteData(format: ui.ImageByteFormat.rawRgba);
      return data!.buffer.asUint8List();
    }))!;
    final colors = <int>{
      for (var x = 1; x < 290; x++)
        () {
          final i = (200 * 291 + x) * 4;
          return (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
        }(),
    };
    // Only the pad fill and the hairline: an antialiased seam would add a
    // blended colour and read as a two-pixel border.
    expect(colors, hasLength(2));
  });

  test('seams split the block into pixel-aligned cells', () {
    expect(padTableSeams(290, 4, 1), [0.0, 73.0, 145.0, 218.0, 290.0]);
    expect(padTableSeams(100, 4, 1), [0.0, 25.0, 50.0, 75.0, 100.0]);

    // Interior seams land on a device pixel; the outer boundary stays exact so
    // no strip of pane shows at the right edge.
    final seams = padTableSeams(123, 4, 1.25);
    expect(seams.first, 0);
    expect(seams.last, 123);
    for (final seam in seams.take(seams.length - 1)) {
      final device = seam * 1.25;
      expect(device, closeTo(device.roundToDouble(), 1e-9));
    }
  });

  testWidgets('a fractional pane leaves no strip at the edge', (tester) async {
    final rects = await pumpPads(tester, size: const Size(290.5, 260));
    expect(rects[3].right, closeTo(290.5, 0.01));
    expect(rects[7].bottom, closeTo(260, 0.01));
  });

  testWidgets('rows keep the 24 px floor at a fractional ratio', (
    tester,
  ) async {
    // 48 px of pane at 1.3 dpr snaps to 62 device pixels = 47.7 logical, so
    // the split has to be clamped rather than trusting the snap.
    final rects = await pumpPads(
      tester,
      size: const Size(200, 40),
      devicePixelRatio: 1.3,
    );
    for (final rect in rects) {
      expect(rect.height, greaterThanOrEqualTo(24 - 1e-9));
    }
    // The painted seam has to follow the clamped layout split, not re-snap it.
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
    expect(painter.rows[1], closeTo(24, 1e-9));
  });

  testWidgets('the painted seam sits on the clamped split', (tester) async {
    tester.view.physicalSize = const Size(400, 300);
    tester.view.devicePixelRatio = 1.3;
    addTearDown(tester.view.reset);
    final theme = MixarThemeData.dark();
    final boundaryKey = GlobalKey();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: (context, child) => MixarTheme(data: theme, child: child!),
        home: Scaffold(
          body: RepaintBoundary(
            key: boundaryKey,
            child: SizedBox(
              width: 200,
              height: 40,
              child: PadGrid(
                children: [
                  for (var i = 0; i < 8; i++)
                    const ColoredBox(color: Color(0xFF111111)),
                ],
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final boundary =
        boundaryKey.currentContext!.findRenderObject()!
            as RenderRepaintBoundary;
    final bytes = (await tester.runAsync(() async {
      final image = await boundary.toImage(pixelRatio: 1.3);
      final data = await image.toByteData(format: ui.ImageByteFormat.rawRgba);
      return data!.buffer.asUint8List();
    }))!;
    final imageWidth = (200 * 1.3).round();
    int pixel(int x, int y) {
      final i = (y * imageWidth + x) * 4;
      return (bytes[i] << 16) | (bytes[i + 1] << 8) | bytes[i + 2];
    }

    // The clamped 24 px split is device row 31 (24 x 1.3 = 31.2), one pixel
    // thick, with pad fill either side.
    expect(pixel(10, 31), 0x27272A);
    expect(pixel(10, 30), 0x111111);
    expect(pixel(10, 32), 0x111111);
  });

  test('the table paints interior seams only', () {
    final lines = padTableLines(
      const Size(400, 300),
      1,
      padTableSeams(400, 4, 1),
      padTableSeams(300, 2, 1),
    );
    // Three column seams and one row seam: no top (the header divider draws
    // it), bottom (the transport divider), or side frame (the deck chrome's
    // hairline) — so no edge is ever doubled.
    expect(lines, hasLength(4));
    expect([for (final line in lines) line.left], [100.0, 200.0, 300.0, 0.0]);
    expect(lines.last, const Rect.fromLTWH(0, 150, 400, 1));
  });

  test('fractional pad widths snap their seams to device pixels', () {
    // A 290 px pane gives 72.5 px pads; an unsnapped seam would blur across
    // two pixels and read as a double line.
    final lines = padTableLines(
      const Size(290, 260),
      1,
      padTableSeams(290, 4, 1),
      padTableSeams(260, 2, 1),
    );
    final verticals = [
      for (final line in lines)
        if (line.width == 1 && line.height == 260) line.left,
    ]..sort();
    expect(verticals, [73.0, 145.0, 218.0]);
    for (final x in verticals) {
      expect(x, x.roundToDouble());
    }
  });
}
