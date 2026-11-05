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

/// Widget-backed save harness: a [WidgetRef] plus a fake settings host and an
/// engine provider that counts every read, so a view-only save can prove it
/// never touched the engine.
class _Harness {
  _Harness(this.baseline) : settings = _FakeSettingsTransport(baseline);

  final AppSettings baseline;
  final _FakeSettingsTransport settings;
  var engineReads = 0;
  late WidgetRef ref;

  Future<void> pump(WidgetTester tester) async {
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
  }
}

void main() {
  testWidgets('saving a waveform position does not restart the engine', (
    tester,
  ) async {
    final harness = _Harness(defaultAppSettings());
    await harness.pump(tester);

    final result = await saveAppSettings(
      harness.ref,
      copyAppSettings(
        harness.baseline,
        waveformPosition: WaveformPositionSetting.middleHorizontal,
      ),
    );

    expect(
      result.saved.waveformPosition,
      WaveformPositionSetting.middleHorizontal,
    );
    expect(
      harness.settings.stored.waveformPosition,
      WaveformPositionSetting.middleHorizontal,
    );
    expect(
      harness.engineReads,
      0,
      reason: 'a layout move must not restart audio',
    );
  });

  testWidgets('saving panel visibility and the drawer stays view-only', (
    tester,
  ) async {
    final harness = _Harness(defaultAppSettings());
    await harness.pump(tester);

    final result = await saveAppSettings(
      harness.ref,
      copyAppSettings(
        harness.baseline,
        showMixer: false,
        showFxBar: false,
        showCrossfader: false,
        libraryDrawerEnabled: true,
        libraryDrawerSide: LibraryDrawerSideSetting.left,
        mixerMeters: MixerMetersSetting.none,
      ),
    );

    expect(result.saved.showMixer, isFalse);
    expect(result.saved.showFxBar, isFalse);
    expect(result.saved.showCrossfader, isFalse);
    expect(harness.settings.stored.libraryDrawerEnabled, isTrue);
    expect(harness.settings.stored.mixerMeters, MixerMetersSetting.none);
    expect(
      harness.engineReads,
      0,
      reason: 'a cosmetic save must not restart audio',
    );
  });
}
