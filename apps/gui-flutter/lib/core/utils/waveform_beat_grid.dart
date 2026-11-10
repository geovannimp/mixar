import 'package:gui_flutter/src/rust/api/library.dart';

/// Phase shift applied when nudging the beat grid (matches Serato/Traktor fine nudge).
const kGridNudgeMs = 10;

const kGridBpmStep = 0.01;

const kGridBpmCoarseStep = 1.0;

const defaultGridBpm = 128.0;

double beatGridFirstBeatSecs(BeatGridData? grid) =>
    grid == null || grid.beats.isEmpty ? 0 : grid.beats.first;

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

/// Beats per bar implied by the downbeat spacing, so arrows land on real bar
/// starts in 3/4 or 7/8 instead of assuming 4/4. Falls back to 4 when the grid
/// carries fewer than two downbeats.
int beatsPerBarFrom({required List<double> downbeats, double? bpm}) {
  final gridBpm = bpm;
  if (downbeats.length < 2 ||
      gridBpm == null ||
      !(gridBpm > 20 && gridBpm < 400)) {
    return 4;
  }
  final barSecs = downbeats[1] - downbeats[0];
  final beatSecs = 60 / gridBpm;
  if (!(barSecs > 0) || !(beatSecs > 0)) {
    return 4;
  }
  return (barSecs / beatSecs).round().clamp(1, 16);
}

/// Bar period in ms: the meter times `bpm` when the BPM is usable, so arrows
/// share the period of the beat grid lines instead of trusting a single
/// downbeat pair. Falls back to the stored downbeat spacing when the track has
/// no BPM.
double? barPeriodMs({required List<double> downbeats, double? bpm}) {
  final gridBpm = bpm;
  if (gridBpm != null && gridBpm > 20 && gridBpm < 400) {
    return beatsPerBarFrom(downbeats: downbeats, bpm: gridBpm) *
        60_000 /
        gridBpm;
  }
  if (downbeats.length >= 2) {
    final spacingMs = (downbeats[1] - downbeats[0]) * 1000;
    if (spacingMs > 0) {
      return spacingMs;
    }
  }
  return null;
}

/// X positions of the phase arrows in the scrolling buffer (origin at the left
/// edge): one arrow per bar, anchored on the first downbeat rather than the
/// track start.
///
/// The stored grid only covers the analyzed span (often the first 30 s), so the
/// arrows are stepped from the bar period across the whole buffer instead of
/// stopping with the stored downbeats. The first downbeat sets the phase; the
/// meter comes from the downbeat spacing, falling back to 4/4 from `bpm` and
/// `firstBeatSecs`.
List<double> phaseMarkXs({
  required double? bpm,
  required List<double> downbeatsSecs,
  required double firstBeatSecs,
  required double originMs,
  required double spanMs,
  required double width,
}) {
  if (spanMs <= 0 || width <= 0) {
    return const [];
  }
  final barMs = barPeriodMs(downbeats: downbeatsSecs, bpm: bpm);
  if (barMs == null || !(barMs > 0)) {
    return const [];
  }
  final anchorMs = downbeatsSecs.isNotEmpty
      ? downbeatsSecs.first * 1000
      : firstBeatSecs * 1000;
  final pxPerMs = width / spanMs;
  final endMs = originMs + spanMs;
  final marks = <double>[];
  var index = ((originMs - anchorMs) / barMs).ceil();
  if (index < 0) {
    index = 0;
  }
  for (var n = 0; n < 10000; n++, index++) {
    final ms = anchorMs + index * barMs;
    if (ms > endMs + 1e-6) {
      break;
    }
    final x = (ms - originMs) * pxPerMs;
    if (x >= 0 && x <= width) {
      marks.add(x);
    }
  }
  return marks;
}

/// Beat x positions in the scrolling buffer (origin at the left edge).
///
/// shortcut: constant-tempo grid. Beats are extrapolated from `bpm` and a
/// single phase, and every 4th beat is treated as a bar. Ignores
/// `BeatGridData.beats` (variable tempo) and `BeatGridData.downbeats` (real
/// bar starts, non-4/4). Upgrade: bisect `beats` for the visible span and mark
/// bars from `downbeats`.
List<BeatMark> beatGridXs({
  required double bpm,
  required double firstBeatSecs,
  required double originMs,
  required double spanMs,
  required double width,
}) {
  if (!(bpm > 20 && bpm < 400) || spanMs <= 0 || width <= 0) {
    return const [];
  }
  final beatPeriodMs = 60_000 / bpm;
  final phaseMs = firstBeatSecs * 1000;
  final pxPerMs = width / spanMs;
  final endMs = originMs + spanMs;
  var beatIndex = ((originMs - phaseMs) / beatPeriodMs).floor();
  final marks = <BeatMark>[];
  for (var i = 0; i < 10000; i++) {
    final beatMs = phaseMs + beatIndex * beatPeriodMs;
    if (beatMs > endMs + beatPeriodMs) {
      break;
    }
    if (beatMs >= originMs - 1e-6 && beatMs <= endMs + 1e-6) {
      final x = (beatMs - originMs) * pxPerMs;
      if (x >= 0 && x <= width) {
        marks.add(BeatMark(x: x, isBar: beatIndex % 4 == 0));
      }
    }
    beatIndex += 1;
  }
  return marks;
}
