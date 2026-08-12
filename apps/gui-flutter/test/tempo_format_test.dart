import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/tempo_format.dart';

void main() {
  test('mid fader is unity pitch', () {
    expect(normToSpeedRatio(0.5), 1.0);
    expect(formatPitchPercent(0.5), '+0.00%');
  });

  test('range endpoints match ±tempoRange', () {
    expect(normToSpeedRatio(0, 0.06), closeTo(1.06, 1e-12));
    expect(normToSpeedRatio(1, 0.06), closeTo(0.94, 1e-12));
    expect(formatPitchPercent(0, 0.06), '+6.00%');
    expect(formatPitchPercent(1, 0.06), '-6.00%');
  });

  test('nextTempoRange cycles steps', () {
    expect(nextTempoRange(0.06), 0.10);
    expect(nextTempoRange(0.25), 0.06);
  });

  test('nextTempoRange falls back to the first step', () {
    expect(nextTempoRange(0.42), 0.06);
  });

  test('slider round-trip', () {
    expect(pitchSliderToSpeed(50), 0.5);
    expect(speedToPitchSlider(0.5), 50);
  });

  test('effectiveBpm scales with pitch; null when unloaded', () {
    expect(effectiveBpm(null, 0.5), isNull);
    expect(effectiveBpm(128, 0.5), 128);
    expect(effectiveBpm(100, 0, 0.06), closeTo(106, 1e-12));
  });

  test('formatBpm handles unloaded and non-finite input', () {
    expect(formatBpm(null), '—');
    expect(formatBpm(double.nan), '—');
    expect(formatBpm(128), '128.00');
  });

  test('formatTempoRange renders percent, and clamps invalid input', () {
    expect(formatTempoRange(0.06), '±6%');
    expect(formatTempoRange(-1), '±0%');
  });
}
