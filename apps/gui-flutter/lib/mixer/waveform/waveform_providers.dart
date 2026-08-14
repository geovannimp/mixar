import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

final waveformOverviewProvider =
    FutureProvider.family<List<SpectralPeak>, String>((ref, trackId) async {
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
  final lib = await ref.watch(libraryTransportProvider.future);
  return lib.getBeatGrid(trackId: trackId);
});
