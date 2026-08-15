import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/mixer/waveform/waveform_strip.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

class ScrollingLane extends ConsumerStatefulWidget {
  const ScrollingLane({required this.deckId, required this.label, super.key});

  final int deckId;
  final String label;

  @override
  ConsumerState<ScrollingLane> createState() => _ScrollingLaneState();
}

class _ScrollingLaneState extends ConsumerState<ScrollingLane>
    with SingleTickerProviderStateMixin {
  late final Ticker _ticker;
  final _playhead = ValueNotifier(0.0);
  var _displayMs = 0.0;
  var _anchorMs = 0.0;
  var _anchorElapsed = Duration.zero;
  var _elapsed = Duration.zero;
  var _lastElapsed = Duration.zero;
  var _scrubbing = false;
  var _scrubAnchorX = 0.0;
  var _scrubAnchorMs = 0.0;
  final _seekClock = Stopwatch();

  @override
  void initState() {
    super.initState();
    _ticker = createTicker(_onTick);
  }

  @override
  void dispose() {
    _ticker.dispose();
    _playhead.dispose();
    super.dispose();
  }

  void _onTick(Duration elapsed) {
    final dt = (elapsed - _lastElapsed).inMicroseconds / 1e3;
    _lastElapsed = elapsed;
    _elapsed = elapsed;
    if (_scrubbing || !mounted || dt <= 0) {
      return;
    }
    final playing = ref.read(deckPlayingProvider(widget.deckId));
    if (!playing) {
      return;
    }
    final speed = ref.read(deckSpeedRatioProvider(widget.deckId));
    final estimate = engineEstimateMs(
      anchorMs: _anchorMs,
      ageMs: (elapsed - _anchorElapsed).inMicroseconds / 1e3,
      speed: speed,
    );
    _displayMs = correctPlayheadDrift(
      displayMs: _displayMs + dt * speed,
      estimateMs: estimate,
    );
    _playhead.value = _displayMs;
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final playing = ref.watch(deckPlayingProvider(widget.deckId));
    if (playing && !_ticker.isTicking) {
      _elapsed = Duration.zero;
      _lastElapsed = Duration.zero;
      _anchorElapsed = Duration.zero;
      _anchorMs = _displayMs;
      _playhead.value = _displayMs;
      _ticker.start();
    } else if (!playing && _ticker.isTicking) {
      _ticker.stop();
    }
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    final durationMs = ref.watch(deckDurationMsProvider(widget.deckId)) ?? 0;
    final strip = trackId == null || durationMs <= 0
        ? null
        : ref.watch(waveformStripProvider((trackId, durationMs)));
    final grid = trackId == null
        ? null
        : ref.watch(beatGridProvider(trackId)).value;

    ref.listen(deckPositionMsProvider(widget.deckId), (prev, next) {
      _anchorMs = next.toDouble();
      _anchorElapsed = _elapsed;
      if (_scrubbing) {
        return;
      }
      final playingNow = ref.read(deckPlayingProvider(widget.deckId));
      if (!playheadShouldSnap(
        displayMs: _displayMs,
        engineMs: next.toDouble(),
        playing: playingNow,
      )) {
        return;
      }
      _displayMs = next.toDouble();
      _playhead.value = _displayMs;
    });
    ref.listen(deckSpeedRatioProvider(widget.deckId), (prev, next) {
      _anchorMs = _displayMs;
      _anchorElapsed = _elapsed;
    });

    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final height = constraints.maxHeight;
        if (width <= 0 || height <= 0) {
          return const SizedBox.expand();
        }
        final pxPerMs = strip?.pxPerMs ?? stripPxPerMs(durationMs);
        final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;

        return Listener(
          onPointerDown: durationMs <= 0
              ? null
              : (e) {
                  _scrubbing = true;
                  _scrubAnchorX = e.localPosition.dx;
                  _scrubAnchorMs = _displayMs;
                },
          onPointerMove: (e) {
            if (!_scrubbing) {
              return;
            }
            final ms = centerScrubMs(
              anchorPosMs: _scrubAnchorMs,
              deltaX: e.localPosition.dx - _scrubAnchorX,
              width: width,
              spanMs: cropVisibleMs(
                durationMs: durationMs,
                viewportWidth: width,
              ).toDouble(),
            );
            _displayMs = ms;
            _playhead.value = ms;
            _throttledSeek(ms.round());
          },
          onPointerUp: (e) {
            if (!_scrubbing) {
              return;
            }
            final ms = centerScrubMs(
              anchorPosMs: _scrubAnchorMs,
              deltaX: e.localPosition.dx - _scrubAnchorX,
              width: width,
              spanMs: cropVisibleMs(
                durationMs: durationMs,
                viewportWidth: width,
              ).toDouble(),
            ).round();
            _scrubbing = false;
            unawaited(_seek(ms));
          },
          onPointerCancel: (_) => _scrubbing = false,
          child: ClipRect(
            child: Stack(
              fit: StackFit.expand,
              children: [
                const ColoredBox(color: kWaveformBg),
                if (strip != null)
                  AnimatedBuilder(
                    animation: _playhead,
                    builder: (context, child) {
                      return Positioned(
                        left: snapPx(
                          stripTranslateX(
                            positionMs: _playhead.value,
                            viewportWidth: width,
                            pxPerMs: pxPerMs,
                          ),
                          dpr,
                        ),
                        top: 0,
                        bottom: 0,
                        width: strip.widthPx.toDouble(),
                        child: child ?? const SizedBox.shrink(),
                      );
                    },
                    child: RepaintBoundary(
                      child: _StripLayer(
                        strip: strip,
                        height: height,
                        grid: grid,
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
    required this.grid,
  });

  final WaveformStrip strip;
  final double height;
  final BeatGridData? grid;

  @override
  Widget build(BuildContext context) {
    final grid = this.grid;
    final marks = grid == null || grid.bpm == null
        ? const <BeatMark>[]
        : beatGridXs(
            bpm: grid.bpm!,
            firstBeatSecs: grid.beats.isEmpty ? 0 : grid.beats.first,
            originMs: 0,
            spanMs: strip.durationMs.toDouble(),
            width: strip.widthPx.toDouble(),
          );
    return SizedBox(
      width: strip.widthPx.toDouble(),
      height: height,
      child: Stack(
        fit: StackFit.expand,
        children: [
          CustomPaint(
            painter: _StripPainter(strip: strip),
            size: Size(strip.widthPx.toDouble(), height),
          ),
          CustomPaint(painter: _BeatGridPainter(marks: marks)),
        ],
      ),
    );
  }
}

class _StripPainter extends CustomPainter {
  _StripPainter({required this.strip});

  final WaveformStrip strip;

  @override
  void paint(Canvas canvas, Size size) {
    if (strip.heightPx <= 0) {
      return;
    }
    final sy = size.height / strip.heightPx;
    canvas.scale(1, sy);
    canvas.drawPicture(strip.l0);
    for (final tile in strip.tiles) {
      canvas.save();
      canvas.translate(tile.startPx, 0);
      canvas.drawPicture(tile.picture);
      canvas.restore();
    }
  }

  @override
  bool shouldRepaint(_StripPainter oldDelegate) =>
      !identical(strip, oldDelegate.strip);
}

class _BeatGridPainter extends CustomPainter {
  _BeatGridPainter({required this.marks});

  final List<BeatMark> marks;

  @override
  void paint(Canvas canvas, Size size) {
    final bar = Paint()
      ..color = const Color.fromARGB(80, 200, 205, 215)
      ..strokeWidth = 1
      ..isAntiAlias = false;
    final beat = Paint()
      ..color = const Color.fromARGB(55, 170, 175, 185)
      ..strokeWidth = 1
      ..isAntiAlias = false;
    for (final mark in marks) {
      final x = mark.x.roundToDouble();
      canvas.drawLine(
        Offset(x, 0),
        Offset(x, size.height),
        mark.isBar ? bar : beat,
      );
    }
  }

  @override
  bool shouldRepaint(_BeatGridPainter oldDelegate) =>
      !listEquals(marks, oldDelegate.marks);
}
