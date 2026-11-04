import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_layout_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpPanel(
    WidgetTester tester, {
    AppSettings? draft,
    ValueChanged<AppSettings>? onChanged,
  }) async {
    final settings = draft ?? defaultAppSettings();
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          // Desktop selects keep the dropdown in-tree, so the test can pick an
          // option without mounting a dialog.
          appSettingsProvider.overrideWith(
            (ref) async => copyAppSettings(
              settings,
              selectStyle: SelectStyleSetting.desktop,
            ),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsLayoutPanel(
                draft: settings,
                onChanged: onChanged ?? (_) {},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> openSelect(WidgetTester tester) async {
    await tester.tap(find.byType(ShadSelect<WaveformPositionSetting>));
    await tester.pumpAndSettle();
  }

  testWidgets('layout panel exposes the waveform position', (tester) async {
    await pumpPanel(tester);
    expect(find.text('Waveform position'), findsOneWidget);
    expect(find.text('POSITION'), findsOneWidget);
    expect(find.text('Top'), findsOneWidget);

    await openSelect(tester);
    for (final label in [
      'Top',
      'Middle horizontal',
      'Middle vertical',
      'Bottom',
    ]) {
      expect(find.text(label), findsWidgets, reason: 'missing $label option');
    }
  }, semanticsEnabled: false);

  testWidgets('choosing middle vertical emits the position', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await openSelect(tester);
    await tester.tap(find.text('Middle vertical'));
    await tester.pumpAndSettle();

    expect(changed?.waveformPosition, WaveformPositionSetting.middleVertical);
  }, semanticsEnabled: false);

  testWidgets('hiding the waveform drops the position select', (tester) async {
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: copyAppSettings(defaultAppSettings(), showWaveform: false),
      onChanged: (next) => changed = next,
    );

    expect(find.byType(ShadSelect<WaveformPositionSetting>), findsNothing);
    expect(find.text('POSITION'), findsNothing);

    await tester.tap(find.byType(SettingsHeaderSwitch));
    await tester.pumpAndSettle();
    expect(changed?.showWaveform, isTrue);
  }, semanticsEnabled: false);

  testWidgets('the switch starts on and enables hiding', (tester) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    final toggle = tester.widget<SettingsHeaderSwitch>(
      find.byType(SettingsHeaderSwitch),
    );
    expect(toggle.value, isTrue);

    await tester.tap(find.byType(SettingsHeaderSwitch));
    await tester.pumpAndSettle();
    expect(changed?.showWaveform, isFalse);
  }, semanticsEnabled: false);
}
