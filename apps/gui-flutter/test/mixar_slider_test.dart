import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
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
}
