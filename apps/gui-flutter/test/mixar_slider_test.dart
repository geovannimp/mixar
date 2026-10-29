import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/shell/mixar_slider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

/// Host whose [MixarSlider.onChanged] deliberately does not feed `value` back,
/// mirroring a parent that only adopts the dragged value later (engine echo or
/// an optimistic round-trip).
class _SliderHost extends StatefulWidget {
  const new({required this.initial, super.key});

  final double initial;

  @override
  State<_SliderHost> createState() => _SliderHostState();
}

class _SliderHostState extends State<_SliderHost> {
  late double value = widget.initial;
  double max = 1;

  /// External change, e.g. a hardware knob reaching the app as an engine echo.
  void show(double next) => setState(() => value = next);

  /// Shrinks the range while keeping the value, e.g. a control that becomes
  /// shorter: the thumb must still re-clamp.
  void shrinkRange(double nextMax) => setState(() => max = nextMax);

  /// Rebuild with the same value, e.g. an unrelated provider notification.
  void rebuildUnchanged() => setState(() {});

  @override
  Widget build(BuildContext context) => SizedBox(
    // The FX bar's LEVEL size; ShadSlider's gesture area is the track itself.
    width: 110,
    height: 16,
    child: MixarSlider(value: value, max: max, onChanged: (_) {}),
  );
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<GlobalKey<_SliderHostState>> pumpSlider(WidgetTester tester) async {
    final key = GlobalKey<_SliderHostState>();
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: Center(child: _SliderHost(key: key, initial: 0.5)),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return key;
  }

  ShadSliderController controllerOf(WidgetTester tester) =>
      tester.widget<ShadSlider>(find.byType(ShadSlider)).controller!;

  testWidgets('mirrors an external value onto the thumb', (tester) async {
    final key = await pumpSlider(tester);

    key.currentState!.show(0.9);
    await tester.pumpAndSettle();

    expect(
      controllerOf(tester).value,
      closeTo(0.9, 1e-6),
      reason: 'an engine echo must move the thumb',
    );
  });

  testWidgets('re-clamps the thumb when the range shrinks', (tester) async {
    final key = await pumpSlider(tester);

    // Same value, smaller range: an early return on the value alone would leave
    // the thumb outside 0..0.4.
    key.currentState!.shrinkRange(0.4);
    await tester.pump();

    expect(controllerOf(tester).value, closeTo(0.4, 1e-6));
  });

  testWidgets('a rebuild mid-drag keeps the thumb under the finger', (
    tester,
  ) async {
    final key = await pumpSlider(tester);

    final gesture = await tester.startGesture(
      tester.getCenter(find.byType(MixarSlider)),
    );
    // The first move only wins the gesture arena; the pan starts on the next.
    await gesture.moveBy(const Offset(40, 0));
    await gesture.moveBy(const Offset(1, 0));
    await tester.pump();

    final dragged = controllerOf(tester).value;
    expect(dragged, greaterThan(0.5), reason: 'the drag moved the thumb');

    // The parent still carries the pre-drag value; a rebuild must not pull the
    // thumb back while the gesture is live.
    key.currentState!.rebuildUnchanged();
    await tester.pump();
    expect(controllerOf(tester).value, closeTo(dragged, 1e-6));

    await gesture.up();
    await tester.pumpAndSettle();
  });

  testWidgets('a rebuild after the drag reconciles to the model', (
    tester,
  ) async {
    final key = await pumpSlider(tester);

    // The host never adopts the dragged value, like a command the engine
    // ignored (soft takeover) or a failed publish.
    await tester.drag(find.byType(MixarSlider), const Offset(40, 0));
    await tester.pumpAndSettle();
    expect(controllerOf(tester).value, greaterThan(0.5));

    key.currentState!.rebuildUnchanged();
    await tester.pump();

    expect(
      controllerOf(tester).value,
      closeTo(0.5, 1e-6),
      reason: 'outside a drag the model is authoritative',
    );
  });

  testWidgets('reserves the thumb height so the card body cannot crop it', (
    tester,
  ) async {
    const sliderWidth = 200.0;
    final theme = MixarThemeData.dark();
    final captureKey = GlobalKey();
    // The real settings card clips its body to the body's content box
    // (ClipRect + SizeTransition), so anything a field paints outside its
    // layout box is lost: the session card cut the deck-volume thumb at the
    // track's bottom edge.
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(
          body: RepaintBoundary(
            key: captureKey,
            child: SettingsSectionCard(
              title: 'Slider',
              subtitle: 'Thumb crop',
              child: SizedBox(
                width: sliderWidth,
                child: MixarSlider(value: 0.5, onChanged: (_) {}),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final rect = tester.getRect(find.byType(MixarSlider));
    expect(
      rect.height,
      28,
      reason: 'the 20px thumb plus 8px of focus ring must fit the box',
    );

    // Keyboard focus is the tallest paint: the ring adds 4 px on every side of
    // the thumb, so asserting the focused frame also covers a reservation that
    // only fits the bare thumb.
    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pumpAndSettle();
    expect(
      tester
          .widgetList<Focus>(
            find.descendant(
              of: find.byType(ShadSlider),
              matching: find.byType(Focus),
            ),
          )
          .any((focus) => focus.focusNode?.hasFocus ?? false),
      isTrue,
      reason: 'Tab must focus the slider to paint its focus ring',
    );

    final boundary =
        captureKey.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    // Rasterizing needs the real event loop; toImage deadlocks inside the
    // fake-async zone tester.runAsync would otherwise keep. Rows and stride
    // come from the image, not the render box, so a non-1 device pixel ratio
    // cannot shift the sampling window.
    final raster = await tester.runAsync(() async {
      final image = await boundary.toImage();
      try {
        final data = await image.toByteData();
        return (rgba: data!.buffer.asUint8List(), width: image.width);
      } finally {
        image.dispose();
      }
    });
    final rgba = raster!.rgba;
    final stride = raster.width * 4;
    final left = rect.left.toInt() * 4;
    final right = rect.right.toInt() * 4;
    final paintedRows = <int>{
      for (var y = rect.top.toInt(); y < rect.bottom.toInt(); y++)
        if (_hasThumbPixel(rgba, y * stride + left, right - left)) y,
    };
    expect(
      paintedRows.length,
      greaterThanOrEqualTo(26),
      reason: 'the focused thumb and ring span the whole box: $paintedRows',
    );
  });
}

/// Mixar's accent (`MixarTeal.s600`), the slider's active track and thumb ring.
bool _hasThumbPixel(Uint8List rgba, int offset, int stride) {
  for (var i = offset; i < offset + stride; i += 4) {
    final r = rgba[i];
    final g = rgba[i + 1];
    final b = rgba[i + 2];
    if (g > 150 && b > 130 && r < 90) {
      return true;
    }
  }
  return false;
}
