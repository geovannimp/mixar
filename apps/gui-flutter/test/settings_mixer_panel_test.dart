import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_mixer_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('mixer settings exposes the realtime stems opt-in', (
    tester,
  ) async {
    final theme = MixarThemeData.dark();
    AppSettings? changed;
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: theme,
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsMixerPanel(
                draft: defaultAppSettings(),
                onChanged: (next) => changed = next,
              ),
            ),
          ),
        ),
      ),
    );

    expect(find.text('Realtime Stems'), findsOneWidget);
    final description = find.textContaining(
      'Tracks with offline stems always take priority.',
    );
    expect(description, findsOneWidget);

    // Offline stems are prioritized, and the warning to prefer them is bold.
    final text = tester.widget<Text>(description);
    final spans = (text.textSpan! as TextSpan).children!.cast<TextSpan>();
    final warning = spans.firstWhere(
      (span) => span.text?.contains('Not recommended.') ?? false,
    );
    expect(warning.style?.fontWeight, FontWeight.w700);
    expect(text.textSpan!.toPlainText(), isNot(contains('StemgenRT')));
    expect(text.textSpan!.toPlainText(), isNot(contains('default')));

    expect(defaultAppSettings().realtimeStems, isFalse);
    final realtimeSwitch = find.descendant(
      of: find.widgetWithText(SettingsSectionCard, 'Realtime Stems'),
      matching: find.byType(SettingsHeaderSwitch),
    );
    expect(realtimeSwitch, findsOneWidget);
    expect(tester.widget<SettingsHeaderSwitch>(realtimeSwitch).value, isFalse);

    await tester.tap(realtimeSwitch);
    await tester.pumpAndSettle();
    expect(changed?.realtimeStems, isTrue);
  }, semanticsEnabled: false);
}
