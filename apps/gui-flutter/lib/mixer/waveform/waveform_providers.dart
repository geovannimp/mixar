import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

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

class WaveformDisplayModeNotifier extends Notifier<WaveformDisplayMode> {
  @override
  WaveformDisplayMode build() => WaveformDisplayMode.rgb;

  void set(WaveformDisplayMode mode) => state = mode;
}

final waveformDisplayModeProvider =
    NotifierProvider<WaveformDisplayModeNotifier, WaveformDisplayMode>(
      WaveformDisplayModeNotifier.new,
    );
