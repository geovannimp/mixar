import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/widgets/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpLibrary(WidgetTester tester) async {
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
  }

  Future<void> pumpUi(WidgetTester tester, {bool meters = true}) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: copyAppSettings(
                  defaultAppSettings(),
                  showMixerMeters: meters,
                ),
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('Library panel hosts list appearance controls', (tester) async {
    await pumpLibrary(tester);
    expect(find.text('Library list'), findsOneWidget);
    expect(find.text('Dim played tracks'), findsOneWidget);
    expect(find.text('TRACK ROW LAYOUT'), findsOneWidget);
    expect(
      find.widgetWithText(SettingsSectionCard, 'Visible fields'),
      findsOne,
    );
  }, semanticsEnabled: false);

  testWidgets('UI panel keeps chrome, meters, and drawer only', (tester) async {
    await pumpUi(tester);
    expect(find.text('Chrome and controls'), findsOneWidget);
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsOneWidget);
    expect(find.text('Library drawer'), findsOneWidget);
    expect(find.text('Library list'), findsNothing);
    expect(find.text('Dim played tracks'), findsNothing);
    expect(find.text('TRACK ROW LAYOUT'), findsNothing);
    expect(find.text('Analytics'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('UI panel hides meter mode when mixer meters off', (
    tester,
  ) async {
    await pumpUi(tester, meters: false);
    expect(find.text('Mixer meters'), findsOneWidget);
    expect(find.text('METER MODE'), findsNothing);
  }, semanticsEnabled: false);
}
