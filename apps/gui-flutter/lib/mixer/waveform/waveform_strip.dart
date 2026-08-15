import 'dart:async';
import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/waveform_picture.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';

class WaveformTile {
  const WaveformTile({required this.startPx, required this.picture});

  final double startPx;
  final Picture picture;
}

class WaveformStrip {
  WaveformStrip({
    required this.durationMs,
    required this.widthPx,
    required this.heightPx,
    required this.l0,
    this.tiles = const [],
  });

  final int durationMs;
  final int widthPx;
  final int heightPx;
  final Picture l0;
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
  WaveformStripNotifier(this.arg);

  final (String, int) arg;
  var _gen = 0;
  WaveformStrip? _owned;

  @override
  WaveformStrip? build() {
    final (trackId, durationMs) = arg;
    final peaks = ref.watch(waveformOverviewProvider(trackId)).value;
    if (peaks == null || peaks.isEmpty || durationMs <= 0) {
      return null;
    }
    final gen = ++_gen;
    final width = stripWidthPx(durationMs);
    const height = kWaveformStripHeight;
    final l0 = recordWaveformPicture(
      overview: peaks,
      durationMs: durationMs,
      originMs: 0,
      spanMs: durationMs.toDouble(),
      size: Size(width.toDouble(), height),
    );
    final strip = WaveformStrip(
      durationMs: durationMs,
      widthPx: width,
      heightPx: height.round(),
      l0: l0,
    );
    _owned = strip;
    ref.onDispose(() {
      final toDrop = _owned;
      _owned = null;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        toDrop?.dispose();
      });
    });
    unawaited(_fillL1(trackId, peaks, durationMs, width, gen));
    return strip;
  }

  Future<void> _fillL1(
    String trackId,
    List<SpectralPeak> overview,
    int durationMs,
    int width,
    int gen,
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
        final detail = DetailWindow(
          peaks: decodeRgbPeaks(packed.rgb),
          startMs: packed.startMs,
          endMs: packed.endMs,
        );
        final picture = recordWaveformPicture(
          overview: overview,
          detail: detail,
          durationMs: durationMs,
          originMs: startMs.toDouble(),
          spanMs: (endMs - startMs).toDouble(),
          size: Size(w.toDouble(), kWaveformStripHeight),
          fallbackToOverview: false,
          fillBackground: true,
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

final waveformStripProvider =
    NotifierProvider.family<
      WaveformStripNotifier,
      WaveformStrip?,
      (String, int)
    >(WaveformStripNotifier.new);
