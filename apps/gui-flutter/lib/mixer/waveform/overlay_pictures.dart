import 'dart:ui';

import 'package:flutter/painting.dart';
import 'package:gui_flutter/mixer/pads/hot_cue_pads.dart';
import 'package:gui_flutter/mixer/pads/pad_button.dart';
import 'package:gui_flutter/mixer/waveform/beat_grid.dart';
import 'package:gui_flutter/mixer/waveform/overlay_geometry.dart';
import 'package:gui_flutter/src/rust/api/engine.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

const _savedLoopFill = Color.fromRGBO(255, 255, 255, 0.08);
const _savedLoopBorder = Color.fromRGBO(255, 255, 255, 0.20);
const _activeLoopFill = Color.fromRGBO(52, 211, 153, 0.18);
const _activeLoopBorder = Color.fromRGBO(52, 211, 153, 0.70);

Picture recordBeatGridPicture({
  required List<BeatMark> marks,
  required Size size,
}) {
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  final bar = Paint()
    ..color = const Color.fromARGB(80, 200, 205, 215)
    ..strokeWidth = 1
    ..isAntiAlias = false;
  final beat = Paint()
    ..color = const Color.fromARGB(55, 170, 175, 185)
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
  return recorder.endRecording();
}

Picture recordLoopPicture({
  required List<SavedLoopInfo> loops,
  required int durationMs,
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
  for (final loop in loops) {
    final rect = loopRegionRect(
      inMs: loop.inMs,
      outMs: loop.outMs,
      durationMs: durationMs,
      width: size.width,
      height: size.height,
    );
    if (rect == null) {
      continue;
    }
    canvas.drawRect(rect, fill);
    canvas.drawRect(rect, border);
  }
  return recorder.endRecording();
}

/// Returns null when there is no engaged active loop to draw.
Picture? recordActiveLoopPicture({
  required ActiveLoopInfo? loop,
  required int durationMs,
  required Size size,
}) {
  if (loop == null || !loop.active) {
    return null;
  }
  final rect = loopRegionRect(
    inMs: loop.inMs,
    outMs: loop.outMs,
    durationMs: durationMs,
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

Picture recordCuePicture({
  required List<DeckHotCue> cues,
  required int durationMs,
  required Size size,
}) {
  final recorder = PictureRecorder();
  final canvas = Canvas(recorder, Offset.zero & size);
  for (final cue in cues) {
    final x = msToX(
      ms: cue.positionMs,
      durationMs: durationMs,
      width: size.width,
    );
    final color = hotCueAccent(cue.slot).border.withValues(alpha: 1);
    final line = Paint()
      ..color = color
      ..strokeWidth = 1
      ..isAntiAlias = false;
    canvas.drawLine(Offset(x, 0), Offset(x, size.height), line);
    _paintCueFlag(canvas, x: x, label: '${cue.slot + 1}', color: color);
  }
  return recorder.endRecording();
}

void _paintCueFlag(
  Canvas canvas, {
  required double x,
  required String label,
  required Color color,
}) {
  const flagW = 14.0;
  const flagH = 12.0;
  final left = (x - flagW / 2).clamp(0.0, double.infinity);
  final rect = Rect.fromLTWH(left, 1, flagW, flagH);
  canvas.drawRRect(
    RRect.fromRectAndRadius(rect, const Radius.circular(2)),
    Paint()..color = color,
  );
  canvas.drawRRect(
    RRect.fromRectAndRadius(rect, const Radius.circular(2)),
    Paint()
      ..color = const Color.fromRGBO(0, 0, 0, 0.4)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1,
  );
  final tp = TextPainter(
    text: TextSpan(
      text: label,
      style: const TextStyle(
        color: Color.fromARGB(255, 255, 255, 255),
        fontSize: 9,
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
