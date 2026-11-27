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
      defaultAppSettings(),
      onboardingCompleted: true,
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
    await tester.tap(find.byType(SettingsHeaderSwitch));
    await tester.pumpAndSettle();

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
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), onboardingCompleted: false),
    );

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
      draft: copyAppSettings(defaultAppSettings(), onboardingCompleted: false),
      onChanged: (settings) => changed = settings,
    );

    await tester.tap(find.byType(SettingsHeaderSwitch));
    await tester.pumpAndSettle();

    expect(changed, isNotNull);
    expect(changed!.onboardingCompleted, isTrue);
  }, semanticsEnabled: false);
}
