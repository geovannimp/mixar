import 'package:flutter/foundation.dart';
import 'package:gui_flutter/src/rust/api/library.dart';

/// Phase shift applied when nudging the beat grid (matches Serato/Traktor fine nudge).
const kGridNudgeMs = 10;

const kGridBpmStep = 0.01;

const kGridBpmCoarseStep = 1.0;

const defaultGridBpm = 128.0;

double beatGridFirstBeatSecs(BeatGridData? grid) =>
    grid == null ? 0 : grid.firstBeatMs / 1000.0;

double nudgeFirstBeatSecs(double firstBeatSecs, int deltaMs) =>
    firstBeatSecs + deltaMs / 1000.0;

double stepGridBpm(double bpm, double delta) =>
    (bpm + delta).clamp(20.01, 399.99);

/// Parse a typed BPM; null if empty/invalid/out of range.
double? parseGridBpm(String raw) {
  final v = double.tryParse(raw.trim());
  if (v == null || !v.isFinite || v <= 20 || v >= 400) {
    return null;
  }
  return v.clamp(20.01, 399.99);
}

@immutable
class BeatMark {
  const new({required this.x, required this.isBar});

  final double x;
  final bool isBar;

  @override
  bool operator ==(Object other) =>
      other is BeatMark && x == other.x && isBar == other.isBar;

  @override
  int get hashCode => Object.hash(x, isBar);
}

/// Evenly spaced marks of a constant-tempo grid, as buffer x positions paired
/// with the grid index that produced them.
///
/// [firstIndex] is the index of the first mark to consider; it keeps the phase
/// of the grid relative to [anchorMs]. Marks outside the buffer or the window
/// are dropped.
List<({int index, double x})> evenGridXs({
  required double anchorMs,
  required double stepMs,
  required double originMs,
  required double spanMs,
  required double width,
  required int firstIndex,
}) {
  final marks = <({int index, double x})>[];
  if (!(stepMs > 0) || spanMs <= 0 || width <= 0) {
    return marks;
  }
  final pxPerMs = width / spanMs;
  final endMs = originMs + spanMs;
  var index = firstIndex;
  for (var n = 0; n < 10000; n++, index++) {
    final ms = anchorMs + index * stepMs;
    if (ms > endMs + 1e-6) {
      break;
    }
    final x = (ms - originMs) * pxPerMs;
    if (x >= 0 && x <= width) {
      marks.add((index: index, x: x));
    }
  }
  return marks;
}

/// X positions of the phase arrows in the scrolling buffer (origin at the left
/// edge): one arrow per bar, anchored on the first downbeat.
///
/// Bars also run before the first downbeat when the window starts before it, so
/// the lane keeps its grid before the track's audio.
List<double> phaseMarkXs({
  required double? bpm,
  required double firstDownbeatSecs,
  required int beatsPerBar,
  required double originMs,
  required double spanMs,
  required double width,
}) {
  final gridBpm = bpm;
  if (gridBpm == null || !(gridBpm > 0)) {
    return const [];
  }
  final meter = beatsPerBar.clamp(1, 16);
  final barMs = meter * 60_000 / gridBpm;
  final anchorMs = firstDownbeatSecs * 1000;
  return [
    for (final mark in evenGridXs(
      anchorMs: anchorMs,
      stepMs: barMs,
      originMs: originMs,
      spanMs: spanMs,
      width: width,
      firstIndex: ((originMs - anchorMs) / barMs).floor(),
    ))
      mark.x,
  ];
}

/// Beat x positions in the scrolling buffer (origin at the left edge).
///
/// Bars are marked from the first downbeat and the segment's meter, so a 3/4
/// grid arrows every third beat instead of assuming 4/4.
List<BeatMark> beatGridXs({
  required double bpm,
  required double firstBeatSecs,
  required int beatsPerBar,
  required double firstDownbeatSecs,
  required double originMs,
  required double spanMs,
  required double width,
}) {
  if (!(bpm > 20 && bpm < 400) || spanMs <= 0 || width <= 0) {
    return const [];
  }
  final meter = beatsPerBar.clamp(1, 16);
  final beatPeriodMs = 60_000 / bpm;
  final phaseMs = firstBeatSecs * 1000;
  final firstDownbeatIndex =
      ((firstDownbeatSecs * 1000 - phaseMs) / beatPeriodMs).round();
  return [
    for (final mark in evenGridXs(
      anchorMs: phaseMs,
      stepMs: beatPeriodMs,
      originMs: originMs,
      spanMs: spanMs,
      width: width,
      firstIndex: ((originMs - phaseMs) / beatPeriodMs).floor(),
    ))
      BeatMark(
        x: mark.x,
        isBar: (mark.index - firstDownbeatIndex) % meter == 0,
      ),
  ];
}
