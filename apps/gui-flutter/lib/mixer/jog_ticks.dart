import 'dart:math' as math;

/// Must match engine-dsp `JOG_INTERVALS_PER_REV`.
const kJogIntervalsPerRev = 720;

/// Default DJ time signature: 4/4.
const kDefaultBeatsPerBar = 4;

/// Bars per jog position cycle.
const kJogBarCycleLength = 4;

/// Convert an angular delta in degrees to relative jog ticks.
int degreesToJogTicks(double deltaDeg) {
  if (!deltaDeg.isFinite || deltaDeg == 0) {
    return 0;
  }
  return ((deltaDeg / 360) * kJogIntervalsPerRev).round();
}

double? barCycleDurationMs(
  double bpm, [
  int beatsPerBar = kDefaultBeatsPerBar,
  int cycleBars = kJogBarCycleLength,
]) {
  if (!bpm.isFinite || bpm <= 0) {
    return null;
  }
  final beatsInCycle = cycleBars * beatsPerBar;
  final ms = ((beatsInCycle * 60) / bpm) * 1000;
  if (ms <= 0) {
    return null;
  }
  return ms;
}

/// Continuous jog tracker angle (no 0–360 wrap).
double barCycleRotationDeg(
  int positionMs,
  double bpm, [
  int beatsPerBar = kDefaultBeatsPerBar,
  int cycleBars = kJogBarCycleLength,
]) {
  final cycle = barCycleDurationMs(bpm, beatsPerBar, cycleBars);
  if (cycle == null) {
    return 0;
  }
  return (positionMs / cycle) * 360;
}

double pointerAngleDeg(
  double localX,
  double localY,
  double width,
  double height,
) {
  final cx = width / 2;
  final cy = height / 2;
  return math.atan2(localY - cy, localX - cx) * 180 / math.pi;
}
