import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show NotifierProviderFamily;
import 'package:gui_flutter/core/models/waveform_peaks.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/core/utils/waveform_picture.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';

class WaveformTile {
  const new({required this.startPx, required this.picture});

  final double startPx;
  final WaveformPicture picture;
}

class WaveformStrip {
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
  final WaveformPicture l0;
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
      tile.picture.dispose();
    }
  }
}

class WaveformStripNotifier extends Notifier<WaveformStrip?> {
  new(this.arg);

  final (String, int, Color) arg;
  var _gen = 0;
  WaveformStrip? _owned;

  @override
  WaveformStrip? build() {
    final (trackId, durationMs, background) = arg;
    final peaks = ref.watch(waveformOverviewProvider(trackId)).value;
    final mode = ref.watch(waveformDisplayModeProvider);
    ref.onDispose(() {
      _dropAfterFrame(_owned);
      _owned = null;
    });
    if (peaks == null || peaks.isEmpty || durationMs <= 0) {
      // Bump so an L0 recording still in flight cannot publish a strip the
      // notifier has just decided not to show.
      _gen++;
      _dropAfterFrame(_owned);
      _owned = null;
      return null;
    }
    final gen = ++_gen;
    final prev = _owned;
    _owned = null;
    _dropAfterFrame(prev);
    unawaited(_recordL0(trackId, peaks, durationMs, background, mode, gen));
    return null;
  }

  Future<void> _recordL0(
    String trackId,
    List<SpectralPeak> overview,
    int durationMs,
    Color background,
    WaveformDisplayMode mode,
    int gen,
  ) async {
    final width = stripWidthPx(durationMs);
    const height = kWaveformStripHeight;
    final l0 = await recordWaveformPicture(
      peaks: overview,
      size: Size(width.toDouble(), height),
      background: background,
      mode: mode,
    );
    if (!ref.mounted || gen != _gen) {
      l0.dispose();
      return;
    }
    final strip = WaveformStrip(
      durationMs: durationMs,
      widthPx: width,
      heightPx: height.round(),
      l0: l0,
    );
    _owned = strip;
    state = strip;
    unawaited(_fillL1(trackId, durationMs, width, gen, mode, background));
  }

  Future<void> _fillL1(
    String trackId,
    int durationMs,
    int width,
    int gen,
    WaveformDisplayMode mode,
    Color background,
  ) async {
    final lib = await ref.read(libraryTransportProvider.future);
    if (!ref.mounted || gen != _gen) {
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
      if (!ref.mounted || gen != _gen) {
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
        if (!ref.mounted || gen != _gen) {
          return;
        }
        final peaks = decodeRgbPeaks(packed.rgb);
        if (peaks.isEmpty) {
          continue;
        }
        final picture = await recordWaveformPicture(
          peaks: peaks,
          size: Size(w.toDouble(), kWaveformStripHeight),
          background: background,
          mode: mode,
        );
        final cur = _owned;
        if (cur == null || gen != _gen) {
          picture.dispose();
          return;
        }
        final next = cur.withTile(
          WaveformTile(startPx: startPx.toDouble(), picture: picture),
        );
        _owned = next;
        state = next;
      } catch (_) {
        continue;
      }
      await Future<void>.delayed(Duration.zero);
    }
  }

  void _dropAfterFrame(WaveformStrip? strip) {
    if (strip == null) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      strip.dispose();
    });
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
/// Strip pictures bake the lane background, so the theme colour is part of the
/// key: flipping the theme records a fresh strip instead of showing dark
/// pictures on a light lane.
final NotifierProviderFamily<
  WaveformStripNotifier,
  WaveformStrip?,
  (String, int, Color)
>
waveformStripProvider = NotifierProvider.autoDispose
    .family<WaveformStripNotifier, WaveformStrip?, (String, int, Color)>(
      WaveformStripNotifier.new,
    );
