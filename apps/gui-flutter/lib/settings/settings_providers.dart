import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';

final settingsTransportProvider = FutureProvider<SettingsTransport>((
  ref,
) async {
  final support = await getApplicationSupportDirectory();
  return SettingsTransport.open(path: p.join(support.path, 'settings.json'));
});

final audioBackendNamesProvider = Provider<List<String>>(
  (_) => AudioBackendTransport.listNames(),
);

final audioDevicesProvider = FutureProvider.family<List<OutputDevice>, String>((
  ref,
  backend,
) async {
  final transport = await AudioBackendTransport.open(name: backend);
  return transport.listOutputDevices();
});

final appSettingsProvider = FutureProvider<AppSettings>((ref) async {
  final settings = await ref.watch(settingsTransportProvider.future);
  return normalizeAppSettings(await settings.getSettings());
});

final libraryTableColumnsProvider = Provider<List<String>>((ref) {
  return ref
      .watch(appSettingsProvider)
      .maybeWhen(
        data: (s) => s.libraryTableColumns,
        orElse: () => List<String>.from(kDefaultLibraryColumns),
      );
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

class SaveAppSettingsResult {
  const SaveAppSettingsResult({
    required this.saved,
    this.applyError,
    this.trustedControllersChanged = false,
  });

  final AppSettings saved;
  final String? applyError;
  final bool trustedControllersChanged;
}

Future<SaveAppSettingsResult> saveAppSettings(
  WidgetRef ref,
  AppSettings draft,
) async {
  final settings = await ref.read(settingsTransportProvider.future);
  final library = await ref.read(libraryTransportProvider.future);
  final previous = await ref.read(appSettingsProvider.future);
  final normalized = normalizeAppSettings(draft);
  final trustedChanged = !_sameTrusted(
    previous.trustedControllerDeviceIds,
    normalized.trustedControllerDeviceIds,
  );
  final saved = await settings.saveSettings(settings: normalized);
  ref.invalidate(appSettingsProvider);
  ref.invalidate(libraryTableColumnsProvider);
  String? applyError;
  try {
    await library.applyLibrarySettings(
      analysisDuration: _libraryAnalysisDuration(normalized.analysisDuration),
    );
    final engine = await ref.read(engineTransportProvider.future);
    if (engine != null && await engine.isRunning()) {
      await engine.restartFromSettings();
      for (var deckId = 0; deckId < 2; deckId++) {
        final trackId = ref.read(deckTrackIdProvider(deckId));
        if (trackId != null) {
          await engine.loadLibraryTrack(deckId: deckId, trackId: trackId);
        }
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

bool _sameTrusted(List<String> a, List<String> b) {
  if (a.length != b.length) {
    return false;
  }
  final left = [...a]..sort();
  final right = [...b]..sort();
  for (var i = 0; i < left.length; i++) {
    if (left[i] != right[i]) {
      return false;
    }
  }
  return true;
}
