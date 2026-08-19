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
  const SaveAppSettingsResult({required this.saved, this.applyError});

  final AppSettings saved;
  final String? applyError;
}

Future<SaveAppSettingsResult> saveAppSettings(
  WidgetRef ref,
  AppSettings draft,
) async {
  final settings = await ref.read(settingsTransportProvider.future);
  final library = await ref.read(libraryTransportProvider.future);
  final normalized = normalizeAppSettings(draft);
  final saved = await settings.saveSettings(settings: normalized);
  ref.invalidate(appSettingsProvider);
  ref.invalidate(libraryTableColumnsProvider);
  String? applyError;
  try {
    await library.applyLibrarySettings(
      analysisDuration: _libraryAnalysisDuration(normalized.analysisDuration),
      scanFolderTree: normalized.scanFolderTree,
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
  return SaveAppSettingsResult(saved: saved, applyError: applyError);
}
