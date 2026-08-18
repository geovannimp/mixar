import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

final waveformOverviewProvider =
    FutureProvider.family<List<SpectralPeak>, String>((ref, trackId) async {
      ref.watch(libraryAnalysisEpochProvider);
      final lib = await ref.watch(libraryTransportProvider.future);
      final packed = await lib.getWaveformOverview(trackId: trackId);
      if (packed == null) {
        return const [];
      }
      return decodeRgbPeaks(packed.rgb);
    });

final beatGridProvider = FutureProvider.family<BeatGridData?, String>((
  ref,
  trackId,
) async {
  ref.watch(libraryAnalysisEpochProvider);
  final lib = await ref.watch(libraryTransportProvider.future);
  return lib.getBeatGrid(trackId: trackId);
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
