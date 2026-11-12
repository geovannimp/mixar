import 'dart:math' as math;

import 'package:flutter/foundation.dart' show immutable, listEquals;
import 'package:flutter/material.dart' show Icons;
import 'package:flutter/painting.dart';
import 'package:flutter/rendering.dart' show CustomPainter;
import 'package:gui_flutter/core/utils/waveform_overlay_geometry.dart';

enum WaveformSectionType { cue, loop }

/// One cue or loop drawn on a waveform.
///
/// Cues have only [startMs]; loops also have [endMs]. Flags use one shared
/// design regardless of type.
@immutable
class WaveformSection {
  const new({
    required this.type,
    required this.startMs,
    required this.color,
    this.endMs,
    this.label,
    this.active = false,
  }) : assert(
         type == WaveformSectionType.cue || endMs != null,
         'loops need an end',
       );

  final WaveformSectionType type;
  final int startMs;
  final int? endMs;
  final Color color;
  final String? label;
  final bool active;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is WaveformSection &&
          type == other.type &&
          startMs == other.startMs &&
          endMs == other.endMs &&
          color == other.color &&
          label == other.label &&
          active == other.active;

  @override
  int get hashCode => Object.hash(type, startMs, endMs, color, label, active);
}

/// Shared cue/loop renderer for every waveform surface.
///
/// Draws at the canvas's current size, so flags keep their shape and pixel size
/// while the waveform is resized instead of being stretched with a cached
/// picture.
class WaveformSectionPainter extends CustomPainter {
  const new({
    required this.sections,
    required this.originMs,
    required this.spanMs,
    required this.compact,
  });

  final List<WaveformSection> sections;
  final int originMs;
  final int spanMs;

  /// Preview strips use a 1px box; the taller lane uses in/out edges only.
  final bool compact;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.width <= 0 || size.height <= 0 || spanMs <= 0) {
      return;
    }
    for (final section in sections) {
      paintWaveformSection(
        canvas: canvas,
        section: section,
        originMs: originMs,
        spanMs: spanMs,
        size: size,
        compact: compact,
      );
    }
  }

  @override
  bool shouldRepaint(WaveformSectionPainter oldDelegate) =>
      !listEquals(sections, oldDelegate.sections) ||
      originMs != oldDelegate.originMs ||
      spanMs != oldDelegate.spanMs ||
      compact != oldDelegate.compact;
}

/// Paints one section. Shared by the waveform painters and the delete overlay.
void paintWaveformSection({
  required Canvas canvas,
  required WaveformSection section,
  required int originMs,
  required int spanMs,
  required Size size,
  required bool compact,
}) {
  if (size.width <= 0 || size.height <= 0 || spanMs <= 0) {
    return;
  }
  final startX = msToX(
    ms: section.startMs,
    originMs: originMs,
    spanMs: spanMs,
    width: size.width,
  );
  if (section.type == WaveformSectionType.loop) {
    final endMs = section.endMs;
    if (endMs == null) {
      return;
    }
    final rect = loopRegionRect(
      inMs: section.startMs,
      outMs: endMs,
      originMs: originMs,
      spanMs: spanMs,
      width: size.width,
      height: size.height,
    );
    if (rect != null) {
      _paintLoopBand(
        canvas: canvas,
        rect: rect,
        size: size,
        color: section.color,
        compact: compact,
        active: section.active,
      );
    }
  } else {
    canvas.drawLine(
      Offset(startX, 0),
      Offset(startX, size.height),
      Paint()
        ..color = section.color
        ..strokeWidth = 2
        ..isAntiAlias = false,
    );
  }
  _paintFlag(
    canvas,
    x: startX,
    section: section,
    laneHeight: size.height,
    width: size.width,
  );
}

void _paintLoopBand({
  required Canvas canvas,
  required Rect rect,
  required Size size,
  required Color color,
  required bool compact,
  required bool active,
}) {
  canvas.drawRect(
    rect,
    Paint()..color = color.withValues(alpha: active ? 0.16 : 0.12),
  );
  if (compact) {
    canvas.drawRect(
      rect,
      Paint()
        ..color = color.withValues(alpha: 0.55)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1
        ..isAntiAlias = false,
    );
  } else {
    final edge = Paint()
      ..color = color.withValues(alpha: 0.85)
      ..strokeWidth = 2
      ..isAntiAlias = false;
    for (final x in [rect.left.roundToDouble(), rect.right.roundToDouble()]) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), edge);
    }
  }
}

void _paintFlag(
  Canvas canvas, {
  required double x,
  required WaveformSection section,
  required double laneHeight,
  required double width,
}) {
  final flagH = laneHeight >= 64 ? 20.0 : (laneHeight * 0.42).clamp(10.0, 14.0);
  final flagW = flagH * 1.35;
  final maxLeft = math.max(0.0, width - flagW);
  final left = (x + 1).clamp(0.0, maxLeft);
  final right = left + flagW;
  final notch = flagW * 0.20;
  final flag = Path()
    ..moveTo(left, 0)
    ..lineTo(right, 0)
    ..lineTo(right - notch, flagH / 2)
    ..lineTo(right, flagH)
    ..lineTo(left, flagH)
    ..close();
  canvas.drawPath(flag, Paint()..color = section.color);

  final glyphColor = section.color.computeLuminance() > 0.5
      ? const Color(0xFF000000)
      : const Color(0xFFFFFFFF);
  final glyphWidth = flagW - notch;
  if (section.type == WaveformSectionType.loop) {
    const icon = Icons.loop;
    final painter = TextPainter(
      text: TextSpan(
        text: String.fromCharCode(icon.codePoint),
        style: TextStyle(
          fontFamily: icon.fontFamily,
          package: icon.fontPackage,
          fontSize: flagH * 0.68,
          height: 1,
          color: glyphColor,
        ),
      ),
      textDirection: TextDirection.ltr,
    )..layout();
    painter.paint(
      canvas,
      Offset(
        left + (glyphWidth - painter.width) / 2,
        (flagH - painter.height) / 2,
      ),
    );
    return;
  }

  final label = section.label;
  if (label == null || label.isEmpty) {
    return;
  }
  final painter = TextPainter(
    text: TextSpan(
      text: label,
      style: TextStyle(
        fontSize: flagH * 0.62,
        fontWeight: FontWeight.w700,
        height: 1,
        color: glyphColor,
      ),
    ),
    textDirection: TextDirection.ltr,
  )..layout();
  painter.paint(
    canvas,
    Offset(
      left + (glyphWidth - painter.width) / 2,
      (flagH - painter.height) / 2,
    ),
  );
}
