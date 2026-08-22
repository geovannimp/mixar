import 'dart:async';
import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/overlay_providers.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
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

  @override
  void initState() {
    super.initState();
    _playhead = AnimationController(vsync: this);
  }

  @override
  void dispose() {
    _playhead.dispose();
    super.dispose();
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
    final strip = trackId == null || durationMs <= 0
        ? null
        : ref.watch(waveformStripProvider((trackId, durationMs)));
    final beatGrid = trackId == null || durationMs <= 0
        ? null
        : ref.watch(stripBeatGridPictureProvider((trackId, durationMs)));
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
        final pxPerMs = strip?.pxPerMs ?? stripPxPerMs(durationMs);
        final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;

        return Listener(
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
              spanMs: cropVisibleMs(
                durationMs: durationMs,
                viewportWidth: width,
              ).toDouble(),
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
                    spanMs: cropVisibleMs(
                      durationMs: durationMs,
                      viewportWidth: width,
                    ).toDouble(),
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
                        width: strip.widthPx.toDouble(),
                        child: child ?? const SizedBox.shrink(),
                      );
                    },
                    child: RepaintBoundary(
                      child: _StripLayer(
                        strip: strip,
                        height: height,
                        beatGrid: beatGrid,
                        loops: loops,
                        activeLoop: activeLoop,
                        cues: cues,
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
    required this.beatGrid,
    required this.loops,
    required this.activeLoop,
    required this.cues,
  });

  final WaveformStrip strip;
  final double height;
  final Picture? beatGrid;
  final Picture? loops;
  final Picture? activeLoop;
  final Picture? cues;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: strip.widthPx.toDouble(),
      height: height,
      child: CustomPaint(
        painter: _StripPainter(
          strip: strip,
          beatGrid: beatGrid,
          loops: loops,
          activeLoop: activeLoop,
          cues: cues,
        ),
        size: Size(strip.widthPx.toDouble(), height),
      ),
    );
  }
}

class _StripPainter extends CustomPainter {
  _StripPainter({
    required this.strip,
    required this.beatGrid,
    required this.loops,
    required this.activeLoop,
    required this.cues,
  });

  final WaveformStrip strip;
  final Picture? beatGrid;
  final Picture? loops;
  final Picture? activeLoop;
  final Picture? cues;

  @override
  void paint(Canvas canvas, Size size) {
    if (strip.heightPx <= 0) {
      return;
    }
    final sy = size.height / strip.heightPx;
    canvas.save();
    canvas.scale(1, sy);
    canvas.drawPicture(strip.l0);
    for (final tile in strip.tiles) {
      canvas.save();
      canvas.translate(tile.startPx, 0);
      canvas.drawPicture(tile.picture);
      canvas.restore();
    }
    // Overlays are authored at strip height; scale with the waveform.
    for (final picture in [beatGrid, loops, activeLoop, cues]) {
      if (picture != null) {
        canvas.drawPicture(picture);
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(_StripPainter oldDelegate) =>
      !identical(strip, oldDelegate.strip) ||
      !identical(beatGrid, oldDelegate.beatGrid) ||
      !identical(loops, oldDelegate.loops) ||
      !identical(activeLoop, oldDelegate.activeLoop) ||
      !identical(cues, oldDelegate.cues);
}
