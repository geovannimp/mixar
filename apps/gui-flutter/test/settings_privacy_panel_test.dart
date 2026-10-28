import 'package:flutter/semantics.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_privacy_panel.dart';
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
              child: SettingsPrivacyPanel(
                draft: draft ?? defaultAppSettings(),
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
  }

  testWidgets('Privacy panel hosts the analytics consent card', (tester) async {
    await pumpPanel(tester);
    expect(find.text('Privacy'), findsOneWidget);
    expect(find.text('Anonymous app usage data.'), findsOneWidget);
    expect(find.text('Analytics'), findsOneWidget);
    expect(find.text('Share anonymous app data.'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('analytics setting is off by default and enables opt-in', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: defaultAppSettings(),
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
  });
}
