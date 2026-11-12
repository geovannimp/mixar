import 'dart:async';
import 'dart:ui';

import 'package:flutter/gestures.dart' show PointerScrollEvent, kPrimaryButton;
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/pad_modes.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/utils/waveform_gain.dart';
import 'package:gui_flutter/core/utils/waveform_layout.dart';
import 'package:gui_flutter/core/utils/waveform_picture.dart';
import 'package:gui_flutter/core/utils/waveform_section_painter.dart';
import 'package:gui_flutter/core/utils/waveform_spectral_color.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/overlay_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_providers.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_strip.dart';
import 'package:gui_flutter/providers/mixer/waveform/waveform_zoom.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';
import 'package:gui_flutter/widgets/mixer/track_end_blink.dart';
import 'package:gui_flutter/widgets/mixer/waveform/loop_band_delete.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/shell/mixar_toast.dart';

class ScrollingLane extends ConsumerStatefulWidget {
  const new({required this.deckId, this.label, this.nowMs, super.key});

  final int deckId;

  /// Deck letter painted in the lane's leading corner. Vertical lanes leave it
  /// null: the text would be rotated with the lane and unreadable.
  final String? label;

  /// Monotonic millisecond clock for the scrub-seek throttle.
  ///
  /// Defaults to a real wall clock. Tests inject a controllable clock so the
  /// 32 ms window does not depend on how fast the runner pumps events.
  final int Function()? nowMs;

  @override
  ConsumerState<ScrollingLane> createState() => _ScrollingLaneState();
}

