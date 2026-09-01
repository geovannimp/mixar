import 'package:flutter/painting.dart';

/// Musical ↔ Camelot (Mixed In Key) key display helpers.
///
/// Matches `library_core::key_format` (`C` → `8B`, `Am` → `8A`; A=minor, B=major).
enum KeyDisplayMode { musical, camelot }

/// Track key color coding (mirrors persisted [`KeyColorModeSetting`]).
enum KeyColorMode { off, absolute, harmonic }

/// Circle-of-fifths majors starting at C. Index `i` → Camelot `(i + 7) % 12 + 1` + `B`.
const kMajorKeys = [
  'C',
  'G',
  'D',
  'A',
  'E',
  'B',
  'F#',
  'C#',
  'G#',
  'D#',
  'A#',
  'F',
];

/// Relative minors starting at Am. Index `i` → Camelot `(i + 7) % 12 + 1` + `A`.
const kMinorKeys = [
  'Am',
  'Em',
  'Bm',
  'F#m',
  'C#m',
  'G#m',
  'D#m',
  'A#m',
  'Fm',
  'Cm',
  'Gm',
  'Dm',
];

const _camelotOffset = 7;

/// Musical → Camelot (`C` → `8B`, `Am` → `8A`).
String? musicalToCamelot(String key) {
  final trimmed = key.trim();
  final major = kMajorKeys.indexOf(trimmed);
  if (major >= 0) {
    return '${(major + _camelotOffset) % 12 + 1}B';
  }
  final minor = kMinorKeys.indexOf(trimmed);
  if (minor >= 0) {
    return '${(minor + _camelotOffset) % 12 + 1}A';
  }
  return null;
}

/// Camelot → musical (`8B` → `C`, `8A` → `Am`). Accepts lower/upper letter suffix.
String? camelotToMusical(String code) {
  final trimmed = code.trim();
  if (trimmed.length < 2) {
    return null;
  }
  final upper = trimmed.toUpperCase();
  final bool minor;
  final String numberText;
  if (upper.endsWith('A')) {
    minor = true;
    numberText = upper.substring(0, upper.length - 1);
  } else if (upper.endsWith('B')) {
    minor = false;
    numberText = upper.substring(0, upper.length - 1);
  } else {
    return null;
  }
  final number = int.tryParse(numberText);
  if (number == null || number < 1 || number > 12) {
    return null;
  }
  final index = (number + 12 - 1 - _camelotOffset) % 12;
  return minor ? kMinorKeys[index] : kMajorKeys[index];
}

/// Camelot wheel slot `(1–12, A=minor / B=major)` for musical or Camelot input.
(int number, bool minor)? camelotSlotForKey(String key) {
  final trimmed = key.trim();
  if (trimmed.isEmpty) {
    return null;
  }
  final upper = trimmed.toUpperCase();
  if (upper.endsWith('A') || upper.endsWith('B')) {
    final minor = upper.endsWith('A');
    final numberText = upper.substring(0, upper.length - 1);
    final number = int.tryParse(numberText);
    if (number != null && number >= 1 && number <= 12) {
      return (number, minor);
    }
  }
  final camelot = musicalToCamelot(trimmed);
  if (camelot == null) {
    return null;
  }
  return camelotSlotForKey(camelot);
}

Color? colorForKey(String? key, KeyColorMode mode) {
  if (mode == KeyColorMode.off) {
    return null;
  }
  final trimmed = key?.trim();
  if (trimmed == null || trimmed.isEmpty) {
    return null;
  }
  final slot = camelotSlotForKey(trimmed);
  if (slot == null) {
    return null;
  }
  final (number, minor) = slot;
  return switch (mode) {
    KeyColorMode.off => null,
    KeyColorMode.absolute => _absoluteKeyColor(number, minor),
    KeyColorMode.harmonic => _harmonicKeyColor(number, minor),
  };
}

Color _absoluteKeyColor(int number, bool minor) {
  final index = (number - 1) * 2 + (minor ? 0 : 1);
  return HSLColor.fromAHSL(
    1,
    index * (360 / 24),
    0.78,
    minor ? 0.48 : 0.56,
  ).toColor();
}

Color _harmonicKeyColor(int number, bool minor) {
  return HSLColor.fromAHSL(
    1,
    (number - 1) * (360 / 12),
    minor ? 0.72 : 0.52,
    minor ? 0.46 : 0.58,
  ).toColor();
}

String formatDeckKey(String? key, KeyDisplayMode mode) {
  final trimmed = key?.trim();
  if (trimmed == null || trimmed.isEmpty) {
    return '—';
  }
  if (mode == KeyDisplayMode.musical) {
    return camelotToMusical(trimmed) ?? trimmed;
  }
  return musicalToCamelot(trimmed) ?? trimmed;
}
