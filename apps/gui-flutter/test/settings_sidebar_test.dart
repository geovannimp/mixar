import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:gui_flutter/settings/settings_sidebar.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSidebar(
    WidgetTester tester, {
    required SettingsSection active,
    required ValueChanged<SettingsSection> onSelect,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      mixarTestMaterialApp(
        theme: theme,
        home: Scaffold(
          body: SettingsSidebar(active: active, onSelect: onSelect),
        ),
      ),
    );
  }

  testWidgets('lists section labels and reports onSelect', (tester) async {
    SettingsSection? selected;
    await pumpSidebar(
      tester,
      active: SettingsSection.audio,
      onSelect: (section) => selected = section,
    );

    final l10n = lookupAppLocalizations(const Locale('en'));
    for (final section in kSettingsSections) {
      expect(find.text(section.label(l10n)), findsOneWidget);
    }

    await tester.tap(find.text(SettingsSection.mixer.label(l10n)));
    await tester.pump();
    expect(selected, SettingsSection.mixer);
  });
}
