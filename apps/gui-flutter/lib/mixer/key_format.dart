/// Musical vs Camelot key display (Tauri `key-format.ts`). Session-only.
enum KeyDisplayMode { musical, camelot }

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

String? musicalToCamelot(String key) {
  final trimmed = key.trim();
  final major = kMajorKeys.indexOf(trimmed);
  if (major >= 0) {
    return '${major + 1}A';
  }
  final minor = kMinorKeys.indexOf(trimmed);
  if (minor >= 0) {
    return '${minor + 1}B';
  }
  return null;
}

String? camelotToMusical(String code) {
  final trimmed = code.trim().toUpperCase();
  final minor = trimmed.endsWith('B');
  final major = trimmed.endsWith('A');
  if (!minor && !major) {
    return null;
  }
  final number = int.tryParse(trimmed.substring(0, trimmed.length - 1));
  if (number == null || number < 1 || number > 12) {
    return null;
  }
  final index = number - 1;
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
