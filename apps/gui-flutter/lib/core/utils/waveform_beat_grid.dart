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
/// edge): one arrow per bar. Bars are derived from the segment-local downbeat
/// and meter, so variable-tempo grids keep their phase past each change.
List<double> phaseMarkXs({
  required List<BeatGridSegmentData> segments,
  required double originMs,
  required double spanMs,
  required double width,
}) => [
  for (final mark in beatGridXs(
    segments: segments,
    originMs: originMs,
    spanMs: spanMs,
    width: width,
  ))
    if (mark.isBar) mark.x,
];

/// Beat x positions in the scrolling buffer (origin at the left edge).
///
/// Each canonical segment contributes its own beats. The first segment runs
/// before the track starts, and the terminal segment runs past its last marker,
/// matching the old constant-grid extrapolation without flattening a
/// variable-tempo grid.
List<BeatMark> beatGridXs({
  required List<BeatGridSegmentData> segments,
  required double originMs,
  required double spanMs,
  required double width,
}) {
  if (segments.isEmpty || spanMs <= 0 || width <= 0) {
    return const [];
  }

  final windowStart = originMs;
  final windowEnd = originMs + spanMs;
  final byTime = <int, BeatMark>{};
  for (var i = 0; i < segments.length; i++) {
    final segment = segments[i];
    final bpm = segment.bpm;
    if (!(bpm > 20 && bpm < 400)) {
      continue;
    }
    final meter = segment.beatsPerBar.clamp(1, 16);
    final beatMs = 60_000 / bpm;
    final startMs = segment.startTimeMs.toDouble();
    // PlatformInt64 is int on native builds; convert for the web BigInt alias.
    // ignore: noop_primitive_operations
    final startBeat = segment.startBeatIndex.toInt();
    // PlatformInt64 is int on native builds; convert for the web BigInt alias.
    // ignore: noop_primitive_operations
    final count = segment.beatsToNext?.toInt();
    final firstIndex = i == 0
        ? ((windowStart - startMs) / beatMs).floor() + startBeat
        : ((windowStart - startMs) / beatMs).floor().clamp(
            startBeat,
            count == null ? startBeat : startBeat + count - 1,
          );
    final lastIndex = count == null
        ? ((windowEnd - startMs) / beatMs).ceil() + startBeat
        : startBeat + count - 1;
    final downbeatIndex = startBeat + segment.downbeatOffsetBeats;

    for (var index = firstIndex; index <= lastIndex; index++) {
      final time = startMs + (index - startBeat) * beatMs;
      if (time < windowStart - 1e-6 || time > windowEnd + 1e-6) {
        continue;
      }
      byTime[time.round()] = BeatMark(
        x: (time - originMs) / spanMs * width,
        isBar: (index - downbeatIndex) % meter == 0,
      );
    }
    if (byTime.length > 10000) {
      break;
    }
  }
  return byTime.values.toList()..sort((a, b) => a.x.compareTo(b.x));
}
