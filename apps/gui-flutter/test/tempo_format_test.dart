import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/tempo_format.dart';

void main() {
  test('mid fader is unity pitch', () {
    expect(normToSpeedRatio(0.5), 1.0);
    expect(formatPitchPercent(0.5), '+0.00%');
  });

  test('range endpoints match ±tempoRange', () {
    expect(normToSpeedRatio(0), closeTo(1.06, 1e-12));
    expect(normToSpeedRatio(1), closeTo(0.94, 1e-12));
    expect(formatPitchPercent(0), '+6.00%');
    expect(formatPitchPercent(1), '-6.00%');
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
    expect(effectiveBpm(100, 0), closeTo(106, 1e-12));
  });

  test('effectiveBpmFromRatio ignores the fader span; null when invalid', () {
    expect(effectiveBpmFromRatio(null, 1.2), isNull);
    expect(effectiveBpmFromRatio(128, 1.0), 128);
    // 100 BPM at 1.2× mirrors a 120 BPM master even though ±6% saturates at 106.
    expect(effectiveBpmFromRatio(100, 1.2), closeTo(120, 1e-12));
    expect(effectiveBpmFromRatio(100, 0), isNull);
    expect(effectiveBpmFromRatio(100, double.nan), isNull);
  });

  test('formatRatioPercent renders the actual ratio', () {
    expect(formatRatioPercent(1.2), '+20.00%');
    expect(formatRatioPercent(0.96), '-4.00%');
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
