import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/mixer/key_format.dart';

void main() {
  test('formatDeckKey musical and camelot Mixed In Key round-trip', () {
    expect(formatDeckKey(null, KeyDisplayMode.musical), '—');
    expect(formatDeckKey('  ', KeyDisplayMode.camelot), '—');
    expect(formatDeckKey('C', KeyDisplayMode.camelot), '8B');
    expect(formatDeckKey('Am', KeyDisplayMode.camelot), '8A');
    expect(formatDeckKey('8B', KeyDisplayMode.musical), 'C');
    expect(formatDeckKey('8A', KeyDisplayMode.musical), 'Am');
    expect(formatDeckKey('1A', KeyDisplayMode.musical), 'G#m');
    expect(formatDeckKey('unknown', KeyDisplayMode.camelot), 'unknown');
  });

  test('colorForKey absolute vs harmonic', () {
    expect(colorForKey(null, KeyColorMode.off), isNull);
    expect(colorForKey('C', KeyColorMode.off), isNull);
    expect(colorForKey('', KeyColorMode.absolute), isNull);
    expect(colorForKey('unknown', KeyColorMode.harmonic), isNull);

    final cMajor = colorForKey('C', KeyColorMode.absolute)!;
    final aMinor = colorForKey('Am', KeyColorMode.absolute)!;
    expect(cMajor, isNot(equals(aMinor)));

    final slot8 = camelotSlotForKey('C');
    expect(slot8, (8, false));
    expect(camelotSlotForKey('Am'), (8, true));

    final cHarmonic = colorForKey('C', KeyColorMode.harmonic)!;
    final aHarmonic = colorForKey('Am', KeyColorMode.harmonic)!;
    expect(cHarmonic, isNot(equals(aHarmonic)));
    expect(
      _hueDistance(cHarmonic, aHarmonic),
      lessThan(_hueDistance(cHarmonic, colorForKey('7B', KeyColorMode.harmonic)!)),
    );

    final neighbor7 = colorForKey('7B', KeyColorMode.harmonic)!;
    final distant6 = colorForKey('6B', KeyColorMode.harmonic)!;
    expect(
      _hueDistance(cHarmonic, neighbor7),
      lessThan(_hueDistance(cHarmonic, distant6)),
    );
  });
}

double _hueDistance(Color a, Color b) {
  final ha = HSLColor.fromColor(a).hue;
  final hb = HSLColor.fromColor(b).hue;
  final delta = (ha - hb).abs();
  return delta > 180 ? 360 - delta : delta;
}
