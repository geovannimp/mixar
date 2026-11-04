import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class _FakeSettingsTransport implements SettingsTransport {
  new(this.stored);

  AppSettings stored;

  @override
  void dispose() {}

  @override
  bool get isDisposed => false;

  @override
  Future<AppSettings> getSettings() async => stored;

  @override
  Future<AppSettings> saveSettings({required AppSettings settings}) async {
    stored = settings;
    return settings;
  }
}

void main() {
  testWidgets('saving a waveform position does not restart the engine', (
    tester,
  ) async {
    final baseline = defaultAppSettings();
    final settings = _FakeSettingsTransport(baseline);
    var engineReads = 0;
    late WidgetRef ref;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          settingsTransportProvider.overrideWith((_) async => settings),
          appSettingsProvider.overrideWith((_) async => baseline),
          engineTransportProvider.overrideWith((_) async {
            engineReads++;
            return null;
          }),
        ],
        child: Consumer(
          builder: (context, widgetRef, _) {
            ref = widgetRef;
            return const SizedBox.shrink();
          },
        ),
      ),
    );
    await tester.pump();

    final result = await saveAppSettings(
      ref,
      copyAppSettings(
        baseline,
        waveformPosition: WaveformPositionSetting.middleHorizontal,
      ),
    );

    expect(
      result.saved.waveformPosition,
      WaveformPositionSetting.middleHorizontal,
    );
    expect(
      settings.stored.waveformPosition,
      WaveformPositionSetting.middleHorizontal,
    );
    expect(engineReads, 0, reason: 'a layout move must not restart audio');
  });
}
