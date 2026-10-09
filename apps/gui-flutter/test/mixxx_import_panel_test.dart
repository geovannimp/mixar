import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

Future<void> _pump(WidgetTester tester, {required String? path}) async {
  final theme = MixarThemeData.dark();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [mixxxDatabasePathProvider.overrideWith((ref) async => path)],
      child: MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
        home: const Scaffold(body: MixxxImportPanel()),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('disables the action when no Mixxx database is found', (
    tester,
  ) async {
    await _pump(tester, path: null);
    expect(find.text('Import from Mixxx'), findsOneWidget);
    expect(
      find.text('No Mixxx library found on this computer.'),
      findsOneWidget,
    );
    expect(tester.widget<AppButton>(find.byType(AppButton)).onPress, isNull);
  }, semanticsEnabled: false);

  testWidgets('enables the action when a Mixxx database is found', (
    tester,
  ) async {
    await _pump(tester, path: '/home/me/.mixxx/mixxxdb.sqlite');
    expect(find.textContaining('Found:'), findsOneWidget);
    expect(tester.widget<AppButton>(find.byType(AppButton)).onPress, isNotNull);
  }, semanticsEnabled: false);
}
