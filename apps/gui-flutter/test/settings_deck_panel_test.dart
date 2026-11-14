import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_deck_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_slider.dart';
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
  AppSettings faderStartDraft({required bool enabled, int threshold = 5}) =>
      copyAppSettings(
        defaultAppSettings(),
        faderStartEnabled: enabled,
        faderStartThresholdPct: threshold,
      );

  Future<void> pumpEnabledFaderStartPanel(
    WidgetTester tester, {
    required ValueChanged<AppSettings> onChanged,
  }) => pumpDeckPanel(
    tester,
    draft: faderStartDraft(enabled: true),
    onChanged: onChanged,
  );

  /// [find.text] skips offstage widgets by default; the fader-start card lives
  /// below the fold in the 800x600 test viewport, so this helper searches the
  /// full tree before scrolling it into view.
  Finder textAnywhere(String text) =>
      find.byWidgetPredicate((widget) => widget is Text && widget.data == text);

  Finder faderThresholdStepButton(AppLocalizations l10n, String glyph) =>
      find.descendant(
        of: find.widgetWithText(
          SettingsField,
          l10n.settingsDeckFaderStartThreshold.toUpperCase(),
        ),
        matching: find.widgetWithText(AppButton, glyph),
      );

  Future<void> tapFaderThresholdStep(
    WidgetTester tester,
    AppLocalizations l10n,
    String glyph,
  ) async {
    final finder = faderThresholdStepButton(l10n, glyph);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pump();
  }

  testWidgets(
    'fader start toggle reveals threshold stepper and both action pickers',
    (tester) async {
      final l10n = lookupAppLocalizations(const Locale('en'));
      AppSettings? emitted;

      await pumpDeckPanel(
        tester,
        draft: faderStartDraft(enabled: false),
        onChanged: (settings) => emitted = settings,
      );
      final titleFinder = textAnywhere(l10n.settingsDeckFaderStart);
      await tester.ensureVisible(titleFinder);
      await tester.pumpAndSettle();

      expect(find.text('5 %'), findsNothing);
      expect(find.byType(SettingsNumericStepper), findsNothing);
      expect(
        find.byType(SettingsSelect<FaderStartOnNonZeroSetting>),
        findsNothing,
      );
      expect(
        find.byType(SettingsSelect<FaderStartOnZeroSetting>),
        findsNothing,
      );

      final cardFinder = find.ancestor(
        of: titleFinder,
        matching: find.byType(SettingsSectionCard),
      );
      final switchFinder = find.descendant(
        of: cardFinder,
        matching: find.byType(SettingsHeaderSwitch),
      );
      await tester.ensureVisible(switchFinder);
      await tester.tap(switchFinder);
      await tester.pump();

      expect(emitted, isNotNull);
      expect(emitted!.faderStartEnabled, isTrue);
      expect(emitted!.faderStartThresholdPct, 5);

      await pumpDeckPanel(tester, draft: emitted!, onChanged: (_) {});
      await tester.ensureVisible(titleFinder);
      await tester.pumpAndSettle();

      expect(find.text('5 %'), findsOneWidget);
      expect(find.byType(SettingsNumericStepper), findsOneWidget);
      expect(
        find.byType(SettingsSelect<FaderStartOnNonZeroSetting>),
        findsOneWidget,
      );
      expect(
        find.byType(SettingsSelect<FaderStartOnZeroSetting>),
        findsOneWidget,
      );
      expect(switchFinder, findsOneWidget);
    },
  );

  testWidgets('fader start threshold stepper reports the stepped value', (
    tester,
  ) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    AppSettings? emitted;

    await pumpEnabledFaderStartPanel(
      tester,
      onChanged: (settings) => emitted = settings,
    );

    await tester.ensureVisible(textAnywhere(l10n.settingsDeckFaderStart));
    await tester.pumpAndSettle();
    await tapFaderThresholdStep(tester, l10n, '+');

    expect(emitted, isNotNull);
    expect(emitted!.faderStartEnabled, isTrue);
    expect(emitted!.faderStartThresholdPct, 6);
  });

  testWidgets('fader start on-non-zero picker reports the selected action', (
    tester,
  ) async {
    AppSettings? emitted;

    await pumpEnabledFaderStartPanel(
      tester,
      onChanged: (settings) => emitted = settings,
    );

    final select = tester.widget<SettingsSelect<FaderStartOnNonZeroSetting>>(
      find.byType(SettingsSelect<FaderStartOnNonZeroSetting>),
    );
    select.onChanged(FaderStartOnNonZeroSetting.togglePlay);
    await tester.pump();

    expect(emitted?.faderStartOnNonZero, FaderStartOnNonZeroSetting.togglePlay);
  });

  testWidgets('fader start on-zero picker reports the selected action', (
    tester,
  ) async {
    AppSettings? emitted;

    await pumpEnabledFaderStartPanel(
      tester,
      onChanged: (settings) => emitted = settings,
    );

    final select = tester.widget<SettingsSelect<FaderStartOnZeroSetting>>(
      find.byType(SettingsSelect<FaderStartOnZeroSetting>),
    );
    select.onChanged(FaderStartOnZeroSetting.nothing);
    await tester.pump();

    expect(emitted?.faderStartOnZero, FaderStartOnZeroSetting.nothing);
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

    expect(
      find.text(l10n.settingsDeckDefaultSamplerBankC.toUpperCase()),
      findsOneWidget,
    );
    expect(
      find.text(l10n.settingsDeckDefaultSamplerBankD.toUpperCase()),
      findsOneWidget,
    );
  });

  testWidgets('end-of-track warning slider reports the snapped seconds', (
    tester,
  ) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    AppSettings? emitted;
    await pumpDeckPanel(
      tester,
      draft: copyAppSettings(
        defaultAppSettings(),
        trackEndWarningSecs: kDefaultTrackEndWarningSecs,
      ),
      onChanged: (settings) => emitted = settings,
    );

    expect(find.text(l10n.settingsDeckEndWarning), findsOneWidget);
    expect(find.text(l10n.settingsDeckEndWarningSeconds(30)), findsOneWidget);

    final slider = find.byType(MixarSlider);
    await tester.ensureVisible(slider);
    await tester.pumpAndSettle();
    final box = tester.getRect(slider);
    await tester.tapAt(Offset(box.left + box.width * 0.75, box.center.dy));
    // Drain the double-tap recognizer's min-interval timer.
    await tester.pump(const Duration(milliseconds: 50));

    expect(emitted, isNotNull);
    expect(emitted!.trackEndWarningSecs, 45);
    expect(emitted!.trackEndWarningSecs % kTrackEndWarningSecsStep, 0);
  });

  testWidgets('the warning window only shows while the toggle is on', (
    tester,
  ) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    final off = copyAppSettings(defaultAppSettings(), trackEndWarning: false);
    await pumpDeckPanel(tester, draft: off, onChanged: (_) {});

    expect(find.text(l10n.settingsDeckEndWarning), findsOneWidget);
    expect(
      find.bySemanticsLabel(l10n.settingsDeckEndWarningToggle),
      findsOneWidget,
      reason: 'switch lives in the card header like the vinyl brake',
    );
    expect(find.byType(MixarSlider), findsNothing);

    await pumpDeckPanel(
      tester,
      draft: copyAppSettings(off, trackEndWarning: true),
      onChanged: (_) {},
    );

    expect(find.byType(MixarSlider), findsOneWidget);
    expect(find.text(l10n.settingsDeckEndWarningSeconds(30)), findsOneWidget);
  });

  testWidgets('the auto cue switch reports the toggle', (tester) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    AppSettings? emitted;
    await pumpDeckPanel(
      tester,
      draft: defaultAppSettings(),
      onChanged: (settings) => emitted = settings,
    );

    expect(l10n.settingsDeckAutoCue, isNotEmpty);
    final toggle = find.bySemanticsLabel(l10n.settingsDeckAutoCueToggle);
    await tester.ensureVisible(toggle);
    await tester.pumpAndSettle();
    await tester.tap(toggle);
    await tester.pump();

    expect(emitted, isNotNull);
    expect(emitted!.autoCue, isFalse);
  });

  testWidgets('the warning switch reports the toggle', (tester) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    AppSettings? emitted;
    await pumpDeckPanel(
      tester,
      draft: defaultAppSettings(),
      onChanged: (settings) => emitted = settings,
    );

    final toggle = find.bySemanticsLabel(l10n.settingsDeckEndWarningToggle);
    await tester.ensureVisible(toggle);
    await tester.pumpAndSettle();
    await tester.tap(toggle);
    await tester.pump();

    expect(emitted, isNotNull);
    expect(emitted!.trackEndWarning, isFalse);
    expect(
      emitted!.trackEndWarningSecs,
      kDefaultTrackEndWarningSecs,
      reason: 'turning the cue off keeps the saved window',
    );
  });
}