class _ScrollingLaneState extends ConsumerState<ScrollingLane>
    with SingleTickerProviderStateMixin {
  late final AnimationController _playhead;
  var _scrubbing = false;
  var _scrubAnchorX = 0.0;
  var _scrubAnchorMs = 0.0;
  final _seekClock = Stopwatch()..start();
  int? _lastSeekMs;
  var _suppressSeek = false;

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
    final zoomMs = ref.watch(waveformZoomProvider);

    // Do not watch deckPositionMsProvider here — ~30Hz Position ticks would
    // rebuild LayoutBuilder/Listener/Stack. Seed pause/vinyl on play/jog
    // rebuilds; ongoing ticks drive _playhead via ref.listen below.
    if (!advancing && !_scrubbing && durationMs > 0) {
      final enginePosMs = ref.read(deckPositionMsProvider(widget.deckId));
      final v = (enginePosMs / durationMs).clamp(0.0, 1.0);
      if ((_playhead.value - v).abs() > 1e-12) {
        _playhead.value = v;
      }
    }

    _syncPlayback(playing: advancing, durationMs: durationMs, speed: speed);

    // Pause/vinyl always snaps; advancing only snaps large seeks / corrects drift.
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

    return TrackEndBlink(
      deckId: widget.deckId,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth;
          final height = constraints.maxHeight;
          if (width <= 0 || height <= 0) {
            return const SizedBox.expand();
          }
          // Source time on screen: zoom is shared, tempo widens the source span
          // so the musical span holds.
          final visibleSpanMs = visibleSourceMs(speed, zoomMs);
          final pxPerMs = width / visibleSpanMs;
          final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;

          return Listener(
            onPointerSignal: durationMs <= 0
                ? null
                : (event) {
                    if (event is PointerScrollEvent) {
                      ref
                          .read(waveformZoomProvider.notifier)
                          .zoomBy(event.scrollDelta.dy);
                    }
                  },
            onPointerDown: durationMs <= 0
                ? null
                : (e) {
                    if (e.buttons != kPrimaryButton) {
                      return;
                    }
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
                spanMs: visibleSpanMs.toDouble(),
              );
              _playhead.value = (ms / durationMs).clamp(0.0, 1.0);
              _throttledSeek(ms.round());
            },
            onPointerUp: (e) {
              if (_suppressSeek || shiftKeyPressed()) {
                _suppressSeek = false;
                _scrubbing = false;
                return;
              }
              if (!_scrubbing) {
                return;
              }
              final ms = durationMs <= 0
                  ? 0
                  : centerScrubMs(
                      anchorPosMs: _scrubAnchorMs,
                      deltaX: e.localPosition.dx - _scrubAnchorX,
                      width: width,
                      spanMs: visibleSpanMs.toDouble(),
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
            child: ExcludeSemantics(
              child: ClipRect(
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onDoubleTap: durationMs <= 0
                      ? null
                      : () => ref.read(waveformZoomProvider.notifier).reset(),
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      ColoredBox(color: theme.colors.waveformBackground),
                      if (trackId != null && durationMs > 0)
                        _LaneStrip(
                          deckId: widget.deckId,
                          trackId: trackId,
                          durationMs: durationMs,
                          width: width,
                          height: height,
                          pxPerMs: pxPerMs,
                          playhead: _playhead,
                          displayMs: () => _displayMs(durationMs),
                          dpr: dpr,
                          onMenuOpened: () => _suppressSeek = true,
                        ),
                      Align(
                        child: ColoredBox(
                          color: theme.colors.foreground.withValues(alpha: 0.9),
                          child: const SizedBox(
                            width: 1,
                            height: double.infinity,
                          ),
                        ),
                      ),
                      if (slipOn && slipShadowMs != null && trackId != null)
                        AnimatedBuilder(
                          animation: _playhead,
                          builder: (context, _) {
                            final audibleMs = _displayMs(durationMs);
                            final delta = slipShadowMs - audibleMs;
                            if (delta.abs() < 3) {
                              return const SizedBox.shrink();
                            }
                            return Positioned(
                              left: snapPx(
                                width / 2 + delta * pxPerMs - 0.5,
                                dpr,
                              ),
                              top: 0,
                              bottom: 0,
                              width: 1,
                              child: ColoredBox(
                                color: theme.colors.primary.withValues(
                                  alpha: 0.65,
                                ),
                              ),
                            );
                          },
                        ),
                      if (widget.label case final label?)
                        Align(
                          alignment: Alignment.centerLeft,
                          child: Padding(
                            padding: const EdgeInsets.symmetric(horizontal: 10),
                            child: Text(
                              label.toUpperCase(),
                              style: theme.typography.display.xs.copyWith(
                                fontFamily: MixarFonts.spaceGrotesk,
                                color: deckInk(
                                  deckAccentFor(widget.deckId) ??
                                      DeckAccent.neutral,
                                  theme.colors.brightness,
                                ).withValues(alpha: 0.55),
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
              ),
            ),
          );
        },
      ),
    );
  }

  int get _clockMs => widget.nowMs?.call() ?? _seekClock.elapsedMilliseconds;

  void _throttledSeek(int ms) {
    final now = _clockMs;
    final last = _lastSeekMs;
    if (last != null && now - last < 32) {
      return;
    }
    _lastSeekMs = now;
    unawaited(_seek(ms));
  }

  Future<void> _seek(int ms) async {
    try {
      await seekDeck(ref, widget.deckId, ms);
    } catch (e) {
      if (!mounted) {
        return;
      }
      showMixarToast(
        context: context,
        variant: MixarToastVariant.destructive,
        title: Text('$e'),
      );
    }
  }
}

/// The windowed waveform picture, positioned so the playhead sits at center.
///
/// Splitting it out keeps the ~30Hz playhead animation away from the lane's
/// build: only the [Positioned] moves each frame, the strip and its overlays
/// are rebuilt when the window or the strip changes.
class _LaneStrip extends ConsumerWidget {
  const new({
    required this.deckId,
    required this.trackId,
    required this.durationMs,
    required this.width,
    required this.height,
    required this.pxPerMs,
    required this.playhead,
    required this.displayMs,
    required this.dpr,
    this.onMenuOpened,
  });

  final int deckId;
  final String trackId;
  final int durationMs;
  final double width;
  final double height;
  final double pxPerMs;
  final Animation<double> playhead;
  final double Function() displayMs;
  final double dpr;
  final VoidCallback? onMenuOpened;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // The strip owns its window (origin/span) and rebuilds itself when the
    // playhead slides or the zoom changes, keeping the painted window until the
    // new peaks land. Nothing coarse is ever drawn in between.
    final strip = ref.watch(
      waveformStripProvider((trackId, durationMs, deckId)),
    );
    if (strip == null) {
      return const SizedBox.shrink();
    }
    final savedLoops = ref.watch(deckSavedLoopsProvider(deckId));
    final stripOrigin = strip.originMs;
    final stripSpan = strip.spanMs;
    final stripWidth = strip.widthPx;
    // Overlays are recorded at the width they are painted at. Recording them at
    // the (fixed) peak-texture width and scaling that picture would resample
    // 1 px beat lines, and an anti-alias-free hairline drawn at a fractional
    // scale drops out at some zoom levels.
    final displayWidth = (stripSpan * pxPerMs).round();
    final beatGrid = ref.watch(
      stripBeatGridPictureProvider((
        trackId,
        stripOrigin,
        stripSpan,
        displayWidth,
      )),
    );
    final pendingLoopIn = ref.watch(
      stripPendingLoopInPictureProvider((
        deckId,
        stripOrigin,
        stripSpan,
        displayWidth,
      )),
    );
    final hotCues = ref.watch(deckHotCuesProvider(deckId));
    final activeLoop = ref.watch(deckActiveLoopProvider(deckId));
    final accent = context.theme.colors.primary;
    final sections = <WaveformSection>[
      for (final loop in savedLoops)
        WaveformSection(
          type: WaveformSectionType.loop,
          startMs: loop.inMs,
          endMs: loop.outMs,
          color: accent,
        ),
      if (activeLoop != null && activeLoop.active)
        WaveformSection(
          type: WaveformSectionType.loop,
          startMs: activeLoop.inMs,
          endMs: activeLoop.outMs,
          color: accent,
          active: true,
        ),
      for (final cue in hotCues)
        WaveformSection(
          type: WaveformSectionType.cue,
          startMs: cue.positionMs,
          color: hotCueAccent(cue.slot).border.withValues(alpha: 1),
          label: '${cue.slot + 1}',
        ),
    ];

    return AnimatedBuilder(
      animation: playhead,
      builder: (context, child) => Positioned(
        left: snapPx(
          stripTranslateX(
            positionMs: displayMs(),
            originMs: stripOrigin,
            viewportWidth: width,
            pxPerMs: pxPerMs,
          ),
          dpr,
        ),
        top: 0,
        bottom: 0,
        width: displayWidth.toDouble(),
        child: child!,
      ),
      child: LoopBandDeleteRegion(
        deckId: deckId,
        trackId: trackId,
        loops: savedLoops,
        originMs: stripOrigin,
        spanMs: stripSpan,
        width: displayWidth.toDouble(),
        height: height,
        onMenuOpened: onMenuOpened,
        child: RepaintBoundary(
          child: Stack(
            fit: StackFit.expand,
            children: [
              FittedBox(
                fit: BoxFit.fill,
                child: SizedBox(
                  width: stripWidth.toDouble(),
                  height: height,
                  child: _StripWaveform(
                    deckId: deckId,
                    strip: strip,
                    height: height,
                  ),
                ),
              ),
              CustomPaint(
                painter: _StripOverlayPainter(
                  beatGrid: beatGrid,
                  pendingLoopIn: pendingLoopIn,
                ),
              ),
              CustomPaint(
                painter: WaveformSectionPainter(
                  sections: sections,
                  originMs: stripOrigin,
                  spanMs: stripSpan,
                  compact: false,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StripWaveform extends ConsumerWidget {
  const new({required this.deckId, required this.strip, required this.height});

  final int deckId;
  final WaveformStrip strip;
  final double height;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final channel = ref.watch(deckMixerChannelProvider(deckId));
    final gains = waveformGainsFor(
      channel,
      shaped: ref.watch(waveformGainEqShapingProvider),
    );
    return SizedBox(
      width: strip.widthPx.toDouble(),
      height: height,
      child: CustomPaint(
        painter: _WaveformPainter(
          strip: strip,
          background: context.theme.colors.waveformBackground,
          mode: ref.watch(waveformDisplayModeProvider),
          gain: gains.gain,
          bandGain: gains.bandGain,
        ),
        size: Size(strip.widthPx.toDouble(), height),
      ),
    );
  }
}

/// Beat grid, loops and cues, replayed at the width they were recorded at.
class _StripOverlayPainter extends CustomPainter {
  new({required this.beatGrid, required this.pendingLoopIn});

  final Picture? beatGrid;
  final Picture? pendingLoopIn;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.width <= 0 || size.height <= 0) {
      return;
    }
    // These pictures are authored at strip height; scale only vertically so
    // their horizontal placement stays one picture pixel per canvas pixel.
    canvas.save();
    canvas.scale(1, size.height / kWaveformStripHeight);
    for (final picture in [beatGrid, pendingLoopIn]) {
      if (picture != null) {
        canvas.drawPicture(picture);
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(_StripOverlayPainter oldDelegate) =>
      !identical(beatGrid, oldDelegate.beatGrid) ||
      !identical(pendingLoopIn, oldDelegate.pendingLoopIn);
}

class _WaveformPainter extends CustomPainter {
  new({
    required this.strip,
    required this.background,
    required this.mode,
    required this.gain,
    required this.bandGain,
  });

  final WaveformStrip strip;
  final Color background;
  final WaveformDisplayMode mode;
  final double gain;
  final WaveformBandGain bandGain;

  @override
  void paint(Canvas canvas, Size size) {
    if (strip.heightPx <= 0) {
      return;
    }
    final sy = size.height / strip.heightPx;
    final laneHeight = strip.heightPx.toDouble();
    canvas.save();
    canvas.scale(1, sy);
    // Painted live, not replayed from a recorded picture: gains and the
    // display mode are uniforms, so a deck's EQ or a settings flip costs one
    // quad instead of a re-recorded strip.
    WaveformShaderPainter(
      texture: strip.texture,
      background: background,
      mode: mode,
      gain: gain,
      bandGain: bandGain,
    ).paint(canvas, Size(strip.widthPx.toDouble(), laneHeight));
    canvas.restore();
  }

  @override
  bool shouldRepaint(_WaveformPainter oldDelegate) =>
      !identical(strip, oldDelegate.strip) ||
      background != oldDelegate.background ||
      mode != oldDelegate.mode ||
      gain != oldDelegate.gain ||
      bandGain != oldDelegate.bandGain;
}
