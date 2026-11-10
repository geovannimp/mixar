import 'dart:math' as math;

/// Minimum Jaro-Winkler percentage for a fuzzy settings match.
const settingsSearchFuzzyThreshold = 85.0;

/// Best Jaro-Winkler score between [foldedQuery] and [candidates].
///
/// Used only after exact substring matching finds nothing, so typos such as
/// `rekordbxo` still reach Integrations. Short queries are ignored to avoid
/// noisy matches.
double settingsSearchFuzzyScore(
  String foldedQuery,
  Iterable<String> candidates,
) {
  final query = foldedQuery.trim();
  if (query.length < 3) {
    return 0;
  }
  var best = 0.0;
  for (final candidate in candidates) {
    final folded = foldSettingsSearchText(candidate);
    best = _maxScore(query, folded, best);
    for (final token in folded.split(RegExp('[^a-z0-9]+'))) {
      if (token.isEmpty) {
        continue;
      }
      best = _maxScore(query, token, best);
    }
  }
  return best;
}

double _maxScore(String query, String candidate, double best) {
  final score = _jaroWinklerScore(query, candidate);
  return score > best ? score : best;
}

/// Jaro-Winkler similarity in percent (0-100).
///
/// Local implementation so settings search has no third-party dependency.
double _jaroWinklerScore(String a, String b) {
  if (a == b) {
    return 100;
  }
  if (a.isEmpty || b.isEmpty) {
    return 0;
  }

  final matchDistance = math.max(0, math.max(a.length, b.length) ~/ 2 - 1);
  final aMatches = List<bool>.filled(a.length, false);
  final bMatches = List<bool>.filled(b.length, false);
  var matches = 0;

  for (var i = 0; i < a.length; i++) {
    final start = math.max(0, i - matchDistance);
    final end = math.min(i + matchDistance + 1, b.length);
    for (var j = start; j < end; j++) {
      if (bMatches[j] || a[i] != b[j]) {
        continue;
      }
      aMatches[i] = true;
      bMatches[j] = true;
      matches++;
      break;
    }
  }

  if (matches == 0) {
    return 0;
  }

  var transpositions = 0;
  var bIndex = 0;
  for (var i = 0; i < a.length; i++) {
    if (!aMatches[i]) {
      continue;
    }
    while (!bMatches[bIndex]) {
      bIndex++;
    }
    if (a[i] != b[bIndex]) {
      transpositions++;
    }
    bIndex++;
  }

  final matched = matches.toDouble();
  final jaro =
      (matched / a.length +
          matched / b.length +
          (matched - transpositions / 2) / matched) /
      3;

  var prefix = 0;
  final maxPrefix = math.min(4, math.min(a.length, b.length));
  while (prefix < maxPrefix && a[prefix] == b[prefix]) {
    prefix++;
  }

  final winkler = jaro + prefix * 0.1 * (1 - jaro);
  return winkler * 100;
}

const _accentFolds = <String, String>{
  'á': 'a',
  'à': 'a',
  'â': 'a',
  'ã': 'a',
  'ä': 'a',
  'é': 'e',
  'è': 'e',
  'ê': 'e',
  'ë': 'e',
  'í': 'i',
  'ì': 'i',
  'î': 'i',
  'ï': 'i',
  'ó': 'o',
  'ò': 'o',
  'ô': 'o',
  'õ': 'o',
  'ö': 'o',
  'ú': 'u',
  'ù': 'u',
  'û': 'u',
  'ü': 'u',
  'ç': 'c',
  'ñ': 'n',
};

/// Lowercases [value] and replaces accented Latin letters with their ASCII
/// counterparts so settings search ignores case and diacritics.
///
/// The fold table covers the Latin letters the shipped locales (en, pt-BR)
/// use. A locale with other diacritics needs entries added here; pulling in
/// ICU/`intl` folding is not warranted until one ships.
String foldSettingsSearchText(String value) {
  final buffer = StringBuffer();
  for (final char in value.toLowerCase().split('')) {
    buffer.write(_accentFolds[char] ?? char);
  }
  return buffer.toString();
}

/// Whether [haystack] contains [foldedQuery].
///
/// [foldedQuery] must already come from [foldSettingsSearchText]; the haystack
/// is folded here.
bool settingsSearchMatches(String haystack, String foldedQuery) =>
    foldSettingsSearchText(haystack).contains(foldedQuery);
