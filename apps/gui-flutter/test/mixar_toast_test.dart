import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
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
}
