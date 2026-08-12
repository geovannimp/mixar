import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/pads/pad_grid.dart';

void main() {
  test('PadGrid rejects wrong child counts', () {
    expect(
      () => PadGrid(children: const [SizedBox(), SizedBox()]),
      throwsArgumentError,
    );
    expect(
      () => PadGrid(
        children: List<Widget>.generate(8, (_) => const SizedBox()),
      ),
      returnsNormally,
    );
  });

  testWidgets('PadGrid fits both rows in a short height', (tester) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: theme.toApproximateMaterialTheme(),
        builder: (context, child) => FTheme(data: theme, child: child!),
        home: Scaffold(
          body: SizedBox(
            // Wide but short: width-only sizing would clip row 2.
            width: 400,
            height: 140,
            child: PadGrid(
              children: [
                for (var i = 1; i <= 8; i++)
                  Center(child: Text('$i', textDirection: TextDirection.ltr)),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('1'), findsOneWidget);
    expect(find.text('5'), findsOneWidget);
    expect(tester.getSize(find.text('1')).height, greaterThan(0));
    expect(tester.getTopLeft(find.text('5')).dy, lessThan(140));
  });
}
