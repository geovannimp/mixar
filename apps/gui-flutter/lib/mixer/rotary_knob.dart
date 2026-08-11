import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Matches Tauri `CONTROL_NORM_*` / EQ step mapping (`0.1 / 48`).
const kControlNormMin = 0.0;
const kControlNormMax = 1.0;
const kControlNormCenter = 0.5;
const kControlNormStep = 0.1 / 48.0;

/// Travel arc matches typical DJ pots: -135° … +135° (270° total).
const _angleMinDeg = -135.0;
const _angleSpanDeg = 270.0;
const _angleMaxDeg = _angleMinDeg + _angleSpanDeg;

/// Pixels of vertical drag that span the full value range (Tauri parity).
const _dragPixelsPerRange = 72.0;

enum RotaryKnobSize { md, sm }

double valueToAngle(double value, double min, double max) {
  final t = (value - min) / (max - min);
  return t * _angleSpanDeg + _angleMinDeg;
}

({double from, double to}) valueFillAngles(
  double value,
  double min,
  double max, {
  double? center,
}) {
  final valueAngle = valueToAngle(value, min, max);
  final detent = center ?? (min < 0 && max > 0 ? 0.0 : null);
  if (detent == null) {
    return (from: _angleMinDeg, to: valueAngle);
  }
  final zeroAngle = valueToAngle(detent, min, max);
  if (valueAngle >= zeroAngle) {
    return (from: zeroAngle, to: valueAngle);
  }
  return (from: valueAngle, to: zeroAngle);
}

double snapToStep(double value, double step) {
  if (step <= 0) {
    return value;
  }
  final snapped = (value / step).round() * step;
  return snapped == 0 ? 0.0 : snapped;
}

double valueFromVerticalDrag({
  required double startValue,
  required double startY,
  required double clientY,
  required double min,
  required double max,
  required double step,
}) {
  final range = max - min;
  final deltaY = startY - clientY;
  final raw = startValue + (deltaY / _dragPixelsPerRange) * range;
  final snapped = snapToStep(raw, step);
  return snapped.clamp(min, max);
}

/// DJ-style rotary control: 270° travel arc, vertical drag, center detent fill.
class RotaryKnob extends StatefulWidget {
  const RotaryKnob({
    required this.label,
    required this.value,
    required this.onValueChange,
    this.min = kControlNormMin,
    this.max = kControlNormMax,
    this.step = kControlNormStep,
    this.center,
    this.disabled = false,
    this.size = RotaryKnobSize.md,
    this.accentColor,
    this.ringColor,
    super.key,
  });

  final String label;
  final double value;
  final ValueChanged<double> onValueChange;
  final double min;
  final double max;
  final double step;
  final double? center;
  final bool disabled;
  final RotaryKnobSize size;
  final Color? accentColor;
  final Color? ringColor;

  @override
  State<RotaryKnob> createState() => _RotaryKnobState();
}

class _RotaryKnobState extends State<RotaryKnob> {
  double? _startY;
  double? _startValue;

  double get _dialExtent => widget.size == RotaryKnobSize.sm ? 24.0 : 36.0;

  double get _strokeWidth => widget.size == RotaryKnobSize.sm ? 2.4 : 3.6;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final snapped = snapToStep(widget.value, widget.step);
    final angle = valueToAngle(snapped, widget.min, widget.max);
    final fill = valueFillAngles(
      snapped,
      widget.min,
      widget.max,
      center: widget.center,
    );
    final labelColor = widget.accentColor ?? theme.colors.mutedForeground;
    final ringColor = widget.ringColor ?? theme.colors.primary;
    final opacity = widget.disabled ? 0.45 : 1.0;

