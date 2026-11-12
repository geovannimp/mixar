import 'dart:ui';

import 'package:gui_flutter/core/utils/waveform_beat_grid.dart';
import 'package:gui_flutter/core/utils/waveform_overlay_geometry.dart';

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
