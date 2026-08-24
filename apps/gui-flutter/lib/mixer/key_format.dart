/// Musical ↔ Camelot (Mixed In Key) key display helpers.
///
/// Matches `library_core::key_format` (`C` → `8B`, `Am` → `8A`; A=minor, B=major).
enum KeyDisplayMode { musical, camelot }

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
