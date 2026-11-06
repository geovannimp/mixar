import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/utils/controller_audio.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/screens/settings_page.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_audio_panel.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:material_ui/material_ui.dart';

import 'support/mixar_material_app.dart';

AppSettings _overridden() {
  final base = defaultAppSettings();
  const ddj = BusRouteSettings(
    deviceId: 'cpal:ddj',
    leftChannel: 1,
    rightChannel: 2,
    mode: BusChannelMode.stereo,
    lowLatency: false,
  );
  return copyAppSettings(
    base,
    masterBus: ddj,
    previewEnabled: true,
    previewBus: const BusRouteSettings(
      deviceId: 'cpal:ddj',
      leftChannel: 3,
      rightChannel: 4,
      mode: BusChannelMode.stereo,
      lowLatency: false,
    ),
    controllerAudioOverride: ControllerAudioOverride(
      mappingId: 'ddj-400',
      controllerLabel: 'DDJ-400',
      outputDeviceId: 'cpal:ddj',
      previousMasterBus: base.masterBus,
      masterOverridden: true,
      previousPreviewEnabled: false,
      previousPreviewBus: base.previewBus,
      previewOverridden: true,
    ),
  );
}

void main() {
  testWidgets('marks buses overridden by a controller mapping', (tester) async {
    var saved = _overridden();
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

    expect(find.text('Overridden by DDJ-400 mapping'), findsNWidgets(2));

    // Detaching the controller restores the previous routes and clears it.
    saved = restoreControllerAudioOutputs(saved, 'ddj-400')!.settings;
    container.invalidate(appSettingsProvider);
    await tester.pumpAndSettle();

    expect(find.text('Overridden by DDJ-400 mapping'), findsNothing);
    final panel = tester.widget<SettingsAudioPanel>(
      find.byType(SettingsAudioPanel),
    );
    expect(panel.draft.masterBus.deviceId, 'default');
  });
}
