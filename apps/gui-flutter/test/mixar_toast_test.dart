import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/theme/material_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpHome(WidgetTester tester, Widget home) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: Scaffold(body: home),
      ),
    );
  }

  testWidgets('showMixarToast displays title', (tester) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () => showMixarToast(
            context: context,
            title: const Text('Engine failed'),
            duration: const Duration(milliseconds: 200),
          ),
          child: const Text('Show'),
        ),
      ),
    );

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 50));
    expect(find.text('Engine failed'), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pumpAndSettle();
  });

  testWidgets('showMixarToast suffix dismisses persistent toast', (
    tester,
  ) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () => showMixarToast(
            context: context,
            duration: null,
            title: const Text('Controller connected'),
            suffixBuilder: (context, dismiss) =>
                GestureDetector(onTap: dismiss, child: const Text('Enable')),
          ),
          child: const Text('Show'),
        ),
      ),
    );

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('Controller connected'), findsOneWidget);

    await tester.tap(find.text('Enable'));
    await tester.pumpAndSettle();
    expect(find.text('Controller connected'), findsNothing);
  });

  testWidgets('dismissMixarToast is a no-op for unknown id', (tester) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () => dismissMixarToast(context, id: 'missing-toast'),
          child: const Text('Dismiss'),
        ),
      ),
    );

    await tester.tap(find.text('Dismiss'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('dismissMixarToast closes toast by id', (tester) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => Column(
          children: [
            GestureDetector(
              onTap: () => showMixarToast(
                context: context,
                id: 'controller-offer:TestDev Port',
                duration: null,
                title: const Text('Pioneer DDJ-400 connected'),
              ),
              child: const Text('Show'),
            ),
            GestureDetector(
              onTap: () => dismissMixarToast(
                context,
                id: 'controller-offer:TestDev Port',
              ),
              child: const Text('Disconnect'),
            ),
          ],
        ),
      ),
    );

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pumpAndSettle();
    expect(find.text('Pioneer DDJ-400 connected'), findsOneWidget);

    await tester.tap(find.text('Disconnect'));
    await tester.pumpAndSettle();
    expect(find.text('Pioneer DDJ-400 connected'), findsNothing);
  });

  testWidgets('showMixarToast stacks suffix below description', (tester) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () => showMixarToast(
            context: context,
            duration: null,
            title: const Text('Pioneer DDJ-400 connected'),
            description: const Text('Do you want to use this controller?'),
            suffixBuilder: (context, dismiss) => Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 8,
              children: [
                GestureDetector(onTap: dismiss, child: const Text('Cancel')),
                GestureDetector(onTap: dismiss, child: const Text('Enable')),
              ],
            ),
          ),
          child: const Text('Show'),
        ),
      ),
    );

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pumpAndSettle();

    expect(find.text('Cancel'), findsOneWidget);
    expect(find.text('Enable'), findsOneWidget);
    final promptBottom = tester
        .getBottomLeft(find.text('Do you want to use this controller?'))
        .dy;
    final enableTop = tester.getTopLeft(find.text('Enable')).dy;
    expect(
      enableTop,
      greaterThanOrEqualTo(promptBottom),
      reason: 'Enable must sit under the prompt, not beside it',
    );

    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(
      find.text('Pioneer DDJ-400 connected'),
      findsNothing,
      reason: 'Cancel dismiss callback must close the toast',
    );
  });
}
