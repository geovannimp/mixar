import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';

void main() {
  Future<void> pumpHold(
    WidgetTester tester, {
    required bool disabled,
    required VoidCallback onBegin,
    required VoidCallback onEnd,
  }) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: Scaffold(
          body: SizedBox(
            width: 80,
            height: 80,
            child: HoldPadButton(
              disabled: disabled,
              onBegin: onBegin,
              onEnd: onEnd,
              child: const Text('pad'),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('HoldPadButton ends once when disabled mid-hold', (tester) async {
    var begins = 0;
    var ends = 0;

    await pumpHold(
      tester,
      disabled: false,
      onBegin: () => begins++,
      onEnd: () => ends++,
    );

    final gesture = await tester.startGesture(tester.getCenter(find.text('pad')));
    await tester.pump();
    expect(begins, 1);
    expect(ends, 0);

    await pumpHold(
      tester,
      disabled: true,
      onBegin: () => begins++,
      onEnd: () => ends++,
    );
    await tester.pump();
    expect(ends, 1);

    await gesture.up();
    await tester.pump();
    expect(ends, 1);
    expect(begins, 1);
  });

  testWidgets('HoldPadButton ends once on dispose while held', (tester) async {
    var begins = 0;
    var ends = 0;
    final theme = FTheme.neutral.dark.desktop;

    await pumpHold(
      tester,
      disabled: false,
      onBegin: () => begins++,
      onEnd: () => ends++,
    );

    await tester.startGesture(tester.getCenter(find.text('pad')));
    await tester.pump();
    expect(begins, 1);

    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: const Scaffold(body: SizedBox.shrink()),
      ),
    );
    await tester.pump();
    expect(ends, 1);
  });
}
