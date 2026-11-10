import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/core/theme/mixar_palette.dart';
import 'package:gui_flutter/core/utils/jog_ticks.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Test-only: read [_JogPlatterState.debugRotationDeg].
@visibleForTesting
double debugReadJogPlatterRotation(State state) {
  if (state is! _JogPlatterState) {
    throw StateError('expected JogPlatter state');
  }
  return state.debugRotationDeg;
}

/// Test-only: latch local-drag freeze without a private cast.
@visibleForTesting
void debugSetJogPlatterDragging(State state, {required bool dragging}) {
  if (state is! _JogPlatterState) {
    throw StateError('expected JogPlatter state');
  }
  state.debugSetDragging(dragging);
}

/// Test-only: read [_JogPlatterState.debugDragging].
@visibleForTesting
bool debugReadJogPlatterDragging(State state) {
  if (state is! _JogPlatterState) {
    throw StateError('expected JogPlatter state');
  }
  return state.debugDragging;
}

class JogPlatter extends StatefulWidget {
  const new({
    required this.accent,
    required this.playing,
    required this.hasTrack,
    this.bpm,
    this.artwork,
    this.enabled = false,
    this.jogTouching = false,
    this.positionMs = 0,
    this.durationMs,
    this.speed = 1,
    this.onJogTouch,
    this.onJogTurn,
    super.key,
  });

  final DeckAccent accent;
  final bool playing;
  final double? bpm;
  final bool hasTrack;

  /// Cover art for the centre label (the loaded track's artwork bytes). Null
  /// falls back to a plain label disc.
  final Uint8List? artwork;
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

  @visibleForTesting
  double get debugRotationDeg => _rotationDeg;

  @visibleForTesting
  bool get debugDragging => _dragging;

  /// Test-only: latch local-drag freeze without synthesizing pointer events.
  @visibleForTesting
  void debugSetDragging(bool value) {
    if (!mounted) {
      return;
    }
    setState(() {
      _dragging = value;
      if (value) {
        _lastPositionMs = widget.positionMs;
      } else {
        _lastAngleDeg = null;
      }
    });
  }

  bool get _interactive => widget.enabled && widget.hasTrack;

