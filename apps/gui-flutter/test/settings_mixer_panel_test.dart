import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_mixer_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// A measured device result with the margin the host applies.
RealtimeStemsCapability _capability(double realtime) {
  return RealtimeStemsCapability(
    avgHopMs: 2.9 / realtime,
    realtime: realtime,
    hops: 100,
    modelSha: 'model-sha',
    os: 'linux',
    arch: 'x86_64',
    cpuModel: 'Test CPU',
    cores: 8,
    minRealtime: 1.1,
    allowed: realtime >= 1.1,
  );
}

Finder _realtimeSwitch() => find.descendant(
  of: find.widgetWithText(SettingsSectionCard, 'Realtime Stems'),
  matching: find.byType(SettingsHeaderSwitch),
);

Future<void> _pumpPanel(
  WidgetTester tester, {
  required List<AppSettings> changed,
  RealtimeStemsCapability? capability,
  bool running = false,
  double? fraction,
  VoidCallback? onRunBenchmark,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SingleChildScrollView(
            child: SettingsMixerPanel(
              draft: defaultAppSettings(),
              onChanged: (next) => changed.add(next),
              capability: capability,
              benchmarkRunning: running,
              benchmarkFraction: fraction,
              onRunBenchmark: onRunBenchmark,
            ),
          ),
        ),
      ),
    ),
  );
}

void main() {
  testWidgets('mixer settings exposes the realtime stems opt-in', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    await _pumpPanel(tester, changed: changed);

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
    final realtimeSwitch = _realtimeSwitch();
    expect(realtimeSwitch, findsOneWidget);
    expect(tester.widget<SettingsHeaderSwitch>(realtimeSwitch).value, isFalse);

    await tester.tap(realtimeSwitch);
    await tester.pumpAndSettle();
    expect(changed.single.realtimeStems, isTrue);
  }, semanticsEnabled: false);

  testWidgets(
    'a below-margin benchmark disables the switch and offers a re-run',
    (tester) async {
      final changed = <AppSettings>[];
      final l10n = lookupAppLocalizations(const Locale('en'));
      var runs = 0;
      await _pumpPanel(
        tester,
        changed: changed,
        capability: _capability(0.97),
        onRunBenchmark: () => runs++,
      );

      final realtimeSwitch = _realtimeSwitch();
      final widget = tester.widget<SettingsHeaderSwitch>(realtimeSwitch);
      expect(widget.enabled, isFalse);
      expect(widget.value, isFalse);

      // The measured value is visible and the requirement is named.
      expect(
        find.textContaining('0.97× measured, 1.1× required'),
        findsOneWidget,
      );

      await tester.tap(realtimeSwitch);
      await tester.pumpAndSettle();
      expect(
        changed,
        isEmpty,
        reason: 'a gated switch must not change settings',
      );

      final rerun = find.widgetWithText(
        AppButton,
        l10n.settingsMixerRealtimeStemsRerunBenchmark,
      );
      expect(rerun, findsOneWidget);
      await tester.ensureVisible(rerun);
      await tester.pumpAndSettle();
      await tester.tap(rerun);
      expect(runs, 1);
    },
    semanticsEnabled: false,
  );

  testWidgets('a passing benchmark enables the switch and shows the value', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    await _pumpPanel(
      tester,
      changed: changed,
      capability: _capability(1.42),
      onRunBenchmark: () {},
    );

    final realtimeSwitch = _realtimeSwitch();
    final widget = tester.widget<SettingsHeaderSwitch>(realtimeSwitch);
    expect(widget.enabled, isTrue);
    expect(find.textContaining('1.42× realtime'), findsOneWidget);

    await tester.tap(realtimeSwitch);
    await tester.pumpAndSettle();
    expect(changed.single.realtimeStems, isTrue);
  }, semanticsEnabled: false);

  testWidgets('a running benchmark shows hop progress', (tester) async {
    final changed = <AppSettings>[];
    await _pumpPanel(
      tester,
      changed: changed,
      capability: _capability(1.42),
      running: true,
      fraction: 0.5,
      onRunBenchmark: () {},
    );

    expect(find.textContaining('50%'), findsOneWidget);
    // The switch stays disabled while the probe is in flight.
    expect(
      tester.widget<SettingsHeaderSwitch>(_realtimeSwitch()).enabled,
      isFalse,
    );
  }, semanticsEnabled: false);
}
