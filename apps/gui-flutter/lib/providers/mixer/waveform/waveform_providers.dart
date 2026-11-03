import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart'
    show FutureProviderFamily, NotifierProviderFamily, ProviderFamily;
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

final waveformOverviewProvider = FutureProvider.autoDispose
    .family<List<SpectralPeak>, String>((ref, trackId) async {
      ref.watch(libraryAnalysisEpochProvider);
      final lib = await ref.watch(libraryTransportProvider.future);
      final packed = await lib.getWaveformOverview(trackId: trackId);
      if (packed == null) {
        return const [];
      }
      return decodeRgbPeaks(packed.rgb);
    });

/// Overview buckets as a shader texture, one per track.
///
/// A rebuild keeps the texture already on screen until the replacement is
/// encoded, the same deal `WaveformStripNotifier` makes: an analysis refresh
/// must not blank the strip while the new graph is fetched and encoded.
final NotifierProviderFamily<
  WaveformOverviewTextureNotifier,
  WaveformPeaksTexture?,
  String
>
waveformOverviewTextureProvider = NotifierProvider.autoDispose
    .family<WaveformOverviewTextureNotifier, WaveformPeaksTexture?, String>(
      WaveformOverviewTextureNotifier.new,
    );

class WaveformOverviewTextureNotifier extends Notifier<WaveformPeaksTexture?> {
  new(this.trackId);

  final String trackId;
  var _gen = 0;
  WaveformPeaksTexture? _owned;

  @override
  WaveformPeaksTexture? build() {
    final peaks = ref.watch(waveformOverviewProvider(trackId)).value;
    ref.onDispose(() {
      _dropAfterFrame(_owned);
      _owned = null;
    });
    // Bump first: an encode still in flight is superseded by this build, even
    // when this build decides to show nothing.
    final gen = ++_gen;
    final previous = _owned;
    if (peaks == null || peaks.isEmpty) {
      _release(previous);
      return null;
    }
    unawaited(_encode(peaks, gen));
    return previous;
  }

  Future<void> _encode(List<SpectralPeak> peaks, int gen) async {
    final texture = await encodeWaveformTexture(peaks);
    // The encode spans several engine hops; a track switch can unmount this
    // ref, and `onDispose` would throw on an unmounted ref instead of
    // releasing the image.
    if (!ref.mounted || gen != _gen) {
      texture.dispose();
      return;
    }
    final replaced = _owned;
    _owned = texture;
    state = texture;
    _dropAfterFrame(replaced);
  }

  /// Drops [texture] once the frame that may still sample it is gone.
  void _release(WaveformPeaksTexture? texture) {
    if (texture == null || !identical(_owned, texture)) {
      return;
    }
    _owned = null;
    _dropAfterFrame(texture);
  }

  void _dropAfterFrame(WaveformPeaksTexture? texture) {
    if (texture == null) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) => texture.dispose());
  }
}

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
