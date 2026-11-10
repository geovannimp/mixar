import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_library_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsLibraryPanel(
                draft: draft ?? defaultAppSettings(),
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Finder displayChoice() =>
      find.byType(SettingsVisualChoice<KeyDisplayModeSetting>);

  Finder childOf(Finder parent, String label) =>
      find.descendant(of: parent, matching: find.text(label));

  Future<void> tapOption(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  testWidgets('key display defaults to musical and colour is a select', (
    tester,
  ) async {
    await pumpPanel(tester);

    expect(
      tester
          .widget<SettingsVisualChoice<KeyDisplayModeSetting>>(displayChoice())
          .value,
      KeyDisplayModeSetting.musical,
    );
    expect(childOf(displayChoice(), 'Musical'), findsOneWidget);
    expect(childOf(displayChoice(), 'Camelot'), findsOneWidget);

    final color = tester.widget<SettingsSelect<KeyColorModeSetting>>(
      find.byType(SettingsSelect<KeyColorModeSetting>),
    );
    expect(color.value, KeyColorModeSetting.off);
    expect(color.options, KeyColorModeSetting.values);
  }, semanticsEnabled: false);

  testWidgets('tapping Camelot emits the key display mode', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await tapOption(tester, childOf(displayChoice(), 'Camelot'));

    expect(changed?.keyDisplayMode, KeyDisplayModeSetting.camelot);
  }, semanticsEnabled: false);
}
