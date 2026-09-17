import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/settings/settings_section.dart';
import 'package:gui_flutter/settings/settings_sidebar.dart';
import 'package:gui_flutter/shell/material_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpSidebar(
    WidgetTester tester, {
    required SettingsSection active,
    required ValueChanged<SettingsSection> onSelect,
  }) async {
    final theme = MixarThemeData.dark();
    await tester.pumpWidget(
      MaterialApp(
        theme: materialUiThemeFromMixar(theme),
        builder: mixarMaterialAppBuilder(theme),
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

    for (final section in kSettingsSections) {
      expect(find.text(section.label), findsOneWidget);
    }

    await tester.tap(find.text(SettingsSection.mixer.label));
    await tester.pump();
    expect(selected, SettingsSection.mixer);
  });
}
