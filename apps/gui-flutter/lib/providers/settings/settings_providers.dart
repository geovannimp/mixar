import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart' show debugPrint;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show ProviderListenable;
import 'package:gui_flutter/analytics/analytics_transport.dart';
import 'package:gui_flutter/core/models/settings_section.dart';
import 'package:gui_flutter/core/utils/desktop.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsDeepLink {
  const SettingsDeepLink({
    this.section = SettingsSection.ui,
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
  // Sync bridge call behind a Future: keeps a failed init an async provider error.
  return Future.sync(SettingsTransport.open);
});

final audioDevicesProvider = FutureProvider<List<OutputDevice>>(
  (ref) => listOutputDevices(),
);

final appSettingsProvider = FutureProvider<AppSettings>((ref) async {
  final settings = await ref.watch(settingsTransportProvider.future);
  return normalizeAppSettings(await settings.getSettings());
});

/// Resolve the saved settings before the first frame.
///
/// `Application` otherwise boots on the defaults — `system` theme, teal
/// palette, system locale — and swaps them once the bridge answers, so a saved
/// light theme shows a dark frame first. Read the provider from the container
/// handed to `runApp` so its state is already `AsyncData` on frame one.
///
/// [timeout] bounds the bridge read: a hung settings host must not keep the
/// window hidden. On expiry (or any read error) the launch keeps the defaults,
/// and the tree swaps if the read lands later — the same fallback behaviour as
/// before the preload existed.
Future<void> preloadAppSettings(
  ProviderContainer container, {
  Duration timeout = const Duration(seconds: 2),
}) async {
  try {
    await container.read(appSettingsProvider.future).timeout(timeout);
  } on Object catch (error, stackTrace) {
    debugPrint('mixar: settings preload failed: $error\n$stackTrace');
  }
}

/// Live StemgenRT opt-in; the pad-mode label only adds its source suffix when
/// this is on.
final realtimeStemsEnabledProvider = Provider<bool>((ref) {
  final settings = ref.watch(appSettingsProvider).asData?.value;
  return settings?.realtimeStems ?? false;
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
        stemsModel: settings.value.stemsModel,
        tagWriteBack: settings.value.tagWriteBack,
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

/// Restart with frozen reload identities; fresh empty snapshots retire live IDs.
Future<void> restartDecksFromSettings(
  T Function<T>(ProviderListenable<T> provider) read, {
  required Future<void> Function() restart,
  required int previousDeckCount,
  required Future<void> Function({required int deckId, required String trackId})
  loadLibraryTrack,
  required Future<void> Function({required int deckId, required String path})
  loadPath,
  required int deckCount,
}) async {
  final snapshot = read(engineUiProvider);
  final engineUi = read(engineUiProvider.notifier);
  final generations = read(deckLoadGenerationProvider.notifier);
  final restartingDeckCount = previousDeckCount > deckCount
      ? previousDeckCount
      : deckCount;
  for (var deckId = 0; deckId < restartingDeckCount; deckId++) {
    generations.invalidate(deckId);
  }
  // Expansion snapshots can arrive before the restart future completes.
  engineUi.setActiveDeckCount(restartingDeckCount);
  try {
    await restart();
  } catch (_) {
    engineUi.setActiveDeckCount(previousDeckCount);
    rethrow;
  }
  engineUi.setActiveDeckCount(deckCount);
  for (var deckId = deckCount; deckId < previousDeckCount; deckId++) {
    engineUi.clearDeckState(deckId);
  }
  for (var deckId = 0; deckId < deckCount; deckId++) {
    final trackId = snapshot.trackIdFor(deckId);
    if (trackId != null) {
      await loadLibraryTrack(deckId: deckId, trackId: trackId);
    } else {
      final path = snapshot.trackPathFor(deckId);
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
  final previous = await ref.read(appSettingsProvider.future);
  final normalized = normalizeAppSettings(draft);
  // A save that only changes how the list paints — the theme, the consent
  // toggle, the visible row fields — must not restart the engine: audio would
  // glitch for a cosmetic edit. Everything else goes through the apply path
  // below.
  final viewOnly = !appSettingsDirty(
    copyAppSettings(
      normalized,
      analyticsEnabled: previous.analyticsEnabled,
      libraryRowFields: previous.libraryRowFields,
      theme: previous.theme,
      accentPalette: previous.accentPalette,
    ),
    previous,
  );
  final trustedChanged = !const ListEquality<String>().equals(
    previous.trustedControllerDeviceIds,
    normalized.trustedControllerDeviceIds,
  );
  final saved = await settings.saveSettings(settings: normalized);
  if (normalized.analyticsEnabled != previous.analyticsEnabled) {
    try {
      final analytics = await ref.read(analyticsTransportProvider.future);
      await analytics.setEnabled(enabled: normalized.analyticsEnabled);
    } on Object {
      // Analytics must never turn a successful settings save into a failure.
    }
  }
  if (viewOnly) {
    ref.invalidate(appSettingsProvider);
    return SaveAppSettingsResult(saved: saved);
  }
  final library = await ref.read(libraryTransportProvider.future);
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
      stemsModel: normalized.stemsModel,
      tagWriteBack: normalized.tagWriteBack,
    );
    await library.applyHistorySettings(
      enabled: normalized.historyEnabled,
      sessionIdleMinutes: normalized.historySessionIdleMinutes,
      minPlaySeconds: normalized.historyMinPlaySeconds,
      minDeckVolume: normalized.historyMinDeckVolume,
    );
    var engine = await ref.read(engineTransportProvider.future);
    if (engine == null && isDesktopWindow) {
      // A failed engine start caches no transport, only a notice. Saving settings is the
      // user's way back: retry once the audio device configuration has been fixed.
      ref.invalidate(engineTransportProvider);
      engine = await ref.read(engineTransportProvider.future);
    }
    if (engine != null && await engine.isRunning()) {
      await restartDecksFromSettings(
        ref.read,
        restart: engine.restartFromSettings,
        previousDeckCount: previous.numDecks,
        deckCount: normalized.numDecks,
        loadLibraryTrack: engine.loadLibraryTrack,
        loadPath: engine.loadPath,
      );
    } else if (normalized.numDecks < previous.numDecks) {
      // No restart to retire the removed decks: drop their stale mirror state.
      final engineUi = ref.read(engineUiProvider.notifier);
      for (
        var deckId = normalized.numDecks;
        deckId < previous.numDecks;
        deckId++
      ) {
        engineUi.clearDeckState(deckId);
      }
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
