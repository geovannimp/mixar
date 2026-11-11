import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:gui_flutter/core/models/bus_enums.dart';
import 'package:gui_flutter/core/utils/waveform_beat_grid.dart';
import 'package:gui_flutter/core/utils/waveform_overlay_geometry.dart';
import 'package:gui_flutter/src/rust/api/event_mirrors.dart' show SavedLoop;
import 'package:gui_flutter/widgets/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/widgets/mixer/pads/pad_button.dart';

// Saved-loop fill stays neutral so it reads on both the near-black dark lane
// and the light one; the edges reuse the active-loop teal at lower alpha so a
// remembered region is recognizable without competing with the active loop.
const _savedLoopFill = Color.fromRGBO(148, 163, 184, 0.24);
const _savedLoopBorder = Color.fromRGBO(52, 211, 153, 0.45);
const _savedLoopMarker = Color.fromRGBO(52, 211, 153, 0.85);
const _beatGridBar = Color.fromRGBO(150, 155, 165, 0.42);
const _beatGridBeat = Color.fromRGBO(150, 155, 165, 0.26);
// Phase arrows sit on top of the bar lines, so they get more weight than the
// grid itself without becoming chrome.
const _phaseArrow = Color.fromRGBO(190, 195, 205, 0.9);
const _activeLoopFill = Color.fromRGBO(52, 211, 153, 0.18);
const _activeLoopBorder = Color.fromRGBO(52, 211, 153, 0.70);
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
}) {
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  final fill = Paint()
    ..color = _savedLoopFill
    ..style = PaintingStyle.fill;
  final border = Paint()
    ..color = _savedLoopBorder
    ..style = PaintingStyle.stroke
    ..strokeWidth = 1
    ..isAntiAlias = false;
  final edge = Paint()
    ..color = _savedLoopMarker
    ..strokeWidth = 2
    ..isAntiAlias = false;
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
    canvas.drawRect(rect, fill);
    canvas.drawRect(rect, border);
    for (final x in [rect.left.roundToDouble(), rect.right.roundToDouble()]) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), edge);
      canvas.drawLine(Offset(x - 3, 1), Offset(x + 3, 1), edge);
      canvas.drawLine(
        Offset(x - 3, size.height - 1),
        Offset(x + 3, size.height - 1),
        edge,
      );
    }
  }
  return recorder.endRecording();
}

/// Returns null when there is no engaged active loop to draw.
Picture? recordActiveLoopPicture({
  required LoopRegion? loop,
  required int originMs,
  required int spanMs,
  required Size size,
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
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  canvas.drawRect(
    rect,
    Paint()
      ..color = _activeLoopFill
      ..style = PaintingStyle.fill,
  );
  canvas.drawRect(
    rect,
    Paint()
      ..color = _activeLoopBorder
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..isAntiAlias = false,
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
      ..strokeWidth = 1
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
      ..strokeWidth = 1,
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
