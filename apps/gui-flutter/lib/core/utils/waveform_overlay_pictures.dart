import 'dart:math' as math;
import 'dart:ui';

import 'package:flutter/material.dart' show Icons;
import 'package:flutter/painting.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/utils/waveform_beat_grid.dart';
import 'package:gui_flutter/core/utils/waveform_overlay_geometry.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show SavedLoop;
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';

// Saved-loop fill stays neutral so it reads on both the near-black dark lane
// and the light one; the edges reuse the theme primary accent so a remembered
// region is recognizable without competing with the active loop.
const _fallbackLoopAccent = Color.fromRGBO(52, 211, 153, 1);
const _beatGridBar = Color.fromRGBO(150, 155, 165, 0.42);
const _beatGridBeat = Color.fromRGBO(150, 155, 165, 0.26);
// Phase arrows sit on top of the bar lines, so they get more weight than the
// grid itself without becoming chrome.
const _phaseArrow = Color.fromRGBO(190, 195, 205, 0.9);
const _pendingLoopIn = Color.fromRGBO(251, 191, 36, 0.95);

Picture recordBeatGridPicture({
  required List<BeatMark> marks,
  required Size size,
  List<double> phaseArrows = const [],
}) {
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  final bar = Paint()
    ..color = _beatGridBar
    ..strokeWidth = 1
    ..isAntiAlias = false;
  final beat = Paint()
    ..color = _beatGridBeat
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
  final arrow = Paint()..color = _phaseArrow;
  for (final x in phaseArrows) {
    final arrowX = x.roundToDouble();
    // The stored downbeat phase can differ from the even beat grid, so the
    // arrow carries its own bar line instead of trusting one to be there.
    canvas.drawLine(Offset(arrowX, 0), Offset(arrowX, size.height), bar);
    _paintPhaseArrow(canvas, x: arrowX, height: size.height, paint: arrow);
  }
  return recorder.endRecording();
}

/// Down (top edge) and up (bottom edge) triangles marking a phase point.
void _paintPhaseArrow(
  Canvas canvas, {
  required double x,
  required double height,
  required Paint paint,
}) {
  const halfWidth = 5.0;
  const tip = 8.0;
  canvas.drawPath(
    Path()
      ..moveTo(x - halfWidth, 0)
      ..lineTo(x + halfWidth, 0)
      ..lineTo(x, tip)
      ..close(),
    paint,
  );
  canvas.drawPath(
    Path()
      ..moveTo(x - halfWidth, height)
      ..lineTo(x + halfWidth, height)
      ..lineTo(x, height - tip)
      ..close(),
    paint,
  );
}

Picture recordLoopPicture({
  required List<SavedLoop> loops,
  required int originMs,
  required int spanMs,
  required Size size,
  Color? accent,
  bool compact = false,
}) {
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  final color = accent ?? _fallbackLoopAccent;
  for (final loop in loops) {
    final rect = loopRegionRect(
      inMs: loop.inMs,
      outMs: loop.outMs,
      originMs: originMs,
      spanMs: spanMs,
      width: size.width,
      height: size.height,
    );
    if (rect == null) {
      continue;
    }
    paintLoopBand(
      canvas: canvas,
      rect: rect,
      size: size,
      accent: color,
      fill: color.withValues(alpha: 0.12),
      compact: compact,
    );
  }
  return recorder.endRecording();
}

/// Paint one saved-loop band.
///
/// Compact previews use the same 1px box as the delete affordance; the taller
/// scrolling lane gets the 2px in/out edges so the region reads at speed.
void paintLoopBand({
  required Canvas canvas,
  required Rect rect,
  required Size size,
  required Color accent,
  required Color fill,
  required bool compact,
  bool showFlag = true,
}) {
  canvas.drawRect(rect, Paint()..color = fill);
  if (compact) {
    canvas.drawRect(
      rect,
      Paint()
        ..color = accent.withValues(alpha: 0.55)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1
        ..isAntiAlias = false,
    );
  } else {
    final edge = Paint()
      ..color = accent.withValues(alpha: 0.85)
      ..strokeWidth = 2
      ..isAntiAlias = false;
    for (final x in [rect.left.roundToDouble(), rect.right.roundToDouble()]) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), edge);
    }
  }
  if (showFlag) {
    _paintLoopFlag(
      canvas,
      x: rect.left,
      accent: accent,
      laneHeight: size.height,
      width: size.width,
    );
  }
}

