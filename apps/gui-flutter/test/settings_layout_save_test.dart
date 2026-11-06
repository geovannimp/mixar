import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

import 'support/fake_library_transport.dart';
import 'support/fake_settings_transport.dart';

/// Widget-backed save harness: a [WidgetRef] plus a fake settings host and an
/// engine provider that counts every read, so a view-only save can prove it
/// never touched the engine.
class _Harness {
  new(this.baseline) : settings = FakeSettingsTransport(baseline);

  final AppSettings baseline;
  final FakeSettingsTransport settings;
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

/// A [WidgetRef] from a scope that exists only for the ref; unmounting it is
/// what makes later reads throw, as they do on a disposed page.
Future<WidgetRef> pumpCapturedRef(WidgetTester tester) async {
  late WidgetRef ref;
  await tester.pumpWidget(
    ProviderScope(
      child: Consumer(
        builder: (context, widgetRef, _) {
          ref = widgetRef;
          return const SizedBox.shrink();
        },
      ),
    ),
  );
  return ref;
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

  testWidgets('finishing setup does not restart the engine', (tester) async {
    final harness = _Harness(defaultAppSettings());
    await harness.pump(tester);

    final result = await saveAppSettings(
      harness.ref,
      copyAppSettings(harness.baseline, onboardingCompleted: true),
    );

    expect(result.saved.onboardingCompleted, isTrue);
    expect(harness.settings.stored.onboardingCompleted, isTrue);
    expect(
      harness.engineReads,
      0,
      reason: 'setup completion is not an audio change',
    );
  });

  testWidgets('a save outlives the widget that started it', (tester) async {
    // The setup page's scope dies the moment its save swaps `home` for the
    // mixer, so the apply path has to run on a container captured earlier.
    final ref = await pumpCapturedRef(tester);
    // Unmount: any read from `ref` now throws, exactly like the dead page.
    await tester.pumpWidget(const SizedBox.shrink());

    final settings = FakeSettingsTransport(defaultAppSettings());
    final library = FakeLibraryTransport();
    final container = ProviderContainer(
      overrides: [
        settingsTransportProvider.overrideWith((_) async => settings),
        libraryTransportProvider.overrideWith((_) async => library),
        engineTransportProvider.overrideWith((_) async => null),
      ],
    );
    addTearDown(container.dispose);

    // Not view-only: history settings go through the apply path.
    final result = await saveAppSettings(
      ref,
      copyAppSettings(
        defaultAppSettings(),
        onboardingCompleted: true,
        historyEnabled: false,
      ),
      container: container,
    );

    expect(settings.stored.onboardingCompleted, isTrue);
    expect(result.applyError, isNull, reason: 'the apply path must run');
  });
}
