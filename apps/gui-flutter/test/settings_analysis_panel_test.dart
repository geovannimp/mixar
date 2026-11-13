import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_analysis_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

import 'support/mixar_material_app.dart';

void main() {
  Future<void> pumpAnalysisPanel(
    WidgetTester tester, {
    required AppSettings draft,
    required ValueChanged<AppSettings> onChanged,
  }) async {
    await tester.pumpWidget(
      ProviderScope(
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SingleChildScrollView(
              child: SettingsAnalysisPanel(draft: draft, onChanged: onChanged),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  AppSettings thresholdDraft(double db) =>
      copyAppSettings(defaultAppSettings(), cueThresholdDb: db);

  AppButton stepButton(WidgetTester tester, String glyph) =>
      tester.widget<AppButton>(find.widgetWithText(AppButton, glyph));

  /// Scrolls an off-screen stepper button into view before tapping it.
  Future<void> tapStep(WidgetTester tester, String glyph) async {
    final finder = find.widgetWithText(AppButton, glyph);
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pump();
  }

  testWidgets('analysis quality and cue detection are both cards', (
    tester,
  ) async {
    final l10n = lookupAppLocalizations(const Locale('en'));
    await pumpAnalysisPanel(
      tester,
      draft: defaultAppSettings(),
      onChanged: (_) {},
    );

    // The section header and the quality card both read "Analysis", so the
    // card is checked through its unique field label instead.
    expect(
      find.text(l10n.settingsLibraryAnalysisQuality.toUpperCase()),
      findsOneWidget,
    );
    expect(find.text(l10n.settingsAnalysisCueTitle), findsOneWidget);
    expect(
      find.text(l10n.settingsAnalysisCueThreshold.toUpperCase()),
      findsOneWidget,
      reason: '`SettingsField` renders labels uppercased',
    );
  });

  testWidgets('the threshold renders in dB and steps up by one', (
    tester,
  ) async {
    AppSettings? emitted;
    await pumpAnalysisPanel(
      tester,
      draft: thresholdDraft(kDefaultCueThresholdDb),
      onChanged: (settings) => emitted = settings,
    );

    expect(find.text('-60 dB'), findsOneWidget);

    await tapStep(tester, '+');

    expect(emitted, isNotNull);
    expect(emitted!.cueThresholdDb, closeTo(-59.0, 1e-6));
  });

  testWidgets('the threshold stepper stops at the bounds', (tester) async {
    await pumpAnalysisPanel(
      tester,
      draft: thresholdDraft(kMaxCueThresholdDb),
      onChanged: (_) {},
    );
    expect(
      stepButton(tester, '+').onPress,
      isNull,
      reason: 'a threshold above the maximum would be rejected by the host',
    );
    expect(stepButton(tester, '−').onPress, isNotNull);

    await pumpAnalysisPanel(
      tester,
      draft: thresholdDraft(kMinCueThresholdDb),
      onChanged: (_) {},
    );
    expect(stepButton(tester, '−').onPress, isNull);
    expect(stepButton(tester, '+').onPress, isNotNull);
  });
}
