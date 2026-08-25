import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/jog_ticks.dart';

/// Interactive jog platter — `jog_touch` / `jog_turn` (Tauri `JogPlatter`).
class DeckJogHost extends ConsumerWidget {
  const DeckJogHost({
    required this.deckId,
    required this.hasTrack,
    required this.accent,
    this.disabled = false,
    super.key,
  });

  final int deckId;
  final bool hasTrack;
  final FaderAccent accent;
  final bool disabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return JogPlatter(
      accent: accent,
      playing: ref.watch(deckPlayingProvider(deckId)),
      bpm: ref.watch(deckBpmProvider(deckId)),
      hasTrack: hasTrack,
      enabled: !disabled && hasTrack,
      jogTouching: ref.watch(deckJogTouchingProvider(deckId)),
      positionMs: ref.watch(deckPositionMsProvider(deckId)),
      durationMs: ref.watch(deckDurationMsProvider(deckId)),
      speed: ref.watch(deckSpeedRatioProvider(deckId)),
      onJogTouch: (touching) {
        unawaited(_engineCmd(context, () => jogTouch(ref, deckId, touching)));
      },
      onJogTurn: (delta) {
        unawaited(_engineCmd(context, () => jogTurn(ref, deckId, delta)));
      },
    );
  }
}

Future<void> _engineCmd(
  BuildContext context,
  Future<void> Function() fn,
) async {
  try {
    await fn();
  } catch (e) {
    if (!context.mounted) {
      return;
    }
    showFToast(context: context, variant: .destructive, title: Text('$e'));
  }
}

class JogPlatter extends StatefulWidget {
  const JogPlatter({
    required this.accent,
    required this.playing,
    required this.hasTrack,
    this.bpm,
    this.enabled = false,
    this.jogTouching = false,
    this.positionMs = 0,
    this.durationMs,
    this.speed = 1,
    this.onJogTouch,
    this.onJogTurn,
    super.key,
  });

  final FaderAccent accent;
  final bool playing;
  final double? bpm;
  final bool hasTrack;
  final bool enabled;
  final bool jogTouching;
  final int positionMs;
  final int? durationMs;
  final double speed;
  final ValueChanged<bool>? onJogTouch;
  final ValueChanged<int>? onJogTurn;

  @override
  State<JogPlatter> createState() => _JogPlatterState();
}

class _JogPlatterState extends State<JogPlatter> {
  var _rotationDeg = 0.0;
  var _lastPositionMs = 0;
  var _trackerInitialized = false;
  var _dragging = false;
  double? _lastAngleDeg;

  bool get _interactive => widget.enabled && widget.hasTrack;

  @override
  void didUpdateWidget(covariant JogPlatter oldWidget) {
    super.didUpdateWidget(oldWidget);
    final wasInteractive = oldWidget.enabled && oldWidget.hasTrack;
    if (wasInteractive && !_interactive && _dragging) {
      _dragging = false;
      _lastAngleDeg = null;
      widget.onJogTouch?.call(false);
    }
    _syncRotationFromPlayhead();
  }

  void _syncRotationFromPlayhead() {
    if (!widget.hasTrack) {
      _trackerInitialized = false;
      _lastPositionMs = 0;
      _rotationDeg = 0;
      return;
    }
    final bpm = widget.bpm != null && widget.bpm! > 0 ? widget.bpm! : 120.0;
    final cycle = barCycleDurationMs(bpm);
    if (cycle == null) {
      return;
    }
    if (!_trackerInitialized) {
      _trackerInitialized = true;
      _lastPositionMs = widget.positionMs;
      _rotationDeg = barCycleRotationDeg(widget.positionMs, bpm);
      return;
    }
    if (_dragging || widget.jogTouching) {
      _lastPositionMs = widget.positionMs;
      return;
    }
    final delta = widget.positionMs - _lastPositionMs;
    _lastPositionMs = widget.positionMs;
    final seekThreshold = math.max(200, cycle * 0.15);
    if (delta.abs() > seekThreshold) {
      _rotationDeg = barCycleRotationDeg(widget.positionMs, bpm);
    } else {
      _rotationDeg += (delta / cycle) * 360;
    }
  }

  void _pointerDown(PointerDownEvent event, Size size) {
    if (!_interactive || event.buttons != 1) {
      return;
    }
    _dragging = true;
    _lastAngleDeg = pointerAngleDeg(
      event.localPosition.dx,
      event.localPosition.dy,
      size.width,
      size.height,
    );
    widget.onJogTouch?.call(true);
  }

