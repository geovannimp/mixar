import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show NotifierProviderFamily;
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/core/utils/waveform_shader.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/owned_resource.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';

class WaveformTile {
  const new({
    required this.startPx,
    required this.widthPx,
    required this.texture,
  });

  final double startPx;
  final double widthPx;
  final WaveformPeaksTexture texture;
}

class WaveformStrip implements OwnedResource {
  new({
    required this.durationMs,
    required this.widthPx,
    required this.heightPx,
    required this.l0,
    this.tiles = const [],
  });

  final int durationMs;
  final int widthPx;
  final int heightPx;
  final WaveformPeaksTexture l0;
  final List<WaveformTile> tiles;
  var _disposed = false;

  double get pxPerMs => widthPx / durationMs;

  WaveformStrip withTile(WaveformTile tile) => WaveformStrip(
    durationMs: durationMs,
    widthPx: widthPx,
    heightPx: heightPx,
    l0: l0,
    tiles: [...tiles, tile],
  );

  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    l0.dispose();
    for (final tile in tiles) {
      tile.texture.dispose();
    }
  }
}

class WaveformStripNotifier extends Notifier<WaveformStrip?>
    with OwnedResourceNotifier<WaveformStrip> {
  new(this.arg);

  final (String, int) arg;

  @override
  WaveformStrip? build() {
    final (trackId, durationMs) = arg;
    final peaks = ref.watch(waveformOverviewProvider(trackId)).value;
    // Lane fill and display mode are paint-time uniforms: watching them here
    // would re-encode the texture on every theme flip or mode switch.
    ref.onDispose(releaseOwned);
    // Bump first: any recording still in flight is superseded by this build,
    // including the one that decides to render nothing.
    final gen = supersede();
    final previous = owned;
    if (peaks == null || peaks.isEmpty || durationMs <= 0) {
      releaseIfOwned(previous);
      return null;
    }
    unawaited(_recordL0(trackId, peaks, durationMs, gen));
    // Keep the strip that is already on screen until the new one is encoded:
    // blanking the lane for a frame or more on every peaks refresh reads as a
    // glitch, and the peaks behind the old strip are the same track.
    return previous;
  }

  Future<void> _recordL0(
    String trackId,
    List<SpectralPeak> overview,
    int durationMs,
    int gen,
  ) async {
    final width = stripWidthPx(durationMs);
    const height = kWaveformStripHeight;
    final l0 = await encodeWaveformTexture(overview);
    if (!ref.mounted || !isCurrent(gen)) {
      l0.dispose();
      return;
    }
    final strip = WaveformStrip(
      durationMs: durationMs,
      widthPx: width,
      heightPx: height.round(),
      l0: l0,
    );
    publish(strip);
    unawaited(_fillL1(trackId, durationMs, width, gen));
  }

  Future<void> _fillL1(
    String trackId,
    int durationMs,
    int width,
    int gen,
  ) async {
    final lib = await ref.read(libraryTransportProvider.future);
    if (!ref.mounted || !isCurrent(gen)) {
      return;
    }
    const tileW = kWaveformStripTilePx;
    final n = (width / tileW).ceil();
    final focusPx = _focusPx(trackId, width, durationMs);
    final order = [for (var i = 0; i < n; i++) i]
      ..sort((a, b) {
        final ca = (a + 0.5) * tileW;
        final cb = (b + 0.5) * tileW;
        return (ca - focusPx).abs().compareTo((cb - focusPx).abs());
      });
    for (final i in order) {
      if (!ref.mounted || !isCurrent(gen)) {
        return;
      }
      final startPx = i * tileW;
      final remaining = width - startPx;
      if (remaining < 1) {
        continue;
      }
      final w = remaining < tileW ? remaining : tileW;
      final startMs = (startPx / width * durationMs).round();
      final endMs = ((startPx + w) / width * durationMs).round().clamp(
        startMs + 1,
        durationMs,
      );
      try {
        final packed = await lib.getWaveformWindow(
          trackId: trackId,
          startMs: startMs,
          endMs: endMs,
          buckets: w,
        );
        if (!ref.mounted || !isCurrent(gen)) {
          return;
        }
        final peaks = decodeRgbPeaks(packed.rgb);
        if (peaks.isEmpty) {
          continue;
        }
        final texture = await encodeWaveformTexture(peaks);
        final cur = owned;
        if (cur == null || !isCurrent(gen)) {
          texture.dispose();
          return;
        }
        final next = cur.withTile(
          WaveformTile(
            startPx: startPx.toDouble(),
            widthPx: w.toDouble(),
            texture: texture,
          ),
        );
        publish(next);
      } catch (_) {
        continue;
      }
      await Future<void>.delayed(Duration.zero);
    }
  }

  double _focusPx(String trackId, int width, int durationMs) {
    final ids = ref.read(engineUiProvider).trackIds;
    final heads = ref.read(deckPlayheadsProvider);
    for (final e in ids.entries) {
      if (e.value == trackId) {
        return (heads[e.key] ?? 0) / durationMs * width;
      }
    }
    return 0;
  }
}

// autoDispose so unloading a deck releases the strip and its
// waveformOverviewProvider watch (otherwise every loaded track pins peaks).
/// Only the peak texture is cached; lane fill, display mode and gains are
/// uniforms, so neither a theme flip nor an EQ move re-records a strip.
final NotifierProviderFamily<
  WaveformStripNotifier,
  WaveformStrip?,
  (String, int)
>
waveformStripProvider = NotifierProvider.autoDispose
    .family<WaveformStripNotifier, WaveformStrip?, (String, int)>(
      WaveformStripNotifier.new,
    );
