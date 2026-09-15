import 'dart:async';
import 'dart:ui';

import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_picture.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

class WaveformRing {
  WaveformRing({
    required this.originMs,
    required this.chunkMs,
    required this.chunkPx,
    required this.chunkCount,
    required this.durationMs,
    required this.chunks,
    required this.composite,
  });

  final double originMs;
  final double chunkMs;
  final int chunkPx;
  final int chunkCount;
  final int durationMs;
  final List<Picture> chunks;
  final Picture composite;

  double get spanMs => chunkMs * chunkCount;
  int get widthPx => chunkPx * chunkCount;
  double get pxPerMs => spanMs > 0 ? widthPx / spanMs : 0;
  int get heightPx => kWaveformStripHeight.round();

  void disposePictures({required bool includeChunks}) {
    composite.dispose();
    if (includeChunks) {
      for (final c in chunks) {
        c.dispose();
      }
    }
  }
}

class WaveformRingNotifier extends Notifier<WaveformRing?> {
  WaveformRingNotifier(this.arg);

  /// (deckId, windowPx, visibleMs)
  final (int, int, int) arg;
  var _gen = 0;
  WaveformRing? _owned;
  var _sliding = false;

  int get _deckId => arg.$1;
  int get _windowPx => arg.$2;
  int get _visibleMs => arg.$3;

  @override
  WaveformRing? build() {
    final trackId = ref.watch(deckTrackIdProvider(_deckId));
    final durationMs = ref.watch(deckDurationMsProvider(_deckId)) ?? 0;
    final mode = ref.watch(waveformDisplayModeProvider);
    ref.onDispose(() {
      _dropAfterFrame(_owned, includeChunks: true);
      _owned = null;
    });

    ref.listen(deckPositionMsProvider(_deckId), (prev, next) {
      if (_owned == null || _sliding) {
        return;
      }
      final ms = next.toDouble();
      if (_needsFullRebuild(ms)) {
        unawaited(_rebuild(trackId, durationMs, mode, ms));
        return;
      }
      if (ringNeedsSlide(
        positionMs: ms,
        ringOriginMs: _owned!.originMs,
        chunkMs: _owned!.chunkMs,
        chunkCount: _owned!.chunkCount,
        durationMs: durationMs,
      )) {
        unawaited(_slide(trackId, durationMs, mode, ms));
      }
    });

    if (trackId == null || durationMs <= 0 || _windowPx < 1 || _visibleMs < 1) {
      _publish(null);
      return null;
    }

    final pos = ref.read(deckPositionMsProvider(_deckId)).toDouble();
    unawaited(_rebuild(trackId, durationMs, mode, pos));
    return _owned;
  }

  bool _needsFullRebuild(double positionMs) {
    final ring = _owned;
    if (ring == null || ring.chunkMs <= 0) {
      return true;
    }
    final center = ring.originMs + ring.spanMs / 2;
    return (positionMs - center).abs() >
        ring.chunkMs * kWaveformRingVisibleChunks;
  }

  Future<void> _rebuild(
    String? trackId,
    int durationMs,
    WaveformDisplayMode mode,
    double positionMs,
  ) async {
    if (trackId == null || durationMs <= 0) {
      _publish(null);
      return;
    }
    final gen = ++_gen;
    final chunkPx = ringChunkPx(_windowPx.toDouble());
    final range = ringRange(
      positionMs: positionMs,
      visibleMs: _visibleMs.toDouble(),
      durationMs: durationMs,
    );
    if (range.chunkCount < 1 || range.chunkMs <= 0) {
      _publish(null);
      return;
    }

    final lib = await ref.read(libraryTransportProvider.future);
    if (!ref.mounted || gen != _gen) {
      return;
    }

    final chunks = <Picture>[];
    for (var i = 0; i < range.chunkCount; i++) {
      if (!ref.mounted || gen != _gen) {
        _disposePictures(chunks);
        return;
      }
      final startMs = (range.originMs + i * range.chunkMs).round();
      final endMs = (range.originMs + (i + 1) * range.chunkMs).round().clamp(
        startMs + 1,
        durationMs,
      );
      chunks.add(
        await _paintChunk(
          lib: lib,
          trackId: trackId,
          durationMs: durationMs,
          startMs: startMs,
          endMs: endMs,
          chunkPx: chunkPx,
          mode: mode,
        ),
      );
    }
    if (!ref.mounted || gen != _gen) {
      _disposePictures(chunks);
      return;
    }
    final composite = recordCompositePicture(
      chunks: chunks,
      chunkPx: chunkPx,
      height: kWaveformStripHeight,
    );
    _publish(
      WaveformRing(
        originMs: range.originMs,
        chunkMs: range.chunkMs,
        chunkPx: chunkPx,
        chunkCount: range.chunkCount,
        durationMs: durationMs,
        chunks: chunks,
        composite: composite,
      ),
    );
  }

