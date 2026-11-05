import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/onboarding/onboarding_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

import 'support/fake_settings_transport.dart';

/// A container whose settings host starts at [stored], disposed with the test.
ProviderContainer _containerWithSettings(AppSettings stored) {
  final container = ProviderContainer(
    overrides: [
      settingsTransportProvider.overrideWith(
        (_) async => FakeSettingsTransport(stored),
      ),
    ],
  );
  addTearDown(container.dispose);
  return container;
}

void main() {
  /// A container whose saved settings already show setup done, plus them.
  Future<(ProviderContainer, AppSettings)> completedSetup() async {
    final container = _containerWithSettings(
      copyAppSettings(defaultAppSettings(), onboardingCompleted: true),
    );
    return (container, await container.read(appSettingsProvider.future));
  }

  /// The draft only exists to preview step choices before Finish. Once a save
  /// with setup done lands, `appSettingsProvider` owns theme, accent and
  /// locale; a draft installed after that point is never cleared again, and
  /// would keep overriding every later Settings edit for the session.
  test('a saved setup never becomes a preview draft', () async {
    final (container, saved) = await completedSetup();

    // Both orderings Finish can produce: the saved object itself, and a step
    // draft that arrives after the save landed.
    container.read(onboardingDraftProvider.notifier).set(saved);
    expect(container.read(onboardingDraftProvider), isNull);

    container
        .read(onboardingDraftProvider.notifier)
        .set(
          copyAppSettings(saved, accentPalette: AccentPaletteSetting.violet),
        );
    expect(container.read(onboardingDraftProvider), isNull);
  });

  test('a draft during setup is previewed', () async {
    final container = _containerWithSettings(defaultAppSettings());

    final draft = copyAppSettings(
      await container.read(appSettingsProvider.future),
      accentPalette: AccentPaletteSetting.violet,
    );
    container.read(onboardingDraftProvider.notifier).set(draft);

    // The whole object: any field the step copy touches has to survive.
    expect(container.read(onboardingDraftProvider), draft);
  });

  test('a draft disappears when cleared', () async {
    final container = _containerWithSettings(defaultAppSettings());
    container
        .read(onboardingDraftProvider.notifier)
        .set(await container.read(appSettingsProvider.future));
    expect(container.read(onboardingDraftProvider), isNotNull);

    container.read(onboardingDraftProvider.notifier).set(null);

    expect(container.read(onboardingDraftProvider), isNull);
  });
}
