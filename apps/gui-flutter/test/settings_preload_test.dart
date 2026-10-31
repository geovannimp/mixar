import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

void main() {
  test(
    'preload publishes the saved settings before anything reads them',
    () async {
      final settings = copyAppSettings(
        defaultAppSettings(),
        theme: ThemeSetting.light,
      );
      final container = ProviderContainer(
        overrides: [appSettingsProvider.overrideWith((ref) async => settings)],
      );
      addTearDown(container.dispose);

      await preloadAppSettings(container);

      expect(container.read(appSettingsProvider).isLoading, isFalse);
      expect(
        container.read(appSettingsProvider).requireValue.theme,
        ThemeSetting.light,
      );
    },
  );

  test('a bridge read that never answers cannot block the boot', () async {
    // The window is shown after the preload, so an unanswered read must expire
    // into the defaults `Application` already falls back to.
    final never = Completer<AppSettings>();
    final container = ProviderContainer(
      overrides: [appSettingsProvider.overrideWith((ref) => never.future)],
    );
    addTearDown(container.dispose);

    await preloadAppSettings(
      container,
      timeout: const Duration(milliseconds: 20),
    );

    expect(container.read(appSettingsProvider).isLoading, isTrue);
  });
}
