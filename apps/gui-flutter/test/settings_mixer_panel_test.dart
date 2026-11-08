import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_mixer_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

/// A session measurement with the margin the host applies.
RealtimeStemsCapability _capability(double realtime) {
  return RealtimeStemsCapability(
    avgHopMs: 2.9 / realtime,
    realtime: realtime,
    hops: 100,
    modelSha: 'model-sha',
    allowed: realtime >= 1.1,
  );
}

Finder _inRealtimeSection(Finder matching) => find.descendant(
  of: find.widgetWithText(SettingsSectionCard, 'Realtime Stems'),
  matching: matching,
);

Finder _realtimeSwitch() =>
    _inRealtimeSection(find.byType(SettingsHeaderSwitch));

Finder _benchmarkButton() => _inRealtimeSection(find.byType(AppButton));

Future<void> _pumpPanel(
  WidgetTester tester, {
  required List<AppSettings> changed,
  AppSettings? draft,
  RealtimeStemsCapability? capability,
  bool running = false,
  double? fraction,
  String? benchmarkError,
  VoidCallback? onRunBenchmark,
}) async {
  await tester.pumpWidget(
    ProviderScope(
      child: mixarTestMaterialApp(
        theme: MixarThemeData.dark(),
        home: Scaffold(
          body: SingleChildScrollView(
            child: SettingsMixerPanel(
              draft: draft ?? defaultAppSettings(),
              onChanged: changed.add,
              capability: capability,
              benchmarkRunning: running,
              benchmarkFraction: fraction,
              benchmarkError: benchmarkError,
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

    await tester.ensureVisible(realtimeSwitch);
    await tester.pumpAndSettle();
    await tester.tap(realtimeSwitch);
    await tester.pumpAndSettle();
    expect(changed.single.realtimeStems, isTrue);
  }, semanticsEnabled: false);

  testWidgets('the benchmark action replaces the switch until it passes', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    final l10n = lookupAppLocalizations(const Locale('en'));
    var runs = 0;
    await _pumpPanel(tester, changed: changed, onRunBenchmark: () => runs++);

    expect(_realtimeSwitch(), findsNothing);
    expect(_benchmarkButton(), findsOneWidget);
    expect(
      find.text(l10n.settingsMixerRealtimeStemsRunBenchmark),
      findsOneWidget,
    );

    await tester.ensureVisible(_benchmarkButton());
    await tester.pumpAndSettle();
    await tester.tap(_benchmarkButton());
    expect(runs, 1);
  }, semanticsEnabled: false);

  testWidgets('a below-margin result keeps the switch hidden and shows why', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    final l10n = lookupAppLocalizations(const Locale('en'));
    await _pumpPanel(
      tester,
      changed: changed,
      capability: _capability(0.97),
      onRunBenchmark: () {},
    );

    expect(_realtimeSwitch(), findsNothing);
    expect(
      find.text(l10n.settingsMixerRealtimeStemsDeviceTooSlow),
      findsOneWidget,
    );
    expect(find.byIcon(LucideIcons.ban), findsOneWidget);
    expect(
      find.text(l10n.settingsMixerRealtimeStemsRerunBenchmark),
      findsOneWidget,
    );

    await tester.tap(_benchmarkButton());
    await tester.pumpAndSettle();
    expect(
      changed,
      isEmpty,
      reason: 'only a passing run may enable the switch',
    );
  }, semanticsEnabled: false);

  testWidgets('a passing result swaps the action for an enabled switch', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    final l10n = lookupAppLocalizations(const Locale('en'));
    await _pumpPanel(
      tester,
      changed: changed,
      capability: _capability(1.42),
      onRunBenchmark: () {},
    );

    expect(_benchmarkButton(), findsNothing);
    final realtimeSwitch = _realtimeSwitch();
    expect(realtimeSwitch, findsOneWidget);
    expect(tester.widget<SettingsHeaderSwitch>(realtimeSwitch).enabled, isTrue);
    expect(
      find.text(l10n.settingsMixerRealtimeStemsDeviceReady),
      findsOneWidget,
    );
    expect(find.byIcon(LucideIcons.badgeCheck), findsOneWidget);

    await tester.ensureVisible(realtimeSwitch);
    await tester.pumpAndSettle();
    await tester.tap(realtimeSwitch);
    await tester.pumpAndSettle();
    expect(changed.single.realtimeStems, isTrue);
  }, semanticsEnabled: false);

  testWidgets('the running benchmark disables its button and shows progress', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    await _pumpPanel(
      tester,
      changed: changed,
      capability: _capability(1.42),
      running: true,
      fraction: 0.5,
      onRunBenchmark: () {},
    );

    expect(_realtimeSwitch(), findsNothing);
    final button = tester.widget<AppButton>(_benchmarkButton());
    expect(button.onPress, isNull);
    expect(find.textContaining('50%'), findsOneWidget);
  }, semanticsEnabled: false);

  testWidgets('an enabled setting hides the benchmark action and badge', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    final l10n = lookupAppLocalizations(const Locale('en'));
    await _pumpPanel(
      tester,
      changed: changed,
      draft: copyAppSettings(defaultAppSettings(), realtimeStems: true),
      capability: _capability(1.42),
      onRunBenchmark: () {},
    );

    expect(_benchmarkButton(), findsNothing);
    expect(find.byIcon(LucideIcons.badgeCheck), findsNothing);
    expect(find.text(l10n.settingsMixerRealtimeStemsDeviceReady), findsNothing);
    final realtimeSwitch = _realtimeSwitch();
    expect(realtimeSwitch, findsOneWidget);
    expect(tester.widget<SettingsHeaderSwitch>(realtimeSwitch).value, isTrue);
  }, semanticsEnabled: false);

  testWidgets('a failed run reports the error in place of the value', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    await _pumpPanel(
      tester,
      changed: changed,
      benchmarkError: 'model download failed',
      onRunBenchmark: () {},
    );

    expect(find.textContaining('model download failed'), findsOneWidget);
    expect(_realtimeSwitch(), findsNothing);
  }, semanticsEnabled: false);

  testWidgets('crossfader card selects a preset and toggles the flags', (
    tester,
  ) async {
    final changed = <AppSettings>[];
    await _pumpPanel(tester, changed: changed);

    expect(find.text('Crossfader'), findsOneWidget);
    final select = tester.widget<SettingsSelect<String>>(
      find.byType(SettingsSelect<String>),
    );
    expect(select.options, kCrossfaderCurvePresets);
    expect(select.value, 'constant_power');
    expect(find.text('Constant power'), findsOneWidget);

    final hamster = find.text('Hamster (swap A and B)');
    await tester.ensureVisible(hamster);
    await tester.pumpAndSettle();
    await tester.tap(hamster);
    await tester.pumpAndSettle();
    expect(changed.last.crossfaderBehavior.hamster, isTrue);
    expect(changed.last.crossfaderBehavior.disabled, isFalse);

    final disable = find.text('Disable the crossfader');
    await tester.ensureVisible(disable);
    await tester.pumpAndSettle();
    await tester.tap(disable);
    await tester.pumpAndSettle();
    expect(changed.last.crossfaderBehavior.disabled, isTrue);
  }, semanticsEnabled: false);
}
