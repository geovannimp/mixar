import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_ui_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  /// A saved draft, with setup done or scheduled for the next launch.
  AppSettings draftWithSetup({required bool setupDone}) =>
      copyAppSettings(defaultAppSettings(), onboardingCompleted: setupDone);

  /// The setup card is the panel's last one and its switch sits on the test
  /// surface's bottom edge, so each tap scrolls it into view first.
  Future<void> tapSetupSwitch(WidgetTester tester) async {
    final toggle = find.byType(SettingsHeaderSwitch);
    await tester.ensureVisible(toggle);
    await tester.pumpAndSettle();
    await tester.tap(toggle);
    await tester.pumpAndSettle();
  }

  Future<void> pumpPanel(
    WidgetTester tester, {
    required AppSettings draft,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsUiPanel(
                draft: draft,
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('the setup card schedules setup for the next launch', (
    tester,
  ) async {
    AppSettings? changed;
    final draft = copyAppSettings(
      draftWithSetup(setupDone: true),
      theme: ThemeSetting.dark,
    );
    await pumpPanel(
      tester,
      draft: draft,
      onChanged: (settings) => changed = settings,
    );

    expect(find.text('First-run setup'), findsOneWidget);
    expect(
      tester
          .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
          .value,
      isFalse,
      reason: 'setup is done, so nothing is scheduled',
    );
    await tapSetupSwitch(tester);

    expect(changed, isNotNull);
    expect(changed!.onboardingCompleted, isFalse);
    // Scheduling setup is not a settings reset: every other field stays put.
    expect(
      appSettingsDirty(
        copyAppSettings(changed!, onboardingCompleted: true),
        draft,
      ),
      isFalse,
    );
  }, semanticsEnabled: false);

  testWidgets('the switch reads on while setup is scheduled', (tester) async {
    await pumpPanel(tester, draft: draftWithSetup(setupDone: false));

    expect(
      tester
          .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
          .value,
      isTrue,
    );
  }, semanticsEnabled: false);

  testWidgets('turning the switch off clears the schedule', (tester) async {
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: draftWithSetup(setupDone: false),
      onChanged: (settings) => changed = settings,
    );

    await tapSetupSwitch(tester);

    expect(changed, isNotNull);
    expect(changed!.onboardingCompleted, isTrue);
  }, semanticsEnabled: false);
}
