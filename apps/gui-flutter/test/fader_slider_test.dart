import 'dart:typed_data';

import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';

import 'support/mixar_material_app.dart';

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

  test('valueFromFaderRelativeDrag moves by axis delta only', () {
    final next = valueFromFaderRelativeDrag(
      startValue: 100,
      startAxis: 4,
      currentAxis: 28,
      trackLength: 120,
      orientation: FaderOrientation.vertical,
      min: 0,
      max: 100,
      step: 1,
      centerNotch: false,
    );
    // Down +24px on 120px track → −20; no jump from start pointer position.
    expect(next, 80);
  });

  test('valueFromFaderRelativeDrag applies center snap on result', () {
    final next = valueFromFaderRelativeDrag(
      startValue: 48,
      startAxis: 50,
      currentAxis: 52,
      trackLength: 100,
      orientation: FaderOrientation.horizontal,
      min: 0,
      max: 100,
      step: 0.05,
      centerNotch: true,
    );
    expect(next, 50);
  });

  test('faderThumbHitRect keeps full thumb inside layout at max', () {
    const size = Size(40, 120);
    final painted = faderThumbRect(
      size: size,
      orientation: FaderOrientation.vertical,
      t: 1,
    );
    final hit = faderThumbHitRect(
      size: size,
      orientation: FaderOrientation.vertical,
      t: 1,
    );
    // Inset travel: thumb sits fully inside the layout box at ends.
    expect(painted.top, greaterThanOrEqualTo(0));
    expect(painted.bottom, lessThanOrEqualTo(size.height));
    expect(hit.contains(const Offset(20, 1)), isTrue);
    expect(hit.contains(const Offset(20, 60)), isFalse);
  });

  test('faderThumbHitRect keeps full thumb inside layout at min', () {
    const size = Size(40, 120);
    final painted = faderThumbRect(
      size: size,
      orientation: FaderOrientation.vertical,
      t: 0,
    );
    expect(painted.top, greaterThanOrEqualTo(0));
    expect(painted.bottom, lessThanOrEqualTo(size.height));
    expect(
      faderThumbHitRect(
        size: size,
        orientation: FaderOrientation.vertical,
        t: 0,
      ).contains(Offset(20, size.height - 1)),
      isTrue,
    );
  });

  test('thumb ends sit on inset travel (lane/marker span)', () {
    const size = Size(40, 120);
    final half = kFaderThumbV.height / 2;
    final maxCenter = faderThumbCenter(
      size: size,
      orientation: FaderOrientation.vertical,
      t: 1,
    );
    final minCenter = faderThumbCenter(
      size: size,
      orientation: FaderOrientation.vertical,
      t: 0,
    );
    expect(maxCenter.dy, half);
    expect(minCenter.dy, size.height - half);
    expect(
      faderTravelLength(size, FaderOrientation.vertical),
      size.height - kFaderThumbV.height,
    );
  });

  testWidgets('vertical drag down decreases value', (tester) async {
    final theme = MixarThemeData.dark();
    var value = 80.0;

    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
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
                    accent: DeckAccent.a,
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

    // Inset travel (120 − 12px grip = 108): center → 50; +40px → 13.
    expect(value, 13);
  });

  testWidgets('disabled fader ignores drag', (tester) async {
    final theme = MixarThemeData.dark();
    var value = 80.0;
    var calls = 0;

    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 40,
              height: 120,
              child: FaderSlider(
                value: value,
                disabled: true,
                accent: DeckAccent.a,
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

  testWidgets('thumb grab at max does not jump until move', (tester) async {
    final theme = MixarThemeData.dark();
    var value = 100.0;
    var calls = 0;

    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 40,
              height: 120,
              child: StatefulBuilder(
                builder: (context, setState) {
                  return FaderSlider(
                    value: value,
                    accent: DeckAccent.a,
                    onValueChange: (next) {
                      calls += 1;
                      setState(() => value = next);
                    },
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );

    final topLeft = tester.getTopLeft(find.byType(FaderSlider));
    // Outer half of thumb at max — top edge of the layout box.
    final thumbOuter = Offset(topLeft.dx + 20, topLeft.dy + 1);

    final gesture = await tester.startGesture(thumbOuter);
    await tester.pump();
    expect(calls, 0);
    expect(value, 100);

    // Travel length = 120 − 12 = 108; +22px → −20 (snapped).
    await gesture.moveBy(const Offset(0, 22));
    await tester.pump();
    expect(value, 80);

    await gesture.up();
  });

  testWidgets('outer half of thumb at min is grabbable', (tester) async {
    final theme = MixarThemeData.dark();
    var value = 0.0;
    var calls = 0;

    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 40,
              height: 120,
              child: StatefulBuilder(
                builder: (context, setState) {
                  return FaderSlider(
                    value: value,
                    accent: DeckAccent.a,
                    onValueChange: (next) {
                      calls += 1;
                      setState(() => value = next);
                    },
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );

    final topLeft = tester.getTopLeft(find.byType(FaderSlider));
    final thumbOuter = Offset(topLeft.dx + 20, topLeft.dy + 119);

    final gesture = await tester.startGesture(thumbOuter);
    await tester.pump();
    expect(calls, 0);
    expect(value, 0);

    await gesture.moveBy(const Offset(0, -22));
    await tester.pump();
    expect(value, 20);

    await gesture.up();
  });

  testWidgets('track press still seeks absolutely', (tester) async {
    final theme = MixarThemeData.dark();
    var value = 100.0;

    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 40,
              height: 120,
              child: StatefulBuilder(
                builder: (context, setState) {
                  return FaderSlider(
                    value: value,
                    accent: DeckAccent.a,
                    onValueChange: (next) => setState(() => value = next),
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );

    // Mid-track, away from thumb at top.
    await tester.tapAt(tester.getCenter(find.byType(FaderSlider)));
    await tester.pumpAndSettle();
    expect(value, 50);
  });

  testWidgets('the lane stops exactly under the grip at each end', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    // The lane used to extend past the thumb travel, so a fader parked at an
    // end painted a stub of empty slot next to the cap.
    for (final (value, above) in [(100.0, true), (0.0, false)]) {
      final key = GlobalKey();
      await _pumpFader(tester, theme: theme, key: key, value: value);

      final raster = await _faderRaster(tester, key);
      final box = raster.box;
      // One column down the lane centre, over an area the fader does not own:
      // it must still be the card, never the slot or its frame.
      final rows = above
          ? [for (var y = box.top - 12; y < box.top; y++) y]
          : [for (var y = box.bottom; y < box.bottom + 12; y++) y];
      for (final y in rows) {
        expect(
          raster.rgbAt(x: box.center.dx, y: y),
          rgbOf(theme.colors.card),
          reason: 'value $value painted the lane outside its box at y=$y',
        );
      }
    }
  });

  testWidgets('the horizontal plate keeps clear of its row edges', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    final key = GlobalKey();
    // The crossfader strip and the master/cue rows are 24 high; a plate that
    // fills the row leaves the slot no margin.
    await _pumpFader(
      tester,
      theme: theme,
      key: key,
      value: 50,
      size: const Size(160, 24),
      orientation: FaderOrientation.horizontal,
      step: 0.05,
    );

    final raster = await _faderRaster(tester, key);
    final box = raster.box;
    final gripX =
        box.left +
        kFaderThumbH.width / 2 +
        (box.width - kFaderThumbH.width) / 2;
    expect(
      kFaderThumbH.height,
      lessThan(box.height),
      reason: 'the plate must not span the row',
    );
    for (final y in [box.top, box.top + 1, box.bottom - 1, box.bottom - 2]) {
      expect(
        raster.rgbAt(x: gripX, y: y),
        rgbOf(theme.colors.card),
        reason: 'the plate touched the row edge at y=$y',
      );
    }
  });

  testWidgets('the lane keeps a 1px frame around the slot', (tester) async {
    final theme = MixarThemeData.dark();
    final key = GlobalKey();
    await _pumpFader(tester, theme: theme, key: key, value: 30);

    final raster = await _faderRaster(tester, key);
    final box = raster.box;
    // A row above the grip (value 30 keeps it in the lower half) shows the
    // empty lane: 1px frame, 4px recessed fill, 1px frame.
    final y = box.top + 20;
    final frame = rgbOf(theme.colors.border);
    final slot = rgbOf(theme.colors.background);
    expect(
      [
        for (var dx = -3; dx < 3; dx++)
          raster.rgbAt(x: box.center.dx + dx, y: y),
      ],
      [frame, slot, slot, slot, slot, frame],
      reason: 'the frame is the only lane outline in light mode',
    );
  });

  /// Exact-RGB pixel count over the whole capture.
  int countOf(_FaderRaster raster, Color color) {
    final (r, g, b) = rgbOf(color);
    var found = 0;
    for (var i = 0; i < raster.rgba.length; i += 4) {
      if (raster.rgba[i] == r &&
          raster.rgba[i + 1] == g &&
          raster.rgba[i + 2] == b) {
        found++;
      }
    }
    return found;
  }

  testWidgets('each deck accent paints its identity grip and quiet tint', (
    tester,
  ) async {
    const cases = <(DeckAccent, Color, Color, Color, Color)>[
      (
        DeckAccent.a,
        MixarSky.s500,
        MixarSky.s900,
        MixarSky.s300,
        MixarSky.s300,
      ),
      (
        DeckAccent.b,
        MixarRose.s500,
        MixarRose.s900,
        MixarRose.s300,
        MixarRose.s300,
      ),
      (
        DeckAccent.c,
        MixarAmber.s500,
        MixarAmber.s900,
        MixarAmber.s300,
        MixarAmber.s300,
      ),
      (
        DeckAccent.d,
        MixarViolet.s500,
        MixarViolet.s900,
        MixarViolet.s300,
        MixarViolet.s300,
      ),
    ];
    for (final (accent, darkGrip, darkTint, lightGrip, lightTint) in cases) {
      for (final (theme, grip, tint) in [
        (MixarThemeData.dark(), darkGrip, darkTint),
        (MixarThemeData.light(), lightGrip, lightTint),
      ]) {
        final key = GlobalKey();
        await _pumpFader(
          tester,
          theme: theme,
          key: key,
          value: 60,
          accent: accent,
        );
        final raster = await _faderRaster(tester, key);
        final brightness = theme.colors.brightness;
        expect(
          countOf(raster, grip),
          greaterThan(0),
          reason: '$accent grip on $brightness chrome',
        );
        expect(
          countOf(raster, tint),
          greaterThan(0),
          reason: '$accent lane tint on $brightness chrome',
        );
      }
    }
  });

  testWidgets('a neutral fader is chrome on both plates', (tester) async {
    for (final theme in [MixarThemeData.dark(), MixarThemeData.light()]) {
      final key = GlobalKey();
      await _pumpFader(tester, theme: theme, key: key, value: 40);
      final raster = await _faderRaster(tester, key);
      expect(
        countOf(raster, MixarZinc.s500),
        greaterThan(0),
        reason: 'neutral faders stay zinc-500 on ${theme.colors.brightness}',
      );
    }
  });

  testWidgets('a dark deck fader paints the dark steps, never the light 600', (
    tester,
  ) async {
    final key = GlobalKey();
    await _pumpFader(
      tester,
      theme: MixarThemeData.dark(),
      key: key,
      value: 100,
      accent: DeckAccent.a,
    );

    final raster = await _faderRaster(tester, key);
    expect(
      countOf(raster, MixarSky.s500),
      greaterThan(0),
      reason: 'grip plate takes the deck\'s dark step',
    );
    expect(
      countOf(raster, MixarSky.s900),
      greaterThan(0),
      reason: 'lane tint is the deep step, not a bright fill',
    );
    expect(
      countOf(raster, MixarSky.s600),
      0,
      reason:
          'sky-600 is the light-theme step and must not reach the dark fader',
    );
  });

  testWidgets('a light deck fader paints the light steps, never the dark 400', (
    tester,
  ) async {
    final key = GlobalKey();
    await _pumpFader(
      tester,
      theme: MixarThemeData.light(),
      key: key,
      value: 100,
      accent: DeckAccent.a,
    );

    final raster = await _faderRaster(tester, key);
    expect(
      countOf(raster, MixarSky.s300),
      greaterThan(0),
      reason: 'grip plate takes the deck\'s light step',
    );
    expect(
      countOf(raster, MixarSky.s300),
      greaterThan(0),
      reason: 'lane tint is the pale step, not a bright fill',
    );
    expect(
      countOf(raster, MixarSky.s400),
      0,
      reason:
          'sky-400 is the dark-theme step and must not reach the light fader',
    );
  });
}

void _ignore(double _) {}

/// One fader on a card-coloured boundary, padded so paint outside the fader's
/// box still lands inside the capture. Defaults to the 40x120 vertical case.
Future<void> _pumpFader(
  WidgetTester tester, {
  required MixarThemeData theme,
  required GlobalKey key,
  required double value,
  Size size = const Size(40, 120),
  FaderOrientation orientation = FaderOrientation.vertical,
  double step = 1,
  DeckAccent accent = DeckAccent.neutral,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: materialUiThemeFromMixar(theme),
      builder: mixarMaterialAppBuilder(theme),
      home: Scaffold(
        body: Center(
          child: RepaintBoundary(
            key: key,
            child: ColoredBox(
              color: theme.colors.card,
              child: Padding(
                padding: const EdgeInsets.all(12),
                child: SizedBox.fromSize(
                  size: size,
                  child: FaderSlider(
                    value: value,
                    orientation: orientation,
                    step: step,
                    accent: accent,
                    onValueChange: _ignore,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// The fader's box, its boundary origin and the boundary pixels, so a global
/// coordinate maps straight onto the raster without repeating the arithmetic.
typedef _FaderRaster = ({Uint8List rgba, int width, Rect box, Offset origin});

extension on _FaderRaster {
  int _offsetAt(double x, double y) =>
      (((y - origin.dy) * width).toInt() + (x - origin.dx).toInt()) * 4;

  (int, int, int) rgbAt({required double x, required double y}) {
    final offset = _offsetAt(x, y);
    return (rgba[offset], rgba[offset + 1], rgba[offset + 2]);
  }
}

/// Palette/Scheme colour as the 8-bit triple the raster reports.
(int, int, int) rgbOf(Color color) =>
    ((color.r * 255).round(), (color.g * 255).round(), (color.b * 255).round());

Future<_FaderRaster> _faderRaster(WidgetTester tester, GlobalKey key) async {
  final box = tester.getRect(find.byType(FaderSlider));
  final origin = tester.getTopLeft(find.byKey(key));
  final boundary =
      key.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  // Rasterizing needs the real event loop; toImage deadlocks inside the
  // fake-async zone tester.runAsync would otherwise keep.
  final raster = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = await image.toByteData();
      return (rgba: data!.buffer.asUint8List(), width: image.width);
    } finally {
      image.dispose();
    }
  });
  return (rgba: raster!.rgba, width: raster.width, box: box, origin: origin);
}
