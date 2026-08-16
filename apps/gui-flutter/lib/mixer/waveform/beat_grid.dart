class BeatMark {
  const BeatMark({required this.x, required this.isBar});

  final double x;
  final bool isBar;

  @override
  bool operator ==(Object other) =>
      other is BeatMark && x == other.x && isBar == other.isBar;

  @override
  int get hashCode => Object.hash(x, isBar);
}

/// Beat x positions in the scrolling buffer (origin at the left edge).
///
/// ponytail: constant-tempo grid. Beats are extrapolated from `bpm` and a
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
