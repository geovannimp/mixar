import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_deck_panel.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpDeckPanel(
    WidgetTester tester, {
    required AppSettings draft,
    required ValueChanged<AppSettings> onChanged,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          samplerBanksProvider.overrideWith(
            (ref) => Future.value(const <SamplerBankInfo>[]),
          ),
        ],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsDeckPanel(draft: draft, onChanged: onChanged),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  AppSettings brakeDraft({required bool brake, double? stopSecs}) =>
      copyAppSettings(
        defaultAppSettings(),
        vinylBrakePause: brake,
        vinylBrakeStopSecs: stopSecs ?? kDefaultVinylBrakeStopSecs,
      );

  /// `SettingsField` renders labels uppercased.
  Finder stopTimeLabel(AppLocalizations l10n) =>
      find.text(l10n.settingsDeckVinylBrakeStopTime.toUpperCase());

  AppButton stepButton(WidgetTester tester, String glyph) =>
      tester.widget<AppButton>(find.widgetWithText(AppButton, glyph));

  testWidgets('stop time stepper only shows when brake is on', (tester) async {
    final l10n = lookupAppLocalizations(const Locale('en'));

    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: false),
      onChanged: (_) {},
    );
    expect(
      stopTimeLabel(l10n),
      findsNothing,
      reason: 'stop time is meaningless while the brake is off',
    );

    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: true),
      onChanged: (_) {},
    );
    expect(stopTimeLabel(l10n), findsOneWidget);
    expect(find.text('0.50 s'), findsOneWidget);
  });

  testWidgets('stop time stepper reports the stepped value', (tester) async {
    AppSettings? emitted;
    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: true, stopSecs: 0.5),
      onChanged: (settings) => emitted = settings,
    );

    await tester.tap(find.widgetWithText(AppButton, '+'));
    await tester.pump();

    expect(emitted, isNotNull);
    expect(emitted!.vinylBrakePause, isTrue);
    expect(emitted!.vinylBrakeStopSecs, closeTo(0.55, 1e-6));
  });

  testWidgets('stop time stepper clamps to the valid config range', (
    tester,
  ) async {
    AppSettings? emitted;
    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: true, stopSecs: kMinVinylBrakeStopSecs),
      onChanged: (settings) => emitted = settings,
    );
    expect(
      stepButton(tester, '−').onPress,
      isNull,
      reason: 'below 0.05 s would fail EngineConfig::validate',
    );
    await tester.tap(find.widgetWithText(AppButton, '+'));
    await tester.pump();
    expect(emitted!.vinylBrakeStopSecs, closeTo(0.1, 1e-6));

    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: true, stopSecs: kMaxVinylBrakeStopSecs),
      onChanged: (settings) => emitted = settings,
    );
    expect(
      stepButton(tester, '+').onPress,
      isNull,
      reason: 'above 3.0 s would fail EngineConfig::validate',
    );
    final before = emitted;
    await tester.tap(find.widgetWithText(AppButton, '+'));
    await tester.pump();
    expect(emitted, same(before), reason: 'no emission past the ceiling');
  });
}
