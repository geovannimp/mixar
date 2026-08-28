import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/jog_ticks.dart';

void main() {
  test('degreesToJogTicks maps a full turn', () {
    expect(degreesToJogTicks(360), kJogIntervalsPerRev);
    expect(degreesToJogTicks(180), kJogIntervalsPerRev ~/ 2);
    expect(degreesToJogTicks(0), 0);
    expect(degreesToJogTicks(double.nan), 0);
  });

  test('vinylTicksToDeltaMs matches 33⅓ platter mapping', () {
    expect(vinylTicksToDeltaMs(0), 0);
    expect(vinylTicksToDeltaMs(kJogIntervalsPerRev), 1800);
    expect(vinylTicksToDeltaMs(-kJogIntervalsPerRev ~/ 2), -900);
  });

  test('barCycleRotationDeg scales with position', () {
    expect(barCycleRotationDeg(0, 120), 0);
    expect(barCycleDurationMs(120), 8000);
    expect(barCycleRotationDeg(8000, 120), 360);
    expect(barCycleDurationMs(0), isNull);
  });
}