  void _pointerMove(PointerMoveEvent event, Size size) {
    if (!_dragging || !_interactive) {
      return;
    }
    final angle = pointerAngleDeg(
      event.localPosition.dx,
      event.localPosition.dy,
      size.width,
      size.height,
    );
    final prev = _lastAngleDeg;
    _lastAngleDeg = angle;
    if (prev == null) {
      return;
    }
    var deltaDeg = angle - prev;
    if (deltaDeg > 180) {
      deltaDeg -= 360;
    } else if (deltaDeg < -180) {
      deltaDeg += 360;
    }
    setState(() => _rotationDeg += deltaDeg);
    final ticks = degreesToJogTicks(deltaDeg);
    if (ticks != 0) {
      widget.onJogTurn?.call(ticks);
    }
  }

  void _pointerUp() {
    if (!_dragging) {
      return;
    }
    _dragging = false;
    _lastAngleDeg = null;
    widget.onJogTouch?.call(false);
  }

  @override
  void dispose() {
    if (_dragging) {
      widget.onJogTouch?.call(false);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = FaderColors.forAccent(widget.accent);
    final progress =
        widget.hasTrack && widget.durationMs != null && widget.durationMs! > 0
        ? (widget.positionMs / widget.durationMs!).clamp(0.0, 1.0)
        : 0.0;

    return Padding(
      padding: const .symmetric(vertical: 12),
      child: Semantics(
        label: 'Jog wheel',
        child: AspectRatio(
          aspectRatio: 1,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final size = Size(constraints.maxWidth, constraints.maxHeight);
              return Listener(
                behavior: HitTestBehavior.opaque,
                onPointerDown: _interactive
                    ? (e) => _pointerDown(e, size)
                    : null,
                onPointerMove: _interactive
                    ? (e) => _pointerMove(e, size)
                    : null,
                onPointerUp: _interactive ? (_) => _pointerUp() : null,
                onPointerCancel: _interactive ? (_) => _pointerUp() : null,
                child: CustomPaint(
                  painter: _JogPainter(
                    border: theme.colors.border,
                    fill: theme.colors.background.withValues(alpha: 0.9),
                    accent: colors.grip,
                    progress: progress,
                    rotationDeg: _rotationDeg,
                    hasTrack: widget.hasTrack,
                    touching: widget.jogTouching || _dragging,
                  ),
                ),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _JogPainter extends CustomPainter {
  _JogPainter({
    required this.border,
    required this.fill,
    required this.accent,
    required this.progress,
    required this.rotationDeg,
    required this.hasTrack,
    required this.touching,
  });

  final Color border;
  final Color fill;
  final Color accent;
  final double progress;
  final double rotationDeg;
  final bool hasTrack;
  final bool touching;

  @override
  void paint(Canvas canvas, Size size) {
    final c = Offset(size.width / 2, size.height / 2);
    final r = size.shortestSide / 2;
    canvas.drawCircle(c, r - 1.5, Paint()..color = fill);
    canvas.drawCircle(
      c,
      r - 1.5,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = touching ? 3 : 2
        ..color = touching ? accent.withValues(alpha: 0.7) : border,
    );

    final ringR = r * 0.92;
    final ring = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round
      ..color = accent.withValues(alpha: 0.12);
    canvas.drawCircle(c, ringR, ring);
    if (hasTrack && progress > 0) {
      final sweep = progress * 2 * math.pi;
      canvas.drawArc(
        Rect.fromCircle(center: c, radius: ringR),
        -math.pi / 2,
        sweep,
        false,
        ring..color = accent.withValues(alpha: 0.55),
      );
    }

    canvas.drawCircle(
      c,
      r * 0.72,
      Paint()
        ..style = PaintingStyle.stroke
        ..color = border.withValues(alpha: 0.6),
    );

    if (hasTrack) {
      canvas.save();
      canvas.translate(c.dx, c.dy);
      canvas.rotate(rotationDeg * math.pi / 180);
      canvas.drawLine(
        Offset.zero,
        Offset(0, -r * 0.38),
        Paint()
          ..color = accent
          ..strokeWidth = 2
          ..strokeCap = StrokeCap.round,
      );
      canvas.restore();
    }

    canvas.drawCircle(c, 3.5, Paint()..color = hasTrack ? accent : border);
  }

  @override
  bool shouldRepaint(covariant _JogPainter old) =>
      old.progress != progress ||
      old.rotationDeg != rotationDeg ||
      old.hasTrack != hasTrack ||
      old.touching != touching ||
      old.accent != accent ||
      old.border != border ||
      old.fill != fill;
}
