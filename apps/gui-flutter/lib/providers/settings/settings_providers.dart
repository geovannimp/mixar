import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart'
    show FutureProviderFamily, ProviderListenable;
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

class SettingsDeepLink {
  const SettingsDeepLink({
    this.section = SettingsSection.audio,
    this.scrollToPreviewBus = false,
  });

  final SettingsSection section;
  final bool scrollToPreviewBus;
}

class PendingSettingsDeepLink extends Notifier<SettingsDeepLink?> {
  @override
  SettingsDeepLink? build() => null;

  void request(SettingsDeepLink link) => state = link;

  void clear() => state = null;
}

final pendingSettingsDeepLinkProvider =
    NotifierProvider<PendingSettingsDeepLink, SettingsDeepLink?>(
      PendingSettingsDeepLink.new,
    );

final settingsTransportProvider = FutureProvider<SettingsTransport>((
  ref,
) async {
  final support = await getApplicationSupportDirectory();
  return SettingsTransport.open(path: p.join(support.path, 'settings.json'));
});

final audioBackendNamesProvider = Provider<List<String>>(
  (_) => AudioBackendTransport.listNames(),
);

final FutureProviderFamily<List<OutputDevice>, String> audioDevicesProvider =
    FutureProvider.family<List<OutputDevice>, String>((ref, backend) async {
      final transport = await AudioBackendTransport.open(name: backend);
      return transport.listOutputDevices();
    });

final appSettingsProvider = FutureProvider<AppSettings>((ref) async {
  final settings = await ref.watch(settingsTransportProvider.future);
  return normalizeAppSettings(await settings.getSettings());
});

/// Saved library row layout, the default the list renders with.
final libraryRowDensitySettingProvider = Provider<LibraryRowDensity>((ref) {
  return ref
      .watch(appSettingsProvider)
      .maybeWhen(
        data: (s) => libraryRowDensityFromSettings(s.libraryRowDensity),
        orElse: () => LibraryRowDensity.comfortable,
      );
});

/// Saved select / dropdown presentation style (default auto).
final selectStyleSettingProvider = Provider<SelectStyleSetting>((ref) {
  return ref
      .watch(appSettingsProvider)
      .maybeWhen(
        data: (s) => s.selectStyle,
        orElse: () => SelectStyleSetting.auto,
      );
});

/// Session-local density toggle for the library toolbar. It overrides the
/// saved default for the current session only, so the button never writes
/// settings.json or takes the engine-restart path in [saveAppSettings].
class LibraryRowDensityOverride extends Notifier<LibraryRowDensity?> {
  @override
  LibraryRowDensity? build() => null;

  void toggle(LibraryRowDensity current) =>
      state = state == null ? current.other : null;

  /// Null clears the override and falls back to the saved setting.
  void set(LibraryRowDensity? density) => state = density;
}

final libraryRowDensityOverrideProvider =
    NotifierProvider<LibraryRowDensityOverride, LibraryRowDensity?>(
      LibraryRowDensityOverride.new,
    );

/// Effective row density: the session override if set, else the saved default.
final libraryRowDensityProvider = Provider<LibraryRowDensity>((ref) {
  final override = ref.watch(libraryRowDensityOverrideProvider);
  return override ?? ref.watch(libraryRowDensitySettingProvider);
});

final samplerBanksProvider = FutureProvider<List<SamplerBankInfo>>((ref) async {
  final library = await ref.watch(libraryTransportProvider.future);
  return library.listSamplerBanks();
});

LibraryAnalysisDurationSetting _libraryAnalysisDuration(
  AnalysisDurationSetting duration,
) {
  return switch (duration) {
    AnalysisDurationSetting.fast => LibraryAnalysisDurationSetting.fast,
    AnalysisDurationSetting.precise => LibraryAnalysisDurationSetting.precise,
    AnalysisDurationSetting.complete => LibraryAnalysisDurationSetting.complete,
  };
}

/// Push analysis duration + stem format once settings and library are ready.
final librarySettingsBootstrapProvider = Provider<void>((ref) {
  final settings = ref.watch(appSettingsProvider);
  final library = ref.watch(libraryTransportProvider);
  if (settings is AsyncData<AppSettings> &&
      library is AsyncData<LibraryTransport>) {
    unawaited(
      library.value.applyLibrarySettings(
        analysisDuration: _libraryAnalysisDuration(
          settings.value.analysisDuration,
        ),
        stemsFormat: settings.value.stemsFormat,
      ),
    );
  }
});

class SaveAppSettingsResult {
  const new({
    required this.saved,
    this.applyError,
    this.trustedControllersChanged = false,
  });

  final AppSettings saved;
  final String? applyError;
  final bool trustedControllersChanged;
}

/// Reload decks after [EngineTransport.restartFromSettings].
///
/// Library tracks prefer [deckTrackIdProvider]; path-only loads fall back to
/// `engineUiProvider.trackPathFor`.
Future<void> reloadDecksAfterSettingsRestart(
  T Function<T>(ProviderListenable<T> provider) read, {
  required Future<void> Function({required int deckId, required String trackId})
  loadLibraryTrack,
  required Future<void> Function({required int deckId, required String path})
  loadPath,
}) async {
  for (var deckId = 0; deckId < 2; deckId++) {
    final trackId = read(deckTrackIdProvider(deckId));
    if (trackId != null) {
      await loadLibraryTrack(deckId: deckId, trackId: trackId);
    } else {
      final path = read(engineUiProvider).trackPathFor(deckId);
      if (path != null && path.isNotEmpty) {
        await loadPath(deckId: deckId, path: path);
      }
    }
  }
}

Future<SaveAppSettingsResult> saveAppSettings(
  WidgetRef ref,
  AppSettings draft,
) async {
  final settings = await ref.read(settingsTransportProvider.future);
  final library = await ref.read(libraryTransportProvider.future);
  final previous = await ref.read(appSettingsProvider.future);
  final normalized = normalizeAppSettings(draft);
  final trustedChanged = !const ListEquality<String>().equals(
    previous.trustedControllerDeviceIds,
    normalized.trustedControllerDeviceIds,
  );
  final saved = await settings.saveSettings(settings: normalized);
  ref.invalidate(appSettingsProvider);
  ref.invalidate(libraryRowDensitySettingProvider);
  // Drop any session-only density toggle too. It outranks the saved setting in
  // `libraryRowDensityProvider`, so leaving it set would make the list ignore
  // the value the user just saved.
  ref.invalidate(libraryRowDensityOverrideProvider);
  String? applyError;
  try {
    await library.applyLibrarySettings(
      analysisDuration: _libraryAnalysisDuration(normalized.analysisDuration),
      stemsFormat: normalized.stemsFormat,
    );
    await library.applyHistorySettings(
      enabled: normalized.historyEnabled,
      sessionIdleMinutes: normalized.historySessionIdleMinutes,
      minPlaySeconds: normalized.historyMinPlaySeconds,
      minDeckVolume: normalized.historyMinDeckVolume,
    );
    final engine = await ref.read(engineTransportProvider.future);
    if (engine != null && await engine.isRunning()) {
      await engine.restartFromSettings();
      await reloadDecksAfterSettingsRestart(
        ref.read,
        loadLibraryTrack: engine.loadLibraryTrack,
        loadPath: engine.loadPath,
      );
    }
  } catch (e) {
    applyError = '$e';
  }
  return SaveAppSettingsResult(
    saved: saved,
    applyError: applyError,
    trustedControllersChanged: trustedChanged,
  );
}