  Future<void> _slide(
    String? trackId,
    int durationMs,
    WaveformDisplayMode mode,
    double positionMs,
  ) async {
    final cur = _owned;
    if (cur == null || trackId == null || _sliding) {
      return;
    }
    _sliding = true;
    final gen = _gen;
    try {
      final nextOrigin = ringSlideOriginMs(
        ringOriginMs: cur.originMs,
        chunkMs: cur.chunkMs,
        chunkCount: cur.chunkCount,
        positionMs: positionMs,
        durationMs: durationMs,
      );
      if ((nextOrigin - cur.originMs).abs() < 1e-6) {
        return;
      }
      final forward = nextOrigin > cur.originMs;
      final step = kWaveformRingSlideChunks;
      if (cur.chunks.length < step) {
        unawaited(_rebuild(trackId, durationMs, mode, positionMs));
        return;
      }

      final lib = await ref.read(libraryTransportProvider.future);
      if (!ref.mounted || gen != _gen) {
        return;
      }

      final kept = forward
          ? cur.chunks.sublist(step)
          : cur.chunks.sublist(0, cur.chunks.length - step);
      final dropped = forward
          ? cur.chunks.sublist(0, step)
          : cur.chunks.sublist(cur.chunks.length - step);

      final added = <Picture>[];
      for (var i = 0; i < step; i++) {
        final index = forward ? kept.length + i : i;
        final startMs = (nextOrigin + index * cur.chunkMs).round();
        final endMs = (nextOrigin + (index + 1) * cur.chunkMs).round().clamp(
          startMs + 1,
          durationMs,
        );
        added.add(
          await _paintChunk(
            lib: lib,
            trackId: trackId,
            durationMs: durationMs,
            startMs: startMs,
            endMs: endMs,
            chunkPx: cur.chunkPx,
            mode: mode,
          ),
        );
        if (!ref.mounted || gen != _gen) {
          _disposePictures(added);
          return;
        }
      }

      final chunks = forward ? [...kept, ...added] : [...added, ...kept];
      final composite = recordCompositePicture(
        chunks: chunks,
        chunkPx: cur.chunkPx,
        height: kWaveformStripHeight,
      );
      final next = WaveformRing(
        originMs: nextOrigin,
        chunkMs: cur.chunkMs,
        chunkPx: cur.chunkPx,
        chunkCount: chunks.length,
        durationMs: durationMs,
        chunks: chunks,
        composite: composite,
      );
      _owned = next;
      state = next;
      SchedulerBinding.instance.addPostFrameCallback((_) {
        cur.composite.dispose();
        for (final p in dropped) {
          p.dispose();
        }
      });
    } finally {
      _sliding = false;
    }
  }

  Future<Picture> _paintChunk({
    required LibraryTransport lib,
    required String trackId,
    required int durationMs,
    required int startMs,
    required int endMs,
    required int chunkPx,
    required WaveformDisplayMode mode,
  }) async {
    try {
      final packed = await lib.getWaveformWindow(
        trackId: trackId,
        startMs: startMs,
        endMs: endMs,
        buckets: chunkPx,
      );
      final detail = DetailWindow(
        peaks: decodeRgbPeaks(packed.rgb),
        startMs: packed.startMs,
        endMs: packed.endMs,
      );
      return recordWaveformPicture(
        overview: const [],
        detail: detail,
        durationMs: durationMs,
        originMs: startMs.toDouble(),
        spanMs: (endMs - startMs).toDouble(),
        size: Size(chunkPx.toDouble(), kWaveformStripHeight),
        fallbackToOverview: false,
        fillBackground: true,
        mode: mode,
      );
    } catch (_) {
      return recordWaveformPicture(
        overview: const [],
        detail: null,
        durationMs: durationMs,
        originMs: startMs.toDouble(),
        spanMs: (endMs - startMs).toDouble(),
        size: Size(chunkPx.toDouble(), kWaveformStripHeight),
        fallbackToOverview: false,
        fillBackground: true,
        mode: mode,
      );
    }
  }

  void _publish(WaveformRing? next) {
    final prev = _owned;
    _owned = next;
    state = next;
    _dropAfterFrame(prev, includeChunks: true);
  }

  void _dropAfterFrame(WaveformRing? ring, {required bool includeChunks}) {
    if (ring == null) {
      return;
    }
    SchedulerBinding.instance.addPostFrameCallback((_) {
      ring.disposePictures(includeChunks: includeChunks);
    });
  }

  void _disposePictures(List<Picture> pictures) {
    for (final p in pictures) {
      p.dispose();
    }
  }
}

final waveformRingProvider = NotifierProvider.autoDispose
    .family<WaveformRingNotifier, WaveformRing?, (int, int, int)>(
      WaveformRingNotifier.new,
    );
