import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:riverpod/src/providers/future_provider.dart';
import 'package:riverpod/src/providers/provider.dart';

final FutureProviderFamily<List<SpectralPeak>, String>
waveformOverviewProvider = FutureProvider.family<List<SpectralPeak>, String>((
  ref,
  trackId,
) async {
  ref.watch(libraryAnalysisEpochProvider);
  final lib = await ref.watch(libraryTransportProvider.future);
  final packed = await lib.getWaveformOverview(trackId: trackId);
  if (packed == null) {
    return const [];
  }
  return decodeRgbPeaks(packed.rgb);
});

final FutureProviderFamily<BeatGridData?, String> beatGridFetchProvider =
    FutureProvider.family<BeatGridData?, String>((ref, trackId) async {
      ref.watch(libraryAnalysisEpochProvider);
      final lib = await ref.watch(libraryTransportProvider.future);
      return lib.getBeatGrid(trackId: trackId);
    });

/// Beat grid for a track: event cache first, otherwise the initial library fetch.
final ProviderFamily<BeatGridData?, String> beatGridProvider =
    Provider.family<BeatGridData?, String>((ref, trackId) {
      final entry = ref.watch(
        trackBeatGridsProvider.select(
          (cache) => (cache.containsKey(trackId), cache[trackId]),
        ),
      );
      if (entry.$1) {
        return entry.$2;
      }
      return ref.watch(beatGridFetchProvider(trackId)).value;
    });

/// True only while the first fetch is in flight (not after grid edits).
final ProviderFamily<bool, String> beatGridLoadingProvider =
    Provider.family<bool, String>((ref, trackId) {
      if (ref.watch(
        trackBeatGridsProvider.select((cache) => cache.containsKey(trackId)),
      )) {
        return false;
      }
      return ref.watch(beatGridFetchProvider(trackId)).isLoading;
    });

final waveformDisplayModeProvider = Provider<WaveformDisplayMode>((ref) {
  return waveformModeFromSettings(
    ref
        .watch(appSettingsProvider)
        .maybeWhen(
          data: (s) => s.waveformDisplayMode,
          orElse: () => WaveformDisplayModeSetting.rgb,
        ),
  );
});
