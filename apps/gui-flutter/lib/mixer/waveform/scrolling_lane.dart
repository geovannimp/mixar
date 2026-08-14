import 'dart:async';

import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/mixer/waveform/layout.dart';
import 'package:gui_flutter/mixer/waveform/peaks.dart';
import 'package:gui_flutter/mixer/waveform/waveform_picture.dart';
import 'package:gui_flutter/mixer/waveform/waveform_providers.dart';

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
  var _displayMs = 0.0;
  var _engineMs = 0;
  var _engineElapsed = Duration.zero;
  var _elapsed = Duration.zero;
  var _originMs = 0.0;
  var _scrubbing = false;
  var _scrubAnchorX = 0.0;
  var _scrubAnchorMs = 0.0;
  var _l1Gen = 0;
  var _l1TrackId = '';
  var _l1InFlight = false;
  String? _l1FailKey;
  DetailWindow? _detail;
  final _seekClock = Stopwatch();

  @override
  void initState() {
    super.initState();
    _ticker = createTicker(_onTick);
  }

  @override
  void dispose() {
    _ticker.dispose();
    super.dispose();
  }

  void _onTick(Duration elapsed) {
    _elapsed = elapsed;
    if (_scrubbing || !mounted) {
      return;
    }
    final playing = ref.read(deckPlayingProvider(widget.deckId));
    if (!playing) {
      return;
    }
    final speed = ref.read(deckSpeedRatioProvider(widget.deckId));
    final dt = (elapsed - _engineElapsed).inMicroseconds / 1e3;
    final next = _engineMs + dt * speed;
    if ((next - _displayMs).abs() < 0.5) {
      return;
    }
    setState(() {
      _displayMs = next;
      _rebaseOrigin(visibleSourceMs(speed));
    });
  }

  void _rebaseOrigin(int visibleMs) {
    final spanMs = visibleMs * (1 + 2 * kWaveformBufferRatio);
    final halfBuf = spanMs / 2;
    if ((_displayMs - _originMs - halfBuf).abs() >
        visibleMs * kWaveformRefreshMargin) {
      _originMs = _displayMs - halfBuf;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final playing = ref.watch(deckPlayingProvider(widget.deckId));
    if (playing && !_ticker.isTicking) {
      _elapsed = Duration.zero;
      _engineElapsed = Duration.zero;
      _ticker.start();
    } else if (!playing && _ticker.isTicking) {
      _ticker.stop();
    }
    final trackId = ref.watch(deckTrackIdProvider(widget.deckId));
    final durationMs = ref.watch(deckDurationMsProvider(widget.deckId)) ?? 0;
    final speed = ref.watch(deckSpeedRatioProvider(widget.deckId));
    final peaks = trackId == null
        ? const <SpectralPeak>[]
        : (ref.watch(waveformOverviewProvider(trackId)).value ??
              const <SpectralPeak>[]);
    final grid = trackId == null
        ? null
        : ref.watch(beatGridProvider(trackId)).value;

    ref.listen(deckPositionMsProvider(widget.deckId), (prev, next) {
      _engineMs = next;
      _engineElapsed = _elapsed;
      if (_scrubbing) {
        return;
      }
      final playing = ref.read(deckPlayingProvider(widget.deckId));
      if (!playing || (_displayMs - next).abs() >= 180) {
        setState(() {
          _displayMs = next.toDouble();
          _rebaseOrigin(visibleSourceMs(speed));
        });
      }
    });

    if (trackId != _l1TrackId) {
      _l1TrackId = trackId ?? '';
      _detail = null;
      _l1FailKey = null;
      _l1Gen++;
    }

    final visibleMs = visibleSourceMs(speed);
    final spanMs = visibleMs * (1 + 2 * kWaveformBufferRatio);
    _rebaseOrigin(visibleMs);

    return LayoutBuilder(
      builder: (context, constraints) {
        final width = constraints.maxWidth;
        final height = constraints.maxHeight;
        if (width <= 0 || height <= 0) {
          return const SizedBox.expand();
        }

        _maybeFetchL1(trackId, visibleMs, durationMs, width);

        final pxPerMs = width / visibleMs;
        final bufW = width * (1 + 2 * kWaveformBufferRatio);
        final dx = playheadDx(
          positionMs: _displayMs,
          originMs: _originMs,
          width: width,
          pxPerMs: pxPerMs,
        );

        final marks = grid?.bpm == null
            ? const <BeatMark>[]
            : beatGridXs(
                bpm: grid!.bpm!,
                firstBeatSecs: grid.beats.isEmpty ? 0 : grid.beats.first,
                startMs: _displayMs.round() - visibleMs ~/ 2,
                endMs: _displayMs.round() + visibleMs ~/ 2,
                positionMs: _displayMs.round(),
                width: width,
                visibleMs: visibleMs,
              );

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
              spanMs: visibleMs.toDouble(),
            );
            setState(() => _displayMs = ms);
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
              spanMs: visibleMs.toDouble(),
            ).round();
            _scrubbing = false;
            unawaited(_seek(ms));
          },
          onPointerCancel: (_) => _scrubbing = false,
          child: ClipRect(
            child: Stack(
              fit: StackFit.expand,
              children: [
                Positioned(
                  left: dx,
                  top: 0,
                  width: bufW,
                  height: height,
                  child: RepaintBoundary(
                    child: CustomPaint(
                      painter: WaveformBarPainter(
                        overview: peaks,
                        detail: _detail,
                        durationMs: durationMs,
                        originMs: _originMs,
                        spanMs: spanMs.toDouble(),
                      ),
                    ),
                  ),
                ),
                CustomPaint(painter: _BeatGridPainter(marks: marks)),
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

  void _maybeFetchL1(
    String? trackId,
    int visibleMs,
    int durationMs,
    double width,
  ) {
    if (trackId == null || durationMs <= 0 || !mounted || _l1InFlight) {
      return;
    }
    if (_detail != null &&
        _displayMs >= _detail!.startMs + visibleMs * kWaveformRefreshMargin &&
        _displayMs <= _detail!.endMs - visibleMs * kWaveformRefreshMargin) {
      return;
    }
    final range = l1Range(
      positionMs: _displayMs.round(),
      visibleMs: visibleMs,
      durationMs: durationMs,
    );
    if (range.endMs <= range.startMs) {
      return;
    }
    final buckets = (width * 3).round().clamp(16, 16384);
    final failKey = '$trackId:${range.startMs}:${range.endMs}:$buckets';
    if (_l1FailKey == failKey) {
      return;
    }
    final gen = ++_l1Gen;
    _l1InFlight = true;
    unawaited(
      _loadL1(
        trackId,
        range.startMs,
        range.endMs,
        buckets,
        gen,
        failKey,
      ).whenComplete(() {
        _l1InFlight = false;
      }),
    );
  }

  Future<void> _loadL1(
    String trackId,
    int start,
    int end,
    int buckets,
    int gen,
    String failKey,
  ) async {
    try {
      final lib = await ref.read(libraryTransportProvider.future);
      final packed = await lib.getWaveformWindow(
        trackId: trackId,
        startMs: start,
        endMs: end,
        buckets: buckets,
      );
      if (!mounted || gen != _l1Gen) {
        return;
      }
      _l1FailKey = null;
      setState(() {
        _detail = DetailWindow(
          peaks: decodeRgbPeaks(packed.rgb),
          startMs: packed.startMs,
          endMs: packed.endMs,
        );
      });
    } catch (e, st) {
      _l1FailKey = failKey;
      FlutterError.reportError(FlutterErrorDetails(exception: e, stack: st));
    }
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

class _BeatGridPainter extends CustomPainter {
  _BeatGridPainter({required this.marks});

  final List<BeatMark> marks;

  @override
  void paint(Canvas canvas, Size size) {
    for (final mark in marks) {
      final paint = Paint()
        ..color = mark.isBar
            ? const Color.fromARGB(80, 200, 205, 215)
            : const Color.fromARGB(55, 170, 175, 185)
        ..strokeWidth = 1;
      canvas.drawLine(Offset(mark.x, 0), Offset(mark.x, size.height), paint);
    }
  }

  @override
  bool shouldRepaint(_BeatGridPainter oldDelegate) =>
      !identical(marks, oldDelegate.marks);
}
