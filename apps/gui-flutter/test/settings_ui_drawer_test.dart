import 'package:flutter/semantics.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    bool drawerEnabled = false,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft:
                    draft ??
                    copyAppSettings(
                      defaultAppSettings(),
                      libraryDrawerEnabled: drawerEnabled,
                    ),
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('UI panel hosts the library drawer card', (tester) async {
    await pumpPanel(tester, drawerEnabled: false);
    expect(find.text('Library drawer'), findsOneWidget);
    // Position only appears with the switch on.
    expect(find.text('POSITION'), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('UI panel shows the drawer position when enabled', (
    tester,
  ) async {
    await pumpPanel(tester, drawerEnabled: true);
    expect(find.text('POSITION'), findsOneWidget);
    expect(find.text('Bottom'), findsOneWidget);
  }, semanticsEnabled: false);
  testWidgets('analytics setting is off by default and enables opt-in', (
    tester,
  ) async {
    final baseline = defaultAppSettings();
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: baseline,
      onChanged: (settings) => changed = settings,
    );
    final toggle = find.bySemanticsLabel('Send anonymous usage data');
    expect(toggle, findsOneWidget);
    expect(
      tester.getSemantics(toggle).hasFlag(SemanticsFlag.hasToggledState),
      isTrue,
    );
    expect(
      tester.getSemantics(toggle).hasFlag(SemanticsFlag.isToggled),
      isFalse,
    );
    await tester.ensureVisible(toggle);
    await tester.tap(toggle);
    await tester.pump();
    expect(changed?.analyticsEnabled, isTrue);
  }, semanticsEnabled: true);
}
