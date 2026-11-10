import 'package:flutter/widgets.dart';
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
                savedAccentPalette: draft.accentPalette,
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('the setup replay card emits only the pending setup flag', (
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
    final replay = find.text('Show setup again');
    await tester.ensureVisible(replay);
    await tester.pumpAndSettle();
    await tester.tap(replay);
    await tester.pumpAndSettle();

    expect(changed, isNotNull);
    expect(changed!.onboardingCompleted, isFalse);
    // A replay is not a settings reset: every other field stays put.
    expect(
      appSettingsDirty(
        copyAppSettings(changed!, onboardingCompleted: true),
        draft,
      ),
      isFalse,
    );
  }, semanticsEnabled: false);
}