  @override
  void initState() {
    super.initState();
    _syncRotationFromPlayhead();
  }

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
    // Visual spin uses the vinyl timebase (one rev ↔ one hardware/UI circle),
    // not the 4-bar phrase cycle — otherwise playback lags the platter.
    final revMs = vinylRevolutionMs();
    // Only freeze playhead sync during a local pointer drag. Hardware
    // `jogTouching` must still rotate the disc from positionMs (Mixxx-like).
    // Checked before init so a first update mid-drag cannot snap the disc.
    if (_dragging) {
      _trackerInitialized = true;
      _lastPositionMs = widget.positionMs;
      return;
    }
    if (!_trackerInitialized) {
      _trackerInitialized = true;
      _lastPositionMs = widget.positionMs;
      _rotationDeg = vinylRotationDeg(widget.positionMs);
      return;
    }
    final delta = widget.positionMs - _lastPositionMs;
    _lastPositionMs = widget.positionMs;
    final seekThreshold = math.max(200, revMs * 0.15);
    if (delta.abs() > seekThreshold) {
      _rotationDeg = vinylRotationDeg(widget.positionMs);
    } else {
      _rotationDeg += (delta / revMs) * 360;
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
    final accent = deckInk(widget.accent, theme.colors.brightness);
    // Rim ring is neutral on purpose: the plate is the instrument, the deck's
    // colour only marks the elapsed part of the ring and the touch state.
    final ring = theme.colors.brightness == Brightness.dark
        ? MixarZinc.s700
        : MixarZinc.s300;
    final progress =
        widget.hasTrack && widget.durationMs != null && widget.durationMs! > 0
        ? (widget.positionMs / widget.durationMs!).clamp(0.0, 1.0)
        : 0.0;

    return Padding(
      padding: const .symmetric(vertical: 8),
      child: Semantics(
        label: 'Jog wheel',
        child: AspectRatio(
          aspectRatio: 1,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final size = Size(constraints.maxWidth, constraints.maxHeight);
              // Vinyl label: about a third of the disc, turning with it.
              final labelSide = size.shortestSide * 0.36;
              return MouseRegion(
                cursor: _interactive
                    ? (_dragging
                          ? SystemMouseCursors.grabbing
                          : SystemMouseCursors.grab)
                    : MouseCursor.defer,
                child: Listener(
                  behavior: HitTestBehavior.opaque,
                  onPointerDown: _interactive
                      ? (e) => _pointerDown(e, size)
                      : null,
                  onPointerMove: _interactive
                      ? (e) => _pointerMove(e, size)
                      : null,
                  onPointerUp: _interactive ? (_) => _pointerUp() : null,
                  onPointerCancel: _interactive ? (_) => _pointerUp() : null,
                  child: Stack(
                    fit: StackFit.expand,
                    children: [
                      RepaintBoundary(
                        child: CustomPaint(
                          painter: _JogPainter(
                            border: theme.colors.border,
                            plate: theme.colors.card,
                            ring: ring,
                            accent: accent,
                            marker: theme.colors.foreground,
                            progress: progress,
                            rotationDeg: _rotationDeg,
                            hasTrack: widget.hasTrack,
                            touching: widget.jogTouching || _dragging,
                          ),
                        ),
                      ),
                      Center(
                        child: SizedBox.square(
                          dimension: labelSide,
                          child: RepaintBoundary(
                            child: _JogLabel(
                              artwork: widget.artwork,
                              fill: theme.colors.card,
                              border: theme.colors.border,
                              accent: widget.hasTrack ? accent : null,
                              rotationDeg: _rotationDeg,
                            ),
                          ),
                        ),
                      ),
                    ],
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

/// Draws the deck's platter: a flat dark disc, a thick neutral rim ring whose
/// elapsed part is painted in the deck's colour, and a barred position marker.
/// The centre label (track artwork) sits on top as a widget, so it turns with
/// the record instead of being repainted here.
class _JogPainter extends CustomPainter {
  const new({
    required this.border,
    required this.plate,
    required this.ring,
    required this.accent,
    required this.marker,
    required this.progress,
    required this.rotationDeg,
    required this.hasTrack,
    required this.touching,
  });

  final Color border;

  /// Disc body over the panel floor.
  final Color plate;

  /// Neutral band that reads as the platter's rim.
  final Color ring;
  final Color accent;

  /// Ink for the position marker (`foreground`): the one near-white element on
  /// the plate, so the angle reads first.
  final Color marker;
  final double progress;
  final double rotationDeg;
  final bool hasTrack;
  final bool touching;

  static const double _top = -math.pi / 2;

  /// Rim ring: centre line and thickness, as fractions of the disc radius.
  static const double _ringRadius = 0.90;
  static const double _ringWidth = 0.12;

  /// Below this sweep the elapsed arc is not worth a pixel: drawing it would
  /// only stack stroke ends at 12 o'clock.
  static const double _minSweep = 0.01;

  /// Marker span, as fractions of the radius: it leaves the label edge and
  /// runs into the rim ring, which paints over its tip — so the needle reads
  /// as passing behind the ring instead of stopping short of it. It ends
  /// inside the band (not past it) so no tip pokes out at the disc edge.
  static const double _markerStart = 0.24;
  static const double _markerEnd = 0.95;

  @override
  void paint(Canvas canvas, Size size) {
    final c = Offset(size.width / 2, size.height / 2);
    final discR = size.shortestSide / 2 - 1.5;
    if (discR <= 0) {
      return;
    }

    // Flat plate: depth comes from the rim ring, not a gradient, so the wheel
    // keeps the deck's flat-surface language.
    canvas.drawCircle(c, discR, Paint()..color = plate);

    // Marker first, so the rim ring and the elapsed arc paint over it: the
    // needle reads as passing behind the ring instead of being clipped by it.
    _paintMarker(canvas, c, discR);

    // Rim ring, and the elapsed part of it in the deck's colour.
    final ringR = discR * _ringRadius;
    final ringW = discR * _ringWidth;
    canvas.drawCircle(
      c,
      ringR,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = ringW
        ..color = ring,
    );

    // Square ends: a round cap turns a near-zero sweep into a large dot at
    // 12 o'clock, which reads as a stuck control at the very start of a track.
    final sweep = hasTrack ? progress * 2 * math.pi : 0.0;
    if (sweep > _minSweep) {
      canvas.drawArc(
        Rect.fromCircle(center: c, radius: ringR),
        _top,
        sweep,
        false,
        Paint()
          ..style = PaintingStyle.stroke
          ..strokeWidth = ringW
          ..color = accent,
      );
    }

    // Bezel hairline: the disc edge, and the tell that a hand is on the wheel.
    canvas.drawCircle(
      c,
      discR,
      Paint()
        ..style = PaintingStyle.stroke
        ..strokeWidth = touching ? 2.5 : 1.5
        ..color = touching ? accent : border,
    );
  }

  /// Position marker: a bar rather than a hairline, so the angle is readable
  /// across a booth without adding weight to the rest of the wheel.
  void _paintMarker(Canvas canvas, Offset c, double radius) {
    if (!hasTrack) {
      return;
    }
    canvas
      ..save()
      ..translate(c.dx, c.dy)
      ..rotate(rotationDeg * math.pi / 180)
      ..drawLine(
        Offset(0, -radius * _markerStart),
        Offset(0, -radius * _markerEnd),
        Paint()
          ..color = marker
          ..strokeWidth = math.max<double>(4, radius * 0.06),
      )
      ..restore();
  }

  @override
  bool shouldRepaint(covariant _JogPainter old) =>
      old.progress != progress ||
      old.rotationDeg != rotationDeg ||
      old.hasTrack != hasTrack ||
      old.touching != touching ||
      old.accent != accent ||
      old.border != border ||
      old.plate != plate ||
      old.ring != ring ||
      old.marker != marker;
}

/// Centre label of the record: the loaded track's artwork clipped to a circle
/// and turning with the platter. Without artwork it falls back to a plain label
/// disc with one pin, so the centre never reads as empty.
class _JogLabel extends StatelessWidget {
  const new({
    required this.artwork,
    required this.fill,
    required this.border,
    required this.accent,
    required this.rotationDeg,
  });

  final Uint8List? artwork;
  final Color fill;
  final Color border;

  /// Null on an empty deck: the pin stays hairline-quiet.
  final Color? accent;
  final double rotationDeg;

  @override
  Widget build(BuildContext context) {
    final bytes = artwork;
    return DecoratedBox(
      decoration: BoxDecoration(shape: BoxShape.circle, color: fill),
      child: ClipOval(
        child: bytes == null
            ? Center(
                child: SizedBox.square(
                  dimension: 8,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: accent ?? border,
                    ),
                  ),
                ),
              )
            : Transform.rotate(
                angle: rotationDeg * math.pi / 180,
                child: Image.memory(
                  bytes,
                  fit: BoxFit.cover,
                  gaplessPlayback: true,
                ),
              ),
      ),
    );
  }
}