void _paintLoopFlag(
  Canvas canvas, {
  required double x,
  required Color accent,
  required double laneHeight,
  required double width,
}) {
  final flagH = laneHeight >= 64 ? 20.0 : (laneHeight * 0.42).clamp(10.0, 14.0);
  final flagW = flagH * 1.35;
  final maxLeft = math.max(0.0, width - flagW);
  final left = (x + 1).clamp(0.0, maxLeft);
  final bottom = flagH;
  final right = left + flagW;
  final notch = flagW * 0.20;
  final flag = Path()
    ..moveTo(left, 0)
    ..lineTo(right, 0)
    ..lineTo(right - notch, flagH / 2)
    ..lineTo(right, bottom)
    ..lineTo(left, bottom)
    ..close();
  canvas.drawPath(flag, Paint()..color = accent);
  final glyphColor = accent.computeLuminance() > 0.5
      ? const Color(0xFF000000)
      : const Color(0xFFFFFFFF);
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
      left + (flagW - notch - painter.width) / 2,
      (flagH - painter.height) / 2,
    ),
  );
}

/// Returns null when there is no engaged active loop to draw.
Picture? recordActiveLoopPicture({
  required LoopRegion? loop,
  required int originMs,
  required int spanMs,
  required Size size,
  Color? accent,
}) {
  if (loop == null || !loop.active) {
    return null;
  }
  final rect = loopRegionRect(
    inMs: loop.inMs,
    outMs: loop.outMs,
    originMs: originMs,
    spanMs: spanMs,
    width: size.width,
    height: size.height,
  );
  if (rect == null) {
    return null;
  }
  final color = accent ?? _fallbackLoopAccent;
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  paintLoopBand(
    canvas: canvas,
    rect: rect,
    size: size,
    accent: color,
    fill: color.withValues(alpha: 0.16),
    compact: false,
  );
  return recorder.endRecording();
}

/// Vertical amber marker for a pending Loop In (no active region yet).
Picture? recordPendingLoopInPicture({
  required int? pendingInMs,
  required int originMs,
  required int spanMs,
  required Size size,
}) {
  if (pendingInMs == null || spanMs <= 0) {
    return null;
  }
  final x = msToX(
    ms: pendingInMs,
    originMs: originMs,
    spanMs: spanMs,
    width: size.width,
  );
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  final line = Paint()
    ..color = _pendingLoopIn
    ..strokeWidth = 2
    ..isAntiAlias = false;
  canvas.drawLine(Offset(x, 0), Offset(x, size.height), line);
  final flag = Paint()..color = _pendingLoopIn;
  canvas.drawPath(
    Path()
      ..moveTo(x, 0)
      ..lineTo(x + 8, 0)
      ..lineTo(x, 8)
      ..close(),
    flag,
  );
  return recorder.endRecording();
}

Picture recordCuePicture({
  required List<DeckHotCue> cues,
  required int originMs,
  required int spanMs,
  required Size size,
}) {
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  for (final cue in cues) {
    final x = msToX(
      ms: cue.positionMs,
      originMs: originMs,
      spanMs: spanMs,
      width: size.width,
    );
    final color = hotCueAccent(cue.slot).border.withValues(alpha: 1);
    final line = Paint()
      ..color = color
      ..strokeWidth = 2
      ..isAntiAlias = false;
    canvas.drawLine(Offset(x, 0), Offset(x, size.height), line);
    _paintCueFlag(
      canvas,
      x: x,
      label: '${cue.slot + 1}',
      color: color,
      laneHeight: size.height,
    );
  }
  return recorder.endRecording();
}

void _paintCueFlag(
  Canvas canvas, {
  required double x,
  required String label,
  required Color color,
  required double laneHeight,
}) {
  // Strip pictures are authored at kWaveformStripHeight then scaled down to the
  // lane; size for post-scale readability without dominating the waveform.
  final flagH = laneHeight >= 64 ? 24.0 : (laneHeight * 0.5).clamp(12.0, 16.0);
  final flagW = flagH * 1.15;
  final fontSize = flagH * 0.58;
  final left = (x - flagW / 2).clamp(0.0, double.infinity);
  final rect = Rect.fromLTWH(left, 1, flagW, flagH);
  final rrect = RRect.fromRectAndRadius(rect, Radius.circular(flagH * 0.18));
  canvas.drawRRect(rrect, Paint()..color = color);
  canvas.drawRRect(
    rrect,
    Paint()
      ..color = const Color.fromRGBO(0, 0, 0, 0.4)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2,
  );
  final tp = TextPainter(
    text: TextSpan(
      text: label,
      style: TextStyle(
        color: const Color.fromARGB(255, 255, 255, 255),
        fontSize: fontSize,
        fontWeight: FontWeight.w700,
        height: 1,
      ),
    ),
    textDirection: TextDirection.ltr,
  )..layout(maxWidth: flagW);
  tp.paint(
    canvas,
    Offset(
      rect.left + (flagW - tp.width) / 2,
      rect.top + (flagH - tp.height) / 2,
    ),
  );
}
