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

  test('absolute mode shares hue on circle of fifths wedge', () {
    final cMajor = colorForKey('C', KeyColorMode.absolute)!;
    final aMinor = colorForKey('Am', KeyColorMode.absolute)!;
    expect(cMajor, equals(aMinor));

    final gMajor = colorForKey('G', KeyColorMode.absolute)!;
    expect(_hueDistance(cMajor, gMajor), closeTo(30, 1));

    final fMajor = colorForKey('F', KeyColorMode.absolute)!;
    expect(_hueDistance(cMajor, fMajor), closeTo(30, 1));
    expect(_hueDistance(gMajor, fMajor), closeTo(60, 1));
  });

  test('harmonic mode matches Rekordbox-style playing-deck reference', () {
    const ref = '2A';

    expect(
      harmonicMatchForKeys('2A', ref),
      HarmonicMatch.perfect,
    );
    expect(harmonicMatchForKeys('2B', ref), HarmonicMatch.perfect);
    expect(harmonicMatchForKeys('1A', ref), HarmonicMatch.perfect);
    expect(harmonicMatchForKeys('3A', ref), HarmonicMatch.perfect);
    expect(harmonicMatchForKeys('1B', ref), HarmonicMatch.compatible);
    expect(harmonicMatchForKeys('3B', ref), HarmonicMatch.compatible);
    expect(harmonicMatchForKeys('5A', ref), HarmonicMatch.none);

    expect(
      colorForKey('2A', KeyColorMode.harmonic, harmonicReferenceKey: ref),
      const Color(0xFF22C55E),
    );
    expect(
      colorForKey('1B', KeyColorMode.harmonic, harmonicReferenceKey: ref),
      const Color(0xFFEAB308),
    );
    expect(
      colorForKey('5A', KeyColorMode.harmonic, harmonicReferenceKey: ref),
      isNull,
    );
    expect(
      colorForKey('2A', KeyColorMode.harmonic),
      isNull,
    );
  });
}

double _hueDistance(Color a, Color b) {
  final ha = HSLColor.fromColor(a).hue;
  final hb = HSLColor.fromColor(b).hue;
  final delta = (ha - hb).abs();
  return delta > 180 ? 360 - delta : delta;
}
