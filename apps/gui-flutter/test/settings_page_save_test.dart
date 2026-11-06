import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_audio_panel.dart';
import 'package:gui_flutter/widgets/settings/settings_mixer_panel.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

void main() {
  testWidgets('canceling restart while closing preserves the dirty draft', (
    tester,
  ) async {
    final baseline = copyAppSettings(defaultAppSettings(), numDecks: 2);
    final l10n = lookupAppLocalizations(const Locale('en'));
    var closeCount = 0;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [appSettingsProvider.overrideWith((ref) async => baseline)],
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: Scaffold(
            body: SettingsPage(
              initialSection: SettingsSection.mixer,
              onClose: () => closeCount++,
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    tester
        .widget<SettingsMixerPanel>(find.byType(SettingsMixerPanel))
        .onChanged(copyAppSettings(baseline, numDecks: 4));
    await tester.pump();
    await tester.tap(find.bySemanticsLabel(l10n.settingsCloseSemantics));
    await tester.pumpAndSettle();
    expect(find.text(l10n.settingsUnsavedTitle), findsOneWidget);

    await tester.tap(find.widgetWithText(AppButton, l10n.settingsSave).last);
    await tester.pumpAndSettle();
    expect(find.text(l10n.settingsLayoutDeckCountRestartTitle), findsOneWidget);
    await tester.tap(find.widgetWithText(AppButton, l10n.settingsCancel));
    await tester.pumpAndSettle();

    expect(closeCount, 0);
    expect(
      tester
          .widget<SettingsMixerPanel>(find.byType(SettingsMixerPanel))
          .draft
          .numDecks,
      4,
    );
    expect(find.widgetWithText(AppButton, l10n.settingsSave), findsOneWidget);
    expect(
      tester
          .widget<AppButton>(find.widgetWithText(AppButton, l10n.settingsSave))
          .onPress,
      isNotNull,
    );

    await tester.tap(find.bySemanticsLabel(l10n.settingsCloseSemantics));
    await tester.pumpAndSettle();
    expect(find.text(l10n.settingsUnsavedTitle), findsOneWidget);
    await tester.tap(find.widgetWithText(AppButton, l10n.settingsCancel));
    await tester.pumpAndSettle();
    expect(closeCount, 0);
  });

  testWidgets('adopts audio outputs saved outside the page', (tester) async {
    var saved = defaultAppSettings();
    final container = ProviderContainer(
      overrides: [
        appSettingsProvider.overrideWith((ref) async => saved),
        audioDevicesProvider.overrideWith(
          (ref) async => const <OutputDevice>[],
        ),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(
            body: SettingsPage(initialSection: SettingsSection.audio),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<SettingsAudioPanel>(find.byType(SettingsAudioPanel))
          .draft
          .masterBus
          .deviceId,
      'default',
    );

    // What the controller audio prompt saves while Settings is open.
    saved = copyAppSettings(
      saved,
      masterBus: const BusRouteSettings(
        deviceId: 'cpal:ddj',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        lowLatency: false,
      ),
      previewEnabled: true,
      previewBus: const BusRouteSettings(
        deviceId: 'cpal:ddj',
        leftChannel: 3,
        rightChannel: 4,
        mode: BusChannelMode.stereo,
        lowLatency: false,
      ),
    );
    container.invalidate(appSettingsProvider);
    await tester.pumpAndSettle();

    final panel = tester.widget<SettingsAudioPanel>(
      find.byType(SettingsAudioPanel),
    );
    expect(panel.draft.masterBus.deviceId, 'cpal:ddj');
    expect(panel.draft.previewEnabled, isTrue);
    expect(panel.draft.previewBus.leftChannel, 3);
  });

  testWidgets('keeps dirty edits while adopting an external audio save', (
    tester,
  ) async {
    var saved = defaultAppSettings();
    final container = ProviderContainer(
      overrides: [
        appSettingsProvider.overrideWith((ref) async => saved),
        audioDevicesProvider.overrideWith(
          (ref) async => const <OutputDevice>[],
        ),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: mixarTestMaterialApp(
          theme: MixarThemeData.dark(),
          home: const Scaffold(
            body: SettingsPage(initialSection: SettingsSection.audio),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    tester
        .widget<SettingsAudioPanel>(find.byType(SettingsAudioPanel))
        .onChanged(copyAppSettings(defaultAppSettings(), bufferSize: 1024));
    await tester.pump();

    saved = copyAppSettings(
      saved,
      masterBus: const BusRouteSettings(
        deviceId: 'cpal:ddj',
        leftChannel: 1,
        rightChannel: 2,
        mode: BusChannelMode.stereo,
        lowLatency: false,
      ),
    );
    container.invalidate(appSettingsProvider);
    await tester.pumpAndSettle();

    final panel = tester.widget<SettingsAudioPanel>(
      find.byType(SettingsAudioPanel),
    );
    expect(panel.draft.bufferSize, 1024, reason: 'local edit is preserved');
    expect(panel.draft.masterBus.deviceId, 'cpal:ddj');
  });
}
