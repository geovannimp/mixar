import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_deck_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

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

  /// Scrolls an off-screen stepper button into view before tapping it; the
  /// vinyl brake card sits below the fold in the 800x600 test viewport.
  Future<void> tapStep(WidgetTester tester, String glyph) async {
    final finder = find.widgetWithText(AppButton, glyph);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pump();
  }

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

  testWidgets('vinyl brake renders as its own settings card', (tester) async {
    final l10n = lookupAppLocalizations(const Locale('en'));

    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: false),
      onChanged: (_) {},
    );
    expect(find.text(l10n.settingsDeckVinylBrakePause), findsOneWidget);
    expect(
      find.bySemanticsLabel(l10n.settingsDeckVinylBrakePauseToggle),
      findsOneWidget,
    );
    expect(stopTimeLabel(l10n), findsNothing);

    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: true),
      onChanged: (_) {},
    );
    expect(find.text(l10n.settingsDeckVinylBrakePause), findsOneWidget);
    expect(
      find.bySemanticsLabel(l10n.settingsDeckVinylBrakePauseToggle),
      findsOneWidget,
      reason: 'brake toggle lives in the card header like mixer meters',
    );
    expect(stopTimeLabel(l10n), findsOneWidget);
  });

  testWidgets('stop time stepper buttons are labeled for screen readers', (
    tester,
  ) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: true, stopSecs: 0.5),
      onChanged: (_) {},
    );

    expect(
      stepButton(tester, '−').semanticsLabel,
      l10n.settingsStepperDecrease(l10n.settingsDeckVinylBrakeStopTime),
    );
    expect(
      stepButton(tester, '+').semanticsLabel,
      l10n.settingsStepperIncrease(l10n.settingsDeckVinylBrakeStopTime),
    );
  });

  testWidgets('stop time stepper reports the stepped value', (tester) async {
    AppSettings? emitted;
    await pumpDeckPanel(
      tester,
      draft: brakeDraft(brake: true, stopSecs: 0.5),
      onChanged: (settings) => emitted = settings,
    );

    await tapStep(tester, '+');

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
    await tapStep(tester, '+');
    expect(emitted!.vinylBrakeStopSecs, closeTo(0.1, 1e-6));

    emitted = null;
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
    await tapStep(tester, '+');
    expect(emitted, isNull, reason: 'no emission past the ceiling');
  });
  testWidgets('four-deck profile shows sampler defaults for decks C and D', (
    tester,
  ) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    final draft = copyAppSettings(
      defaultAppSettings(),
      numDecks: 4,
      deckDefaultSamplerBankId: [null, null, 'c', 'd'],
    );
    await pumpDeckPanel(tester, draft: draft, onChanged: (_) {});

    expect(find.text(l10n.settingsDeckCount.toUpperCase()), findsOneWidget);
    expect(
      find.text(l10n.settingsDeckDefaultSamplerBankC.toUpperCase()),
      findsOneWidget,
    );
    expect(
      find.text(l10n.settingsDeckDefaultSamplerBankD.toUpperCase()),
      findsOneWidget,
    );
  });

  testWidgets('switching to four decks adds missing sampler defaults', (
    tester,
  ) async {
    var draft = defaultAppSettings();
    await pumpDeckPanel(
      tester,
      draft: draft,
      onChanged: (next) => draft = next,
    );

    await tester.tap(find.text('2 decks'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('4 decks'));
    await tester.pumpAndSettle();

    await pumpDeckPanel(tester, draft: draft, onChanged: (_) {});
    expect(tester.takeException(), isNull);
    expect(draft.deckDefaultSamplerBankId, [null, null, null, null]);
  });
}
