import 'package:flutter_smart_dialog/flutter_smart_dialog.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_toast.dart';
import 'package:material_ui/material_ui.dart';

import 'support/forui_material_app.dart';

void main() {
  Future<void> pumpHome(WidgetTester tester, Widget home) async {
    final theme = FTheme.neutral.dark.desktop;
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromForui(theme),
        navigatorObservers: [FlutterSmartDialog.observer],
        builder: FlutterSmartDialog.init(
          builder: foruiMaterialAppBuilder(theme),
        ),
        home: Scaffold(body: home),
      ),
    );
  }

  tearDown(() async {
    await SmartDialog.dismiss(status: SmartStatus.allDialog, force: true);
  });

  testWidgets('showMixarToast displays title', (tester) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () => showMixarToast(
            title: const Text('Engine failed'),
            duration: const Duration(milliseconds: 200),
          ),
          child: const Text('Show'),
        ),
      ),
    );

    await tester.tap(find.text('Show'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.text('Engine failed'), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 200));
  });

  testWidgets('showMixarToast suffix dismisses persistent toast', (
    tester,
  ) async {
    await pumpHome(
      tester,
      Builder(
        builder: (context) => GestureDetector(
          onTap: () => showMixarToast(
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
    await tester.pump(const Duration(milliseconds: 250));
    expect(find.text('Controller connected'), findsOneWidget);

    await tester.ensureVisible(find.text('Enable'));
    await tester.tap(find.text('Enable'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    expect(find.text('Controller connected'), findsNothing);
  });
}
