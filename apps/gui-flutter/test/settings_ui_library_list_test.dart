import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_library_panel.dart';
import 'package:gui_flutter/settings/settings_ui_panel.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('UI panel hosts library list controls', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: defaultAppSettings(),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    expect(find.text('Library list'), findsOneWidget);
    expect(find.text('Dim played tracks'), findsOneWidget);
    expect(find.text('TRACK ROW LAYOUT'), findsOneWidget);
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('UI panel hides meter mode when mixer meters off', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: copyAppSettings(
                  defaultAppSettings(),
                  showMixerMeters: false,
                ),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('Library panel no longer hosts list controls', (tester) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          mixxxDatabasePathProvider.overrideWith((ref) async => null),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsLibraryPanel(
                draft: defaultAppSettings(),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Dim played tracks'), findsNothing);
    expect(find.text('TRACK ROW LAYOUT'), findsNothing);
  }, semanticsEnabled: false);
}