    return Opacity(
      opacity: opacity,
      child: Column(
        mainAxisSize: .min,
        children: [
          Text(
            widget.label.toUpperCase(),
            style: theme.typography.body.xs.copyWith(
              fontWeight: .w600,
              letterSpacing: 0.6,
              fontSize: widget.size == RotaryKnobSize.sm ? 7 : 8,
              color: labelColor,
              height: 1,
            ),
          ),
          const SizedBox(height: 2),
          Listener(
            behavior: .opaque,
            onPointerDown: widget.disabled
                ? null
                : (event) {
                    _startY = event.position.dy;
                    _startValue = widget.value;
                  },
            onPointerMove: widget.disabled
                ? null
                : (event) {
                    final startY = _startY;
                    final startValue = _startValue;
                    if (startY == null || startValue == null) {
                      return;
                    }
                    widget.onValueChange(
                      valueFromVerticalDrag(
                        startValue: startValue,
                        startY: startY,
                        clientY: event.position.dy,
                        min: widget.min,
                        max: widget.max,
                        step: widget.step,
                      ),
                    );
                  },
            onPointerUp: (_) {
              _startY = null;
              _startValue = null;
            },
            onPointerCancel: (_) {
              _startY = null;
              _startValue = null;
            },
            child: SizedBox(
              width: _dialExtent,
              height: _dialExtent,
              child: CustomPaint(
                painter: _RotaryKnobPainter(
                  fillFromDeg: fill.from,
                  fillToDeg: fill.to,
                  angleDeg: angle,
                  strokeWidth: _strokeWidth,
                  trackColor: theme.colors.border.withValues(alpha: 0.35),
                  fillColor: ringColor,
                  faceColor: theme.colors.secondary,
                  tickColor: theme.colors.foreground,
                  size: widget.size,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _RotaryKnobPainter extends CustomPainter {
  _RotaryKnobPainter({
    required this.fillFromDeg,
    required this.fillToDeg,
    required this.angleDeg,
    required this.strokeWidth,
    required this.trackColor,
    required this.fillColor,
    required this.faceColor,
    required this.tickColor,
    required this.size,
  });

  final double fillFromDeg;
  final double fillToDeg;
  final double angleDeg;
  final double strokeWidth;
  final Color trackColor;
  final Color fillColor;
  final Color faceColor;
  final Color tickColor;
  final RotaryKnobSize size;

  @override
  void paint(Canvas canvas, Size size) {
    final side = math.min(size.width, size.height);
    final scale = side / 100;
    canvas.save();
    canvas.scale(scale);

    final stroke = strokeWidth / scale;
    final radius = 50 - stroke / 2;
    final trackPaint = Paint()
      ..color = trackColor
      ..style = .stroke
      ..strokeWidth = stroke
      ..strokeCap = .butt
      ..isAntiAlias = true;
    final fillPaint = Paint()
      ..color = fillColor
      ..style = .stroke
      ..strokeWidth = stroke
      ..strokeCap = .butt
      ..isAntiAlias = true;

    final track = _clockwiseArcPath(_angleMinDeg, _angleMaxDeg, radius);
    if (track != null) {
      canvas.drawPath(track, trackPaint);
    }
    final valuePath = _clockwiseArcPath(fillFromDeg, fillToDeg, radius);
    if (valuePath != null) {
      canvas.drawPath(valuePath, fillPaint);
    }

    final faceInset = this.size == RotaryKnobSize.sm ? 12.5 : 16.7;
    final faceRadius = 50 - faceInset;
    canvas.drawCircle(
      const Offset(50, 50),
      faceRadius,
      Paint()
        ..color = faceColor
        ..style = .fill
        ..isAntiAlias = true,
    );

    canvas.save();
    canvas.translate(50, 50);
    canvas.rotate(angleDeg * math.pi / 180);
    final tickH = this.size == RotaryKnobSize.sm
        ? faceRadius * 0.64
        : faceRadius * 0.68;
    final tickW = this.size == RotaryKnobSize.sm ? 1.0 : 2.0;
    canvas.drawRRect(
      RRect.fromRectAndRadius(
        Rect.fromCenter(
          center: Offset(0, -tickH / 2),
          width: tickW,
          height: tickH,
        ),
        const Radius.circular(1),
      ),
      Paint()
        ..color = tickColor
        ..style = .fill
        ..isAntiAlias = true,
    );
    canvas.restore();
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _RotaryKnobPainter oldDelegate) {
    return fillFromDeg != oldDelegate.fillFromDeg ||
        fillToDeg != oldDelegate.fillToDeg ||
        angleDeg != oldDelegate.angleDeg ||
        strokeWidth != oldDelegate.strokeWidth ||
        trackColor != oldDelegate.trackColor ||
        fillColor != oldDelegate.fillColor ||
        faceColor != oldDelegate.faceColor ||
        tickColor != oldDelegate.tickColor ||
        size != oldDelegate.size;
  }
}

/// CSS angle: 0° = up, clockwise → point in a 100×100 square.
Offset _polarToSvg(double angleDeg, double radius, [double center = 50]) {
  final rad = angleDeg * math.pi / 180;
  return Offset(
    center + radius * math.sin(rad),
    center - radius * math.cos(rad),
  );
}

Path? _clockwiseArcPath(double fromDeg, double toDeg, double radius) {
  final span = toDeg - fromDeg;
  if (span <= 0.05) {
    return null;
  }
  final start = _polarToSvg(fromDeg, radius);
  final end = _polarToSvg(toDeg, radius);
  final largeArc = span > 180;
  return Path()
    ..moveTo(start.dx, start.dy)
    ..arcToPoint(
      end,
      radius: Radius.circular(radius),
      largeArc: largeArc,
      clockwise: true,
    );
}
