import 'dart:async';
import 'dart:ui';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/overlay_providers.dart';
import 'package:gui_flutter/mixer/waveform/spectral_color.dart';
import 'package:gui_flutter/mixer/waveform/waveform_ring.dart';
import 'package:gui_flutter/shell/app_typography.dart';

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
    final slipOn = ref.watch(deckSlipEnabledProvider(widget.deckId));
    final slipShadowMs = ref.watch(deckSlipShadowMsProvider(widget.deckId));

    // Do not watch deckPositionMs here: during play the AnimationController
    // owns the scroll; watching would rebuild this lane on every engine poll.
    if (!advancing && !_scrubbing && durationMs > 0) {
      final enginePosMs = ref.read(deckPositionMsProvider(widget.deckId));
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
      if (!advancingNow) {
        _setDisplayMs(
          engineMs,
          durationMs: durationMs,
          speed: speedNow,
          playing: false,
        );
        return;
      }
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
        final visibleMs = cropVisibleMs(
          durationMs: durationMs,
          viewportWidth: width,
          speed: speed,
        );
        final ringArg = (widget.deckId, width.round(), visibleMs);
        final ring = trackId == null || durationMs <= 0
            ? null
            : ref.watch(waveformRingProvider(ringArg));
        final beatGrid = trackId == null || durationMs <= 0
            ? null
            : ref.watch(ringBeatGridPictureProvider(ringArg));
        final loops = trackId == null || durationMs <= 0
            ? null
            : ref.watch(ringLoopPictureProvider(ringArg));
        final activeLoop = durationMs <= 0
            ? null
            : ref.watch(ringActiveLoopPictureProvider(ringArg));
        final pendingLoopIn = durationMs <= 0
            ? null
            : ref.watch(ringPendingLoopInPictureProvider(ringArg));
        final cues = trackId == null || durationMs <= 0
            ? null
            : ref.watch(ringCuePictureProvider(ringArg));
        final basePxPerMs =
            ring?.pxPerMs ??
            stripDisplayPxPerMs(
              pxPerMs: stripPxPerMs(durationMs),
              speed: speed,
            );
        final pxPerMs = basePxPerMs;
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
                if (ring != null)
                  AnimatedBuilder(
                    animation: _playhead,
                    builder: (context, child) {
                      final positionMs = _displayMs(durationMs);
                      final dx = snapPx(
                        playheadDx(
                          positionMs: positionMs,
                          originMs: ring.originMs,
                          width: width,
                          pxPerMs: pxPerMs,
                        ),
                        dpr,
                      );
                      // Transform a cached layer — avoids Stack/Positioned
                      // relayout of a 2×-viewport picture every vsync.
                      return Transform.translate(
                        offset: Offset(dx, 0),
                        filterQuality: FilterQuality.none,
                        child: child,
                      );
                    },
                    child: SizedBox(
                      width: ring.widthPx.toDouble(),
                      height: height,
                      child: RepaintBoundary(
                        child: _RingLayer(
                          ring: ring,
                          height: height,
                          beatGrid: beatGrid,
                          loops: loops,
                          activeLoop: activeLoop,
                          pendingLoopIn: pendingLoopIn,
                          cues: cues,
                        ),
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
                if (slipOn && slipShadowMs != null && ring != null)
                  AnimatedBuilder(
                    animation: _playhead,
                    builder: (context, child) {
                      final audibleMs = _displayMs(durationMs);
                      final delta = slipShadowMs - audibleMs;
                      if (delta.abs() < 3) {
                        return const SizedBox.shrink();
                      }
                      return Transform.translate(
                        offset: Offset(
                          snapPx(width / 2 + delta * pxPerMs - 0.5, dpr),
                          0,
                        ),
                        filterQuality: FilterQuality.none,
                        child: child,
                      );
                    },
                    child: ColoredBox(
                      color: theme.colors.primary.withValues(alpha: 0.65),
                      child: SizedBox(width: 1, height: height),
                    ),
                  ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 10),
                    child: Text(
                      widget.label.toUpperCase(),
                      style: theme.typography.display.xs.copyWith(
                        fontFamily: MixarFonts.spaceGrotesk,
                        color: FaderColors.forAccent(
                          faderAccentForDeck(widget.deckId) ??
                              FaderAccent.neutral,
                        ).grip.withValues(alpha: 0.55),
                        fontWeight: FontWeight.w700,
                        letterSpacing: 0.6,
                        height: 1,
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

class _RingLayer extends StatelessWidget {
  const _RingLayer({
    required this.ring,
    required this.height,
    required this.beatGrid,
    required this.loops,
    required this.activeLoop,
    required this.pendingLoopIn,
    required this.cues,
  });

  final WaveformRing ring;
  final double height;
  final Picture? beatGrid;
  final Picture? loops;
  final Picture? activeLoop;
  final Picture? pendingLoopIn;
  final Picture? cues;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: ring.widthPx.toDouble(),
      height: height,
      child: CustomPaint(
        painter: _RingPainter(
          ring: ring,
          beatGrid: beatGrid,
          loops: loops,
          activeLoop: activeLoop,
          pendingLoopIn: pendingLoopIn,
          cues: cues,
        ),
        size: Size(ring.widthPx.toDouble(), height),
      ),
    );
  }
}

class _RingPainter extends CustomPainter {
  _RingPainter({
    required this.ring,
    required this.beatGrid,
    required this.loops,
    required this.activeLoop,
    required this.pendingLoopIn,
    required this.cues,
  });

  final WaveformRing ring;
  final Picture? beatGrid;
  final Picture? loops;
  final Picture? activeLoop;
  final Picture? pendingLoopIn;
  final Picture? cues;

  @override
  void paint(Canvas canvas, Size size) {
    if (ring.heightPx <= 0) {
      return;
    }
    final sy = size.height / ring.heightPx;
    canvas.save();
    canvas.scale(1, sy);
    canvas.drawPicture(ring.composite);
    for (final picture in [beatGrid, loops, activeLoop, pendingLoopIn, cues]) {
      if (picture != null) {
        canvas.drawPicture(picture);
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(_RingPainter oldDelegate) =>
      !identical(ring, oldDelegate.ring) ||
      !identical(beatGrid, oldDelegate.beatGrid) ||
      !identical(loops, oldDelegate.loops) ||
      !identical(activeLoop, oldDelegate.activeLoop) ||
      !identical(pendingLoopIn, oldDelegate.pendingLoopIn) ||
      !identical(cues, oldDelegate.cues);
}
