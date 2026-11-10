import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_visual_choice.dart';
import 'package:gui_flutter/widgets/settings/settings_waveform_panel.dart';
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
      mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SingleChildScrollView(
            child: SettingsWaveformPanel(
              draft: draft ?? defaultAppSettings(),
              onChanged: onChanged ?? (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Finder choice() =>
      find.byType(SettingsVisualChoice<WaveformDisplayModeSetting>);

  Finder option(String label) =>
      find.descendant(of: choice(), matching: find.text(label));

  testWidgets('display mode lists RGB and Filtered as visual rows', (
    tester,
  ) async {
    await pumpPanel(tester);

    expect(find.text('DISPLAY MODE'), findsOneWidget);
    expect(option('RGB'), findsOneWidget);
    expect(option('Filtered'), findsOneWidget);
    expect(
      tester
          .widget<SettingsVisualChoice<WaveformDisplayModeSetting>>(choice())
          .value,
      WaveformDisplayModeSetting.rgb,
    );
  }, semanticsEnabled: false);

  testWidgets('tapping Filtered emits the filtered display mode', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    await tester.tap(option('Filtered'));
    await tester.pumpAndSettle();

    expect(changed?.waveformDisplayMode, WaveformDisplayModeSetting.filtered);
  }, semanticsEnabled: false);

  testWidgets('tapping RGB emits the spectral display mode', (tester) async {
    AppSettings? changed;
    await pumpPanel(
      tester,
      draft: copyAppSettings(
        defaultAppSettings(),
        waveformDisplayMode: WaveformDisplayModeSetting.filtered,
      ),
      onChanged: (next) => changed = next,
    );

    await tester.tap(option('RGB'));
    await tester.pumpAndSettle();

    expect(changed?.waveformDisplayMode, WaveformDisplayModeSetting.rgb);
  }, semanticsEnabled: false);

  testWidgets('the shaping switch reflects the draft and emits the flip', (
    tester,
  ) async {
    AppSettings? changed;
    await pumpPanel(tester, onChanged: (next) => changed = next);

    expect(
      tester
          .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
          .value,
      isTrue,
      reason: 'shaping ships on, as #668 drew it',
    );

    await tester.tap(find.byType(SettingsHeaderSwitch));
    await tester.pumpAndSettle();
    expect(changed?.waveformGainEqShaping, isFalse);

    // A draft with shaping off reads as an off switch.
    await tester.pumpWidget(const SizedBox());
    await pumpPanel(
      tester,
      draft: copyAppSettings(
        defaultAppSettings(),
        waveformGainEqShaping: false,
      ),
    );
    expect(
      tester
          .widget<SettingsHeaderSwitch>(find.byType(SettingsHeaderSwitch))
          .value,
      isFalse,
    );
  }, semanticsEnabled: false);
}
