import 'dart:math' as math;

/// Default tempo fader half-span as pitch fraction (`0.06` = ±6%).
const kDefaultTempoRange = 0.06;

/// Pioneer / Mixxx-style cycle steps (Tauri `TEMPO_RANGE_STEPS`).
const kTempoRangeSteps = <double>[0.06, 0.10, 0.16, 0.25];

double _usableTempoRange(double tempoRange) =>
    tempoRange.isFinite && tempoRange > 0 ? tempoRange : 0.0;

/// Cycle to the next range step (wraps).
double nextTempoRange(double current) {
  const eps = 1e-4;
  final idx = kTempoRangeSteps.indexWhere((s) => (s - current).abs() < eps);
  return idx < 0
      ? kTempoRangeSteps.first
      : kTempoRangeSteps[(idx + 1) % kTempoRangeSteps.length];
}

/// Tempo fader `0..1` → playback ratio (±[tempoRange] fraction).
double normToSpeedRatio(
  double norm, [
  double tempoRange = kDefaultTempoRange,
]) {
  final n = norm.clamp(0.0, 1.0);
  return math.max(0.01, 1 + (0.5 - n) * 2 * _usableTempoRange(tempoRange));
}

/// Map tempo fader position `0..1` to slider 0–100.
double speedToPitchSlider(double speedNorm) {
  final n = speedNorm.clamp(0.0, 1.0);
  return (n * 10000).round() / 100;
}

/// Map slider 0–100 to tempo fader position `0..1`.
double pitchSliderToSpeed(double value) => (value / 100).clamp(0.0, 1.0);

double? effectiveBpm(
  double? bpm,
  double speedNorm, [
  double tempoRange = kDefaultTempoRange,
]) {
  if (bpm == null || !bpm.isFinite || bpm <= 0) {
    return null;
  }
  return bpm * normToSpeedRatio(speedNorm, tempoRange);
}

String formatBpm(double? bpm) {
  if (bpm == null || !bpm.isFinite) {
    return '—';
  }
  return bpm.toStringAsFixed(2);
}

/// Playback-ratio percent offset (e.g. `+6.00%`).
String formatPitchPercent(
  double speedNorm, [
  double tempoRange = kDefaultTempoRange,
]) {
  final percent = (normToSpeedRatio(speedNorm, tempoRange) - 1) * 100;
  final sign = percent >= 0 ? '+' : '';
  return '$sign${percent.toStringAsFixed(2)}%';
}

/// Format tempo range for UI (e.g. `±6%`).
String formatTempoRange(double tempoRange) {
  final pct = (_usableTempoRange(tempoRange) * 100).round();
  return '±$pct%';
}
