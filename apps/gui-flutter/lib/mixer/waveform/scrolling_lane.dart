import 'dart:async';
import 'dart:ui';

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/overlay_providers.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_picture.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/mixer/waveform/waveform_strip.dart';

class ScrollingLane extends ConsumerStatefulWidget {
  const ScrollingLane({required this.deckId, required this.label, super.key});

  final int deckId;
  final String label;

  @override
  ConsumerState<ScrollingLane> createState() => _ScrollingLaneState();
}

class _ScrollingLaneState extends ConsumerState<ScrollingLane>
    with SingleTickerProviderStateMixin {
  late final AnimationController _playhead;
  var _scrubbing = false;
  var _scrubAnchorX = 0.0;
  var _scrubAnchorMs = 0.0;
  final _seekClock = Stopwatch();

  var _zoomGen = 0;
  Picture? _zoomPicture;
  var _zoomStartMs = 0;
  var _zoomEndMs = 0;
  var _zoomPicWidth = 0.0;
  var _zoomForVisibleMs = 0;
  var _zoomTrackId = '';
  var _zoomPendingKey = '';

  @override
  void initState() {
    super.initState();
    _playhead = AnimationController(vsync: this);
  }

  @override
  void dispose() {
    _dropZoomPicture(_zoomPicture);
    _playhead.dispose();
    super.dispose();
  }

  void _dropZoomPicture(Picture? picture) {
    if (picture == null) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      picture.dispose();
    });
  }

  double _displayMs(int durationMs) {
    if (durationMs <= 0) {
      return 0;
    }
    return _playhead.value * durationMs;
  }

  void _setDisplayMs(
    double ms, {
    required int durationMs,
    required double speed,
    required bool playing,
  }) {
    if (durationMs <= 0) {
      _playhead
        ..stop()
        ..value = 0;
      return;
    }
    _playhead
      ..stop()
      ..duration = playheadWallDuration(durationMs: durationMs, speed: speed)
      ..value = (ms / durationMs).clamp(0.0, 1.0);
    if (playing && !_scrubbing) {
      _playhead.forward();
    }
  }

  void _syncPlayback({
    required bool playing,
    required int durationMs,
    required double speed,
  }) {
    if (durationMs <= 0) {
      _playhead.stop();
      return;
    }
    final ms = _displayMs(durationMs);
    final wall = playheadWallDuration(durationMs: durationMs, speed: speed);
    if (_playhead.duration != wall) {
      _playhead
        ..stop()
        ..duration = wall
        ..value = (ms / durationMs).clamp(0.0, 1.0);
    }
    if (playing && !_scrubbing) {
      if (!_playhead.isAnimating) {
        _playhead.forward();
      }
    } else if (_playhead.isAnimating) {
      _playhead.stop();
    }
  }

  bool _advancingNow() => playheadAdvancing(
    playing: ref.read(deckPlayingProvider(widget.deckId)),
    jogTouching: ref.read(deckJogTouchingProvider(widget.deckId)),
  );

  void _scheduleZoomDetail({
    required String trackId,
    required int durationMs,
    required int positionMs,
    required int visibleMs,
    required double width,
  }) {
    if (durationMs <= 0 || visibleMs <= 0 || width <= 0) {
      return;
    }
    final needs =
        _zoomPicture == null ||
        _zoomTrackId != trackId ||
        _zoomForVisibleMs != visibleMs ||
        l1NeedsRefresh(
          positionMs: positionMs.toDouble(),
          detailStartMs: _zoomStartMs,
          detailEndMs: _zoomEndMs,
          visibleMs: visibleMs,
          durationMs: durationMs,
        );
    if (!needs) {
      return;
    }
    final range = l1Range(
      positionMs: positionMs,
      visibleMs: visibleMs,
      durationMs: durationMs,
    );
    final pendingKey =
        '$trackId|$visibleMs|${range.startMs}|${range.endMs}|${width.round()}';
    if (pendingKey == _zoomPendingKey) {
      return;
    }
    _zoomPendingKey = pendingKey;
    final gen = ++_zoomGen;
    final buckets = l1BucketCount(
      startMs: range.startMs,
      endMs: range.endMs,
      visibleMs: visibleMs,
      width: width,
    );
    unawaited(
      _fetchZoomDetail(
        gen: gen,
        trackId: trackId,
        durationMs: durationMs,
        visibleMs: visibleMs,
        startMs: range.startMs,
        endMs: range.endMs,
        buckets: buckets,
      ),
    );
  }

  Future<void> _fetchZoomDetail({
    required int gen,
    required String trackId,
    required int durationMs,
    required int visibleMs,
    required int startMs,
    required int endMs,
    required int buckets,
  }) async {
    try {
      final lib = await ref.read(libraryTransportProvider.future);
      final overview =
          ref.read(waveformOverviewProvider(trackId)).value ?? const [];
      final mode = ref.read(waveformDisplayModeProvider);
      final packed = await lib.getWaveformWindow(
        trackId: trackId,
        startMs: startMs,
        endMs: endMs,
        buckets: buckets,
      );
      if (!mounted || gen != _zoomGen) {
        return;
      }
      final detail = DetailWindow(
        peaks: decodeRgbPeaks(packed.rgb),
        startMs: packed.startMs,
        endMs: packed.endMs,
      );
      if (detail.peaks.length < 2) {
        if (gen == _zoomGen) {
          _zoomPendingKey = '';
        }
        return;
      }
      final spanMs = (detail.endMs - detail.startMs).toDouble();
      if (spanMs <= 0) {
        if (gen == _zoomGen) {
          _zoomPendingKey = '';
        }
        return;
      }
      final picW = buckets.toDouble().clamp(16.0, 16384.0).toDouble();
      final picture = recordWaveformPicture(
        overview: overview,
        detail: detail,
        durationMs: durationMs,
        originMs: detail.startMs.toDouble(),
        spanMs: spanMs,
        size: Size(picW, kWaveformStripHeight),
        fallbackToOverview: false,
        // Overlay on strip L0/L1 — never blank the layers underneath.
        fillBackground: false,
        mode: mode,
      );
      final prev = _zoomPicture;
      setState(() {
        _zoomPicture = picture;
        _zoomStartMs = detail.startMs;
        _zoomEndMs = detail.endMs;
        _zoomPicWidth = picW;
        _zoomForVisibleMs = visibleMs;
        _zoomTrackId = trackId;
      });
      _dropZoomPicture(prev);
    } catch (_) {
      if (gen == _zoomGen) {
        _zoomPendingKey = '';
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final advancing = playheadAdvancing(
      playing: ref.watch(deckPlayingProvider(widget.deckId)),
      jogTouching: ref.watch(deckJogTouchingProvider(widget.deckId)),
    );
    final speed = ref.watch(deckSpeedRatioProvider(widget.deckId));
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    final durationMs = ref.watch(deckDurationMsProvider(widget.deckId)) ?? 0;
    final enginePosMs = ref.watch(deckPositionMsProvider(widget.deckId));
    final strip = trackId == null || durationMs <= 0
        ? null
        : ref.watch(waveformStripProvider((trackId, durationMs)));
    final beatGridData = trackId == null || durationMs <= 0
        ? null
        : ref.watch(beatGridProvider(trackId));
    final loops = trackId == null || durationMs <= 0
        ? null
        : ref.watch(stripLoopPictureProvider((trackId, durationMs)));
    final activeLoop = durationMs <= 0
        ? null
        : ref.watch(
            stripActiveLoopPictureProvider((widget.deckId, durationMs)),
          );
    final cues = trackId == null || durationMs <= 0
        ? null
        : ref.watch(stripCuePictureProvider((trackId, durationMs)));

    // When not interpolating (pause / vinyl touch), drive the lane from the
    // engine playhead every build so jog Position updates cannot be missed.
    if (!advancing && !_scrubbing && durationMs > 0) {
      final v = (enginePosMs / durationMs).clamp(0.0, 1.0);
      if ((_playhead.value - v).abs() > 1e-12) {
        _playhead.value = v;
      }
    }

    _syncPlayback(playing: advancing, durationMs: durationMs, speed: speed);

    ref.listen(deckPositionMsProvider(widget.deckId), (prev, next) {
      if (_scrubbing || durationMs <= 0) {
        return;
      }
      final display = _displayMs(durationMs);
      final engineMs = next.toDouble();
      final advancingNow = _advancingNow();
      final speedNow = ref.read(deckSpeedRatioProvider(widget.deckId));
      if (playheadShouldSnap(
        displayMs: display,
        engineMs: engineMs,
        playing: advancingNow,
      )) {
        _setDisplayMs(
          engineMs,
          durationMs: durationMs,
          speed: speedNow,
          playing: advancingNow,
        );
        return;
      }
      final corrected = correctPlayheadDrift(
        displayMs: display,
        estimateMs: engineMs,
      );
      if (corrected != display) {
        _setDisplayMs(
          corrected,
          durationMs: durationMs,
          speed: speedNow,
          playing: advancingNow,
        );
      }
    });
    ref.listen(deckJogTouchingProvider(widget.deckId), (prev, next) {
      if (_scrubbing || durationMs <= 0) {
        return;
      }
      final advancingNow = playheadAdvancing(
        playing: ref.read(deckPlayingProvider(widget.deckId)),
        jogTouching: next,
      );
      final speedNow = ref.read(deckSpeedRatioProvider(widget.deckId));
      if (!advancingNow) {
        _setDisplayMs(
          ref.read(deckPositionMsProvider(widget.deckId)).toDouble(),
          durationMs: durationMs,
          speed: speedNow,
          playing: false,
        );
        return;
      }
      _syncPlayback(playing: true, durationMs: durationMs, speed: speedNow);
    });
    ref.listen(deckSpeedRatioProvider(widget.deckId), (prev, next) {
      if (durationMs <= 0) {
        return;
      }
      _syncPlayback(
        playing: _advancingNow(),
        durationMs: durationMs,
        speed: next,
      );
    });

    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final height = constraints.maxHeight;
        if (width <= 0 || height <= 0) {
          return const SizedBox.expand();
        }
        final visibleMs = clampWaveformVisibleMs(
          ref.watch(waveformVisibleMsProvider),
          durationMs: durationMs > 0 ? durationMs : null,
        );
        final pxPerMs = visibleMs > 0 ? width / visibleMs : 0.0;
        final stripNaturalW = (strip?.widthPx ?? stripWidthPx(durationMs))
            .toDouble();
        final stripScaledW = durationMs > 0 && pxPerMs > 0
            ? durationMs * pxPerMs
            : stripNaturalW;
        final scaleX = stripNaturalW > 0 ? stripScaledW / stripNaturalW : 1.0;
        final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;
        final gridBpm = beatGridData?.bpm;
        final beatMarks = gridBpm == null || durationMs <= 0 || stripScaledW <= 0
            ? const <BeatMark>[]
            : beatGridXs(
                bpm: gridBpm,
                firstBeatSecs: beatGridFirstBeatSecs(beatGridData),
                originMs: 0,
                spanMs: durationMs.toDouble(),
                width: stripScaledW,
              );

        if (trackId != null && durationMs > 0 && pxPerMs > 0) {
          _scheduleZoomDetail(
            trackId: trackId,
            durationMs: durationMs,
            positionMs: _displayMs(durationMs).round(),
            visibleMs: visibleMs,
            width: width,
          );
        }

        return Listener(
          onPointerSignal: durationMs <= 0
              ? null
              : (signal) {
                  if (signal is PointerScrollEvent &&
                      signal.scrollDelta.dy != 0) {
                    ref
                        .read(waveformVisibleMsProvider.notifier)
                        .zoomByScroll(
                          signal.scrollDelta.dy,
                          durationMs: durationMs,
                        );
                  }
                },
          onPointerDown: durationMs <= 0
              ? null
              : (e) {
                  _scrubbing = true;
                  _playhead.stop();
                  _scrubAnchorX = e.localPosition.dx;
                  _scrubAnchorMs = _displayMs(durationMs);
                },
          onPointerMove: (e) {
            if (!_scrubbing || durationMs <= 0) {
              return;
            }
            final ms = centerScrubMs(
              anchorPosMs: _scrubAnchorMs,
              deltaX: e.localPosition.dx - _scrubAnchorX,
              width: width,
              spanMs: visibleMs.toDouble(),
            );
            _playhead.value = (ms / durationMs).clamp(0.0, 1.0);
            _throttledSeek(ms.round());
          },
          onPointerUp: (e) {
            if (!_scrubbing) {
              return;
            }
            final ms = durationMs <= 0
                ? 0
                : centerScrubMs(
                    anchorPosMs: _scrubAnchorMs,
                    deltaX: e.localPosition.dx - _scrubAnchorX,
                    width: width,
                    spanMs: visibleMs.toDouble(),
                  ).round();
            _scrubbing = false;
            if (durationMs > 0) {
              _setDisplayMs(
                ms.toDouble(),
                durationMs: durationMs,
                speed: speed,
                playing: advancing,
              );
            }
            unawaited(_seek(ms));
          },
          onPointerCancel: (_) {
            _scrubbing = false;
            if (advancing && durationMs > 0) {
              _syncPlayback(
                playing: advancing,
                durationMs: durationMs,
                speed: speed,
              );
            }
          },
          child: ClipRect(
            child: Stack(
              fit: StackFit.expand,
              children: [
                const ColoredBox(color: kWaveformBg),
                if (strip != null)
                  AnimatedBuilder(
                    animation: _playhead,
                    builder: (context, child) {
                      final positionMs = _displayMs(durationMs);
                      return Positioned(
                        left: snapPx(
                          stripTranslateX(
                            positionMs: positionMs,
                            viewportWidth: width,
                            pxPerMs: pxPerMs,
                          ),
                          dpr,
                        ),
                        top: 0,
                        bottom: 0,
                        width: stripScaledW,
                        child: child ?? const SizedBox.shrink(),
                      );
                    },
                    child: RepaintBoundary(
                      child: _StripLayer(
                        strip: strip,
                        height: height,
                        scaledWidth: stripScaledW,
                        scaleX: scaleX,
                        beatMarks: beatMarks,
                        loops: loops,
                        activeLoop: activeLoop,
                        cues: cues,
                        zoomPicture: _zoomPicture,
                        zoomStartMs: _zoomStartMs,
                        zoomEndMs: _zoomEndMs,
                        zoomPicWidth: _zoomPicWidth,
                      ),
                    ),
                  ),
                Align(
                  alignment: Alignment.center,
                  child: ColoredBox(
                    color: theme.colors.foreground.withValues(alpha: 0.9),
                    child: const SizedBox(width: 1, height: double.infinity),
                  ),
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    child: Text(
                      widget.label,
                      style: theme.typography.body.xs.copyWith(
                        color: theme.colors.mutedForeground,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  void _throttledSeek(int ms) {
    if (_seekClock.isRunning && _seekClock.elapsedMilliseconds < 32) {
      return;
    }
    _seekClock
      ..reset()
      ..start();
    unawaited(_seek(ms));
  }

  Future<void> _seek(int ms) async {
    try {
      await seekDeck(ref, widget.deckId, ms);
    } catch (e) {
      if (!mounted) {
        return;
      }
      showFToast(context: context, variant: .destructive, title: Text('$e'));
    }
  }
}

class _StripLayer extends StatelessWidget {
  const _StripLayer({
    required this.strip,
    required this.height,
    required this.scaledWidth,
    required this.scaleX,
    required this.beatMarks,
    required this.loops,
    required this.activeLoop,
    required this.cues,
    required this.zoomPicture,
    required this.zoomStartMs,
    required this.zoomEndMs,
    required this.zoomPicWidth,
  });

  final WaveformStrip strip;
  final double height;
  final double scaledWidth;
  final double scaleX;
  final List<BeatMark> beatMarks;
  final Picture? loops;
  final Picture? activeLoop;
  final Picture? cues;
  final Picture? zoomPicture;
  final int zoomStartMs;
  final int zoomEndMs;
  final double zoomPicWidth;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: scaledWidth,
      height: height,
      child: CustomPaint(
        painter: _StripPainter(
          strip: strip,
          scaleX: scaleX,
          beatMarks: beatMarks,
          loops: loops,
          activeLoop: activeLoop,
          cues: cues,
          zoomPicture: zoomPicture,
          zoomStartMs: zoomStartMs,
          zoomEndMs: zoomEndMs,
          zoomPicWidth: zoomPicWidth,
        ),
        size: Size(scaledWidth, height),
      ),
    );
  }
}

class _StripPainter extends CustomPainter {
  _StripPainter({
    required this.strip,
    required this.scaleX,
    required this.beatMarks,
    required this.loops,
    required this.activeLoop,
    required this.cues,
    required this.zoomPicture,
    required this.zoomStartMs,
    required this.zoomEndMs,
    required this.zoomPicWidth,
  });

  final WaveformStrip strip;
  final double scaleX;
  final List<BeatMark> beatMarks;
  final Picture? loops;
  final Picture? activeLoop;
  final Picture? cues;
  final Picture? zoomPicture;
  final int zoomStartMs;
  final int zoomEndMs;
  final double zoomPicWidth;

  static final _barPaint = Paint()
    ..color = const Color.fromARGB(80, 200, 205, 215)
    ..strokeWidth = 1
    ..isAntiAlias = false;

  static final _beatPaint = Paint()
    ..color = const Color.fromARGB(55, 170, 175, 185)
    ..strokeWidth = 1
    ..isAntiAlias = false;

  @override
  void paint(Canvas canvas, Size size) {
    if (strip.heightPx <= 0 || scaleX <= 0) {
      return;
    }
    final sy = size.height / strip.heightPx;
    canvas.save();
    canvas.scale(scaleX, sy);
    canvas.drawPicture(strip.l0);
    for (final tile in strip.tiles) {
      canvas.save();
      canvas.translate(tile.startPx, 0);
      canvas.drawPicture(tile.picture);
      canvas.restore();
    }
    final zoom = zoomPicture;
    if (zoom != null &&
        strip.durationMs > 0 &&
        zoomEndMs > zoomStartMs &&
        zoomPicWidth > 0 &&
        strip.widthPx > 0) {
      final startPx = zoomStartMs / strip.durationMs * strip.widthPx;
      final endPx = zoomEndMs / strip.durationMs * strip.widthPx;
      final destW = endPx - startPx;
      if (destW > 0) {
        canvas.save();
        canvas.translate(startPx, 0);
        canvas.scale(destW / zoomPicWidth, 1);
        canvas.drawPicture(zoom);
        canvas.restore();
      }
    }
    // Loops/cues stay in strip space so they track the scaled waveform.
    for (final picture in [loops, activeLoop, cues]) {
      if (picture != null) {
        canvas.drawPicture(picture);
      }
    }
    canvas.restore();

    // Beat grid: positions in scaled strip space, stroke stays 1 device px.
    for (final mark in beatMarks) {
      final x = mark.x.roundToDouble();
      canvas.drawLine(
        Offset(x, 0),
        Offset(x, size.height),
        mark.isBar ? _barPaint : _beatPaint,
      );
    }
  }

  @override
  bool shouldRepaint(_StripPainter oldDelegate) =>
      !identical(strip, oldDelegate.strip) ||
      scaleX != oldDelegate.scaleX ||
      !identical(beatMarks, oldDelegate.beatMarks) ||
      !identical(loops, oldDelegate.loops) ||
      !identical(activeLoop, oldDelegate.activeLoop) ||
      !identical(cues, oldDelegate.cues) ||
      !identical(zoomPicture, oldDelegate.zoomPicture) ||
      zoomStartMs != oldDelegate.zoomStartMs ||
      zoomEndMs != oldDelegate.zoomEndMs ||
      zoomPicWidth != oldDelegate.zoomPicWidth;
}
