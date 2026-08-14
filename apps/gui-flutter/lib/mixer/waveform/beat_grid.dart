class BeatMark {
  const BeatMark({required this.x, required this.isBar});

  final double x;
  final bool isBar;
}

List<BeatMark> beatGridXs({
  required double bpm,
  required double firstBeatSecs,
  required int startMs,
  required int endMs,
  required int positionMs,
  required double width,
  required int visibleMs,
}) {
  if (!(bpm > 20 && bpm < 400) || visibleMs <= 0 || width <= 0) {
    return const [];
  }
  final beatPeriodMs = 60_000 / bpm;
  final phaseMs = firstBeatSecs * 1000;
  final pxPerMs = width / visibleMs;
  final centerX = width / 2;
  var beatIndex = ((startMs - phaseMs) / beatPeriodMs).floor();
  final marks = <BeatMark>[];
  for (var i = 0; i < 10000; i++) {
    final beatMs = phaseMs + beatIndex * beatPeriodMs;
    if (beatMs > endMs + beatPeriodMs) {
      break;
    }
    if (beatMs >= startMs - 1e-6 && beatMs <= endMs + 1e-6) {
      final x = centerX + (beatMs - positionMs) * pxPerMs;
      if (x >= 0 && x <= width) {
        marks.add(BeatMark(x: x, isBar: beatIndex.remainder(4) == 0));
      }
    }
    beatIndex += 1;
  }
  return marks;
}
