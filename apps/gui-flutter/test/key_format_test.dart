import 'package:flutter/painting.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/utils/key_format.dart';

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

  test('flat spellings normalize to sharp Camelot slots', () {
    expect(musicalToCamelot('Bb'), musicalToCamelot('A#'));
    expect(musicalToCamelot('Ebm'), musicalToCamelot('D#m'));
    expect(camelotSlotForKey('Bb'), camelotSlotForKey('A#'));
    expect(camelotSlotForKey('Ebm'), camelotSlotForKey('D#m'));
    expect(formatDeckKey('Bb', KeyDisplayMode.camelot), '6B');
    expect(colorForKey('Bb', KeyColorMode.absolute), isNotNull);
  });

  test('absolute mode shares hue on circle of fifths wedge', () {
    final cMajor = colorForKey('C', KeyColorMode.absolute)!;
    final aMinor = colorForKey('Am', KeyColorMode.absolute)!;
    expect(cMajor, isNot(equals(aMinor)));
    expect(_hueDistance(cMajor, aMinor), closeTo(0, 1));

    final gMajor = colorForKey('G', KeyColorMode.absolute)!;
    expect(_hueDistance(cMajor, gMajor), closeTo(30, 1));

    final fMajor = colorForKey('F', KeyColorMode.absolute)!;
    expect(_hueDistance(cMajor, fMajor), closeTo(30, 1));
    expect(_hueDistance(gMajor, fMajor), closeTo(60, 1));

    expect(absoluteHueForCamelotNumber(7), closeTo(330, 0.001));
    expect(colorForKey('F', KeyColorMode.absolute), isNotNull);
  });

  test('transposeKey shifts musical and camelot keys', () {
    expect(transposeKey('C', 2), 'D');
    expect(transposeKey('Am', 2), 'Bm');
    expect(transposeKey('C', -1), 'B');
    expect(transposeKey('F#', 1), 'G');
    expect(transposeKey('8B', 1), 'C#');
    expect(transposeKey(null, 2), isNull);
    expect(transposeKey('  ', 2), isNull);
    expect(transposeKey('unknown', 2), isNull);
  });

  test('transposeKey accepts flats and preserves spelling on zero shift', () {
    expect(transposeKey('Db', 2), transposeKey('C#', 2));
    expect(transposeKey('Ebm', 2), transposeKey('D#m', 2));
    expect(transposeKey('Bb', 0), 'Bb');
    expect(transposeKey('8B', 0), 'C');
    expect(transposeKey('8A', 0), 'Am');
    expect(deckSoundingKeyLabel('Bb', 0, KeyDisplayMode.musical), 'Bb');
  });

  test('deckSoundingKeyLabel applies the session key shift', () {
    expect(deckSoundingKeyLabel('C', 2, KeyDisplayMode.musical), 'D');
    expect(deckSoundingKeyLabel('Am', -2, KeyDisplayMode.musical), 'Gm');
    expect(deckSoundingKeyLabel('C', 0, KeyDisplayMode.musical), 'C');
    expect(deckSoundingKeyLabel('8B', 1, KeyDisplayMode.musical), 'C#');
    expect(
      deckSoundingKeyLabel('8B', 1, KeyDisplayMode.camelot),
      musicalToCamelot('C#'),
    );
    expect(deckSoundingKeyLabel(null, 5, KeyDisplayMode.musical), '—');
    expect(
      deckSoundingKeyLabel('unknown', 5, KeyDisplayMode.musical),
      'unknown',
    );
  });

  test('harmonic mode matches Rekordbox-style playing-deck reference', () {
    const ref = '2A';

    expect(harmonicMatchForKeys('2A', ref), HarmonicMatch.perfect);
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
    expect(colorForKey('2A', KeyColorMode.harmonic), isNull);
  });

  test('canonicalKey folds flats, Camelot codes, and a lower-case root', () {
    expect(canonicalKey('Bb'), 'A#');
    expect(canonicalKey('gb'), 'F#');
    expect(canonicalKey('bm'), 'Bm');
    expect(canonicalKey('8A'), 'Am');
    expect(canonicalKey(' 12b '), 'E');
    expect(canonicalKey('Am'), 'Am');
    expect(canonicalKey(''), isNull);
    expect(canonicalKey('H'), isNull);
  });
}

double _hueDistance(Color a, Color b) {
  final ha = HSLColor.fromColor(a).hue;
  final hb = HSLColor.fromColor(b).hue;
  final delta = (ha - hb).abs();
  return delta > 180 ? 360 - delta : delta;
}
