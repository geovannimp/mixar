import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show NotifierProviderFamily;
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/owned_resource.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_window.dart';

/// Peaks for one source-time window at the finest density the lane can show.
///
/// Zoom swaps the window instead of scaling a whole-track picture: the strip
/// only ever holds detail for the time on screen, so bars stay crisp at any
/// zoom. There is no coarse whole-track fallback — a lane paints nothing until
/// its first window lands, and keeps the previous window (at its own density)
/// while the next one is fetched, so zooming never swaps resolutions.
class WaveformStrip implements OwnedResource {
  new({
    required this.originMs,
    required this.spanMs,
    required this.widthPx,
    required this.heightPx,
    required this.texture,
  });

  /// Track time at the strip's left edge.
  final int originMs;

  /// Track time covered by the strip.
  final int spanMs;

  final int widthPx;
  final int heightPx;
  final WaveformPeaksTexture texture;

  var _disposed = false;

  @override
  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    texture.dispose();
  }
}

/// (track, track duration, deck)
typedef WaveformStripKey = (String, int, int);

class WaveformStripNotifier extends Notifier<WaveformStrip?>
    with OwnedResourceNotifier<WaveformStrip> {
  new(this.arg);

  final WaveformStripKey arg;

  /// Window and analysis epoch already requested, in flight or published, so a
  /// rebuild for anything else does not queue the same fetch twice.
  (int, int, int, int)? _recordedFor;

  /// Whether the fetch for [_recordedFor] is still running. A released or
  /// failed strip leaves nothing painted, and the next build must fetch again
  /// even though the window key did not change.
  var _requesting = false;

  /// Set by the release registered with `ref.onDispose`; a rebuild clears it so
  /// the strip on screen is not dropped just because a dependency changed.
  var _releasePending = false;

  @override
  WaveformStrip? build() {
    // Riverpod runs `onDispose` listeners before every rebuild, so a rebuild
    // would otherwise free the strip it is about to keep painting.
    _releasePending = false;
    ref.onDispose(_releaseUnlessRebuilt);
    final (trackId, durationMs, deckId) = arg;
    final window = ref.watch(waveformWindowProvider(deckId));
    final previous = owned;
    if (durationMs <= 0 || window.spanMs <= 0) {
      // Bump before dropping: a fetch still in flight for the old window must
      // not publish over the empty state.
      supersede();
      _recordedFor = null;
      _requesting = false;
      releaseIfOwned(previous);
      return null;
    }
    final key = (
      window.originMs,
      window.spanMs,
      kMaxPeakTextureWidth,
      ref.watch(libraryAnalysisEpochProvider),
    );
    // `_requesting` keeps the first paint from firing a second fetch, while the
    // `previous == null` arm recovers a strip that a release or a failed fetch
    // left unpainted.
    if (_recordedFor != key || (previous == null && !_requesting)) {
      _recordedFor = key;
      _requesting = true;
      final gen = supersede();
      unawaited(
        _record(
          trackId,
          durationMs,
          window.originMs,
          window.spanMs,
          gen,
        ).whenComplete(() {
          if (isCurrent(gen)) {
            _requesting = false;
          }
        }),
      );
    }
    // The previous window stays on screen until the new one is encoded: a zoom
    // or a slide must not blank the lane while peaks are fetched.
    return previous;
  }

  /// Releases the strip only when the provider is really gone: the callback
  /// also fires on rebuild, and the rebuild that follows clears the flag.
  void _releaseUnlessRebuilt() {
    _releasePending = true;
    // Riverpod also runs dispose listeners when a provider is *invalidated*,
    // and the rebuild that follows can happen later in the same frame. A
    // post-frame check therefore sees the rebuild (which clears the flag)
    // before dropping the strip; a real dispose has no rebuild to cancel it.
    SchedulerBinding.instance.addPostFrameCallback((_) {
      if (!_releasePending) {
        return;
      }
      _releasePending = false;
      releaseOwned();
    });
  }

  Future<void> _record(
    String trackId,
    int durationMs,
    int originMs,
    int spanMs,
    int gen,
  ) async {
    final endMs = (originMs + spanMs).clamp(0, durationMs);
    if (endMs <= originMs) {
      return;
    }
    final WaveformPeaksTexture texture;
    try {
      final lib = await ref.read(libraryTransportProvider.future);
      if (!ref.mounted || !isCurrent(gen)) {
        return;
      }
      final packed = await lib.getWaveformWindow(
        trackId: trackId,
        startMs: originMs,
        endMs: endMs,
        buckets: kMaxPeakTextureWidth,
      );
      if (!ref.mounted || !isCurrent(gen)) {
        return;
      }
      final peaks = decodeRgbPeaks(packed.rgb);
      if (peaks.isEmpty) {
        return;
      }
      texture = await encodeWaveformTexture(peaks);
    } on Object catch (error) {
      // A failed fetch leaves the previous window in place; the lane's hot path
      // must not toast.
      debugPrint('waveform window fetch failed: $error');
      return;
    }
    if (!ref.mounted || !isCurrent(gen)) {
      texture.dispose();
      return;
    }
    publish(
      WaveformStrip(
        originMs: originMs,
        // The track may be shorter than the requested window; publishing the
        // requested span would stretch the peaks over time the track lacks.
        spanMs: endMs - originMs,
        widthPx: kMaxPeakTextureWidth,
        heightPx: kWaveformStripHeight.round(),
        texture: texture,
      ),
    );
  }
}

// autoDispose so unloading a deck releases the strip. The window (origin/span)
// is a dependency, not part of the key: zooming and sliding rebuild this
// provider and keep the strip already painted until the new peaks land.
final NotifierProviderFamily<
  WaveformStripNotifier,
  WaveformStrip?,
  WaveformStripKey
>
waveformStripProvider = NotifierProvider.autoDispose
    .family<WaveformStripNotifier, WaveformStrip?, WaveformStripKey>(
      WaveformStripNotifier.new,
    );
