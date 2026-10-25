import 'dart:convert';

import 'package:gui_flutter/core/utils/key_format.dart';

/// Smart-playlist rule document, mirroring `library_core::smart`.
///
/// The document crosses the Rust boundary as a JSON string (the same text that
/// is persisted in `collections.rules_json`), so the wire keys here are the
/// contract: `inList` → `in`, `durationMs` → `duration_ms`, group match →
/// `match`, a root group without `kind`, nested groups with
/// `"kind": "group"`. Key order matches the Rust serializer, so a parsed
/// document re-encodes to the exact bytes the library stores.
enum SmartMatch { all, any }

/// Track attribute a condition tests.
enum SmartField {
  title,
  artist,
  album,
  genre,
  comment,
  bpm,
  rating,
  year,
  durationMs,
  loudnessLufs,
  addedDaysAgo,
  key,
  colorLabel,
  deckBpmDelta,
  deckKey,
}

/// Comparison a condition applies.
enum SmartOp {
  between,
  contains,
  equals,
  inList,
  isEmpty,
  isNotEmpty,
  harmonicallyCompatible,
}

/// Sort key for the computed membership list.
enum SmartSortField {
  title,
  artist,
  bpm,
  rating,
  year,
  durationMs,
  loudnessLufs,
  addedDaysAgo,
}

/// Collection kind of a smart playlist: the wire token `CollectionType` reports
/// on the Rust side, so every `LibraryCollectionSummary.kind` check reads it
/// here instead of repeating the literal.
const kSmartPlaylistKind = 'smart_playlist';

/// Schema version written by the app; a stored document with another version is
/// reported rather than silently reinterpreted.
const smartPlaylistRulesVersion = 1;

extension SmartMatchInfo on SmartMatch {
  String get wire => switch (this) {
    SmartMatch.all => 'all',
    SmartMatch.any => 'any',
  };

  String get label => switch (this) {
    SmartMatch.all => 'All',
    SmartMatch.any => 'Any',
  };
}

extension SmartFieldInfo on SmartField {
  String get wire => switch (this) {
    SmartField.title => 'title',
    SmartField.artist => 'artist',
    SmartField.album => 'album',
    SmartField.genre => 'genre',
    SmartField.comment => 'comment',
    SmartField.bpm => 'bpm',
    SmartField.rating => 'rating',
    SmartField.year => 'year',
    SmartField.durationMs => 'duration_ms',
    SmartField.loudnessLufs => 'loudness_lufs',
    SmartField.addedDaysAgo => 'added_days_ago',
    SmartField.key => 'key',
    SmartField.colorLabel => 'color_label',
    SmartField.deckBpmDelta => 'deck_bpm_delta',
    SmartField.deckKey => 'deck_key',
  };

  String get label => switch (this) {
    SmartField.title => 'Title',
    SmartField.artist => 'Artist',
    SmartField.album => 'Album',
    SmartField.genre => 'Genre',
    SmartField.comment => 'Comment',
    SmartField.bpm => 'BPM',
    SmartField.rating => 'Rating',
    SmartField.year => 'Year',
    SmartField.durationMs => 'Duration',
    SmartField.loudnessLufs => 'Loudness (LUFS)',
    SmartField.addedDaysAgo => 'Days since added',
    SmartField.key => 'Key',
    SmartField.colorLabel => 'Colour label',
    SmartField.deckBpmDelta => 'Deck BPM (±%)',
    SmartField.deckKey => 'Deck key',
  };

  /// Deck-relative fields read a live deck instead of the track alone.
  bool get isDeck =>
      this == SmartField.deckBpmDelta || this == SmartField.deckKey;

  /// Operators this field accepts, mirroring the Rust field/operator matrix.
  List<SmartOp> get ops => smartFieldOps(this);
}

extension SmartOpInfo on SmartOp {
  String get wire => switch (this) {
    SmartOp.between => 'between',
    SmartOp.contains => 'contains',
    SmartOp.equals => 'equals',
    SmartOp.inList => 'in',
    SmartOp.isEmpty => 'is_empty',
    SmartOp.isNotEmpty => 'is_not_empty',
    SmartOp.harmonicallyCompatible => 'harmonically_compatible',
  };

  String get label => switch (this) {
    SmartOp.between => 'is between',
    SmartOp.contains => 'contains',
    SmartOp.equals => 'is',
    SmartOp.inList => 'is one of',
    SmartOp.isEmpty => 'is empty',
    SmartOp.isNotEmpty => 'is not empty',
    SmartOp.harmonicallyCompatible => 'is harmonically compatible',
  };
}

extension SmartSortFieldInfo on SmartSortField {
  String get wire => switch (this) {
    SmartSortField.title => 'title',
    SmartSortField.artist => 'artist',
    SmartSortField.bpm => 'bpm',
    SmartSortField.rating => 'rating',
    SmartSortField.year => 'year',
    SmartSortField.durationMs => 'duration_ms',
    SmartSortField.loudnessLufs => 'loudness_lufs',
    SmartSortField.addedDaysAgo => 'added_days_ago',
  };

  String get label => switch (this) {
    SmartSortField.title => 'Title',
    SmartSortField.artist => 'Artist',
    SmartSortField.bpm => 'BPM',
    SmartSortField.rating => 'Rating',
    SmartSortField.year => 'Year',
    SmartSortField.durationMs => 'Duration',
    SmartSortField.loudnessLufs => 'Loudness (LUFS)',
    SmartSortField.addedDaysAgo => 'Added',
  };
}

/// Operators [field] accepts — the single Dart mirror of the Rust matrix.
///
/// The first entry is the default the editor switches to when a field changes.
List<SmartOp> smartFieldOps(SmartField field) => switch (field) {
  SmartField.title || SmartField.artist || SmartField.album => const [
    SmartOp.contains,
    SmartOp.equals,
    SmartOp.isEmpty,
    SmartOp.isNotEmpty,
  ],
  SmartField.genre || SmartField.comment => const [
    SmartOp.contains,
    SmartOp.equals,
    SmartOp.inList,
    SmartOp.isEmpty,
    SmartOp.isNotEmpty,
  ],
  SmartField.bpm ||
  SmartField.rating ||
  SmartField.year ||
  SmartField.durationMs ||
  SmartField.loudnessLufs ||
  SmartField.addedDaysAgo => const [
    SmartOp.between,
    SmartOp.isEmpty,
    SmartOp.isNotEmpty,
  ],
  SmartField.key || SmartField.colorLabel => const [
    SmartOp.inList,
    SmartOp.isEmpty,
    SmartOp.isNotEmpty,
  ],
  SmartField.deckBpmDelta => const [SmartOp.between],
  SmartField.deckKey => const [SmartOp.equals, SmartOp.harmonicallyCompatible],
};

/// One rule: a condition or a nested group.
sealed class SmartRule {
  const new();

  Map<String, Object?> toJson();
}

/// Leaf condition.
final class SmartConditionRule extends SmartRule {
  const new({
    required this.field,
    required this.op,
    this.min,
    this.max,
    this.values = const [],
    this.deckId,
  });

  final SmartField field;
  final SmartOp op;

  /// Inclusive lower bound (`between` only).
  final double? min;

  /// Inclusive upper bound (`between` only).
  final double? max;

  /// Value list: one entry for `contains`/`equals`, up to 64 for `inList`.
  final List<String> values;

  /// Deck-relative fields only: `null` reads any playing deck.
  final int? deckId;

  @override
  Map<String, Object?> toJson() => {
    'kind': 'condition',
    'field': field.wire,
    'op': op.wire,
    if (min != null) 'min': min,
    if (max != null) 'max': max,
    if (values.isNotEmpty) 'values': values,
    if (deckId != null) 'deck': deckId,
  };
}

/// Rules combined by [match].
final class SmartGroupRule extends SmartRule {
  const new({required this.rules, this.match = SmartMatch.all});

  final SmartMatch match;
  final List<SmartRule> rules;

  /// Nested form, tagged so the parser can tell it from a condition.
  @override
  Map<String, Object?> toJson() => {'kind': 'group', ...toRootJson()};

  /// Root form: the document's `root` carries no `kind` tag.
  Map<String, Object?> toRootJson() => {
    'match': match.wire,
    'rules': [for (final rule in rules) rule.toJson()],
  };
}

/// Order of the computed membership list.
class SmartSort {
  const new({required this.field, required this.ascending});

  final SmartSortField field;
  final bool ascending;

  Map<String, Object?> toJson() => {
    'field': field.wire,
    'ascending': ascending,
  };
}

/// A smart playlist's persisted rules.
class SmartPlaylistRules {
  const new({
    this.root = const SmartGroupRule(rules: []),
    this.sort,
    this.version = smartPlaylistRulesVersion,
  });

  /// Empty document: no rules, so no tracks.
  const new empty()
    : root = const SmartGroupRule(rules: []),
      sort = null,
      version = smartPlaylistRulesVersion;

  /// Parse a stored document; tolerates absent `min`/`max`/`values`/`deck`.
  ///
  /// Throws [FormatException] when the text is not a rules document this app
  /// understands (unknown field/operator token, wrong shapes).
  factory parse(String json) {
    try {
      final document = SmartPlaylistRules._asMap(
        jsonDecode(json),
        'root document',
      );
      // `version` is required, as in the Rust parser: a document without it is
      // not one of ours, and guessing a version would reinterpret its rules.
      final version = document['version'];
      if (version is! int) {
        throw const FormatException('version must be an integer');
      }
      final sort = document['sort'];
      return SmartPlaylistRules(
        root: SmartPlaylistRules._parseGroup(document['root']),
        sort: sort == null ? null : SmartPlaylistRules._parseSort(sort),
        version: version,
      );
    } on FormatException catch (error) {
      throw FormatException('not a rules document: ${error.message}');
    }
  }

  final int version;
  final SmartSort? sort;
  final SmartGroupRule root;

  /// Canonical storage form (compact JSON, key order fixed by the serializers).
  String toJsonString() => jsonEncode({
    'version': version,
    if (sort != null) 'sort': sort!.toJson(),
    'root': root.toRootJson(),
  });

  /// True when any condition reads a deck.
  bool get usesDeckReference {
    bool groupUses(SmartGroupRule group) {
      for (final rule in group.rules) {
        switch (rule) {
          case SmartConditionRule(:final field) when field.isDeck:
            return true;
          case final SmartGroupRule nested:
            if (groupUses(nested)) {
              return true;
            }
          case SmartConditionRule():
            break;
        }
      }
      return false;
    }

    return groupUses(root);
  }

  /// First problem with the document, or `null` when it is valid.
  ///
  /// Mirrors `SmartPlaylistRules::validate` on the Rust side, message for
  /// message, so the editor can show the same text the library would reject.
  String? validate() {
    if (version != smartPlaylistRulesVersion) {
      return 'unsupported rules version $version';
    }
    return _validateGroup(root, 1, _RuleCounter());
  }

  static String? _validateGroup(
    SmartGroupRule group,
    int depth,
    _RuleCounter counter,
  ) {
    if (depth > _maxDepth) {
      return 'group nesting too deep (max $_maxDepth)';
    }
    for (final rule in group.rules) {
      switch (rule) {
        case final SmartConditionRule condition:
          counter.count += 1;
          if (counter.count > _maxConditions) {
            return 'too many rules (max $_maxConditions)';
          }
          final problem = _validateCondition(condition);
          if (problem != null) {
            return problem;
          }
        case final SmartGroupRule nested:
          if (nested.rules.isEmpty) {
            return 'groups must contain at least one rule';
          }
          final problem = _validateGroup(nested, depth + 1, counter);
          if (problem != null) {
            return problem;
          }
      }
    }
    return null;
  }

  static String? _validateCondition(SmartConditionRule rule) {
    final field = rule.field;
    final op = rule.op;
    if (op == SmartOp.harmonicallyCompatible && field != SmartField.deckKey) {
      return 'harmonically_compatible applies to deck_key only';
    }
    if (rule.deckId != null && !field.isDeck) {
      return 'deck only applies to deck rules';
    }
    if (!field.ops.contains(op)) {
      return 'op ${op.wire} is not allowed on ${field.wire}';
    }
    switch (op) {
      case SmartOp.between:
        return _validateBetween(rule);
      case SmartOp.contains || SmartOp.equals:
        if (field == SmartField.deckKey) {
          return null;
        }
        return _validateSingleValue(rule);
      case SmartOp.inList:
        return _validateValueList(rule);
      case SmartOp.isEmpty ||
          SmartOp.isNotEmpty ||
          SmartOp.harmonicallyCompatible:
        return null;
    }
  }

  static String? _validateBetween(SmartConditionRule rule) {
    final field = rule.field.wire;
    final min = rule.min;
    final max = rule.max;
    if (min == null && max == null) {
      return 'between needs min or max on $field';
    }
    if (!(min?.isFinite ?? true) || !(max?.isFinite ?? true)) {
      return 'bounds must be finite on $field';
    }
    if (min != null && max != null && min > max) {
      return 'min must not exceed max on $field';
    }
    if (rule.field == SmartField.rating) {
      for (final bound in [min, max]) {
        if (bound != null && (bound < 1 || bound > 5)) {
          return 'rating bounds must be within 1-5';
        }
      }
    }
    return null;
  }

  static String? _validateSingleValue(SmartConditionRule rule) {
    final op = rule.op.wire;
    final field = rule.field.wire;
    if (rule.values.length != 1) {
      return 'values must contain exactly one entry for $op on $field';
    }
    if (rule.values.single.trim().isEmpty) {
      return 'values must not be empty for $op on $field';
    }
    return null;
  }

  static String? _validateValueList(SmartConditionRule rule) {
    final field = rule.field;
    switch (field) {
      case SmartField.genre || SmartField.comment:
        final empty =
            'values must not be empty for ${rule.op.wire} '
            'on ${field.wire}';
        if (rule.values.isEmpty) {
          return empty;
        }
        if (rule.values.length > _maxValues) {
          return 'values must contain at most $_maxValues entries for '
              '${rule.op.wire} on ${field.wire}';
        }
        if (rule.values.any((value) => value.trim().isEmpty)) {
          return empty;
        }
        return null;
      case SmartField.key:
        if (rule.values.isEmpty) {
          return 'values must not be empty for ${rule.op.wire} '
              'on ${field.wire}';
        }
        for (final value in rule.values) {
          if (canonicalKey(value) == null) {
            return 'unknown key $value in key rule';
          }
        }
        return null;
      case SmartField.colorLabel:
        if (rule.values.isEmpty) {
          return 'values must not be empty for ${rule.op.wire} '
              'on ${field.wire}';
        }
        for (final value in rule.values) {
          if (!smartColorLabels.contains(value.trim().toLowerCase())) {
            return 'unknown color $value in colour rule';
          }
        }
        return null;
      case SmartField.title ||
          SmartField.artist ||
          SmartField.album ||
          SmartField.bpm ||
          SmartField.rating ||
          SmartField.year ||
          SmartField.durationMs ||
          SmartField.loudnessLufs ||
          SmartField.addedDaysAgo ||
          SmartField.deckBpmDelta ||
          SmartField.deckKey:
        return null;
    }
  }

  static SmartGroupRule _parseGroup(Object? json) {
    final map = _asMap(json, 'rule group');
    final rules = map['rules'];
    if (rules is! List) {
      throw const FormatException('rule group needs a rules list');
    }
    return SmartGroupRule(
      match: switch (map['match']) {
        null || 'all' => SmartMatch.all,
        'any' => SmartMatch.any,
        final other => throw FormatException('unknown match $other'),
      },
      rules: [for (final rule in rules) _parseRule(rule)],
    );
  }

  static SmartRule _parseRule(Object? json) {
    final map = _asMap(json, 'rule');
    if (map['kind'] == 'group') {
      return _parseGroup(map);
    }
    return _parseCondition(map);
  }

  static SmartConditionRule _parseCondition(Map<String, dynamic> map) {
    final rawValues = map['values'];
    final List<String> values;
    if (rawValues == null) {
      values = const [];
    } else if (rawValues is List &&
        rawValues.every((value) => value is String)) {
      values = rawValues.cast<String>().toList();
    } else {
      throw const FormatException('values must be a list of strings');
    }
    return SmartConditionRule(
      field: smartFieldFromWire(map['field']),
      op: smartOpFromWire(map['op']),
      min: _number(map['min'], 'min'),
      max: _number(map['max'], 'max'),
      values: values,
      deckId: _integer(map['deck'], 'deck'),
    );
  }

  static double? _number(Object? raw, String key) {
    if (raw == null) {
      return null;
    }
    if (raw is! num) {
      throw FormatException('$key must be a number');
    }
    return raw.toDouble();
  }

  static int? _integer(Object? raw, String key) {
    if (raw == null) {
      return null;
    }
    if (raw is! int) {
      throw FormatException('$key must be an integer');
    }
    return raw;
  }

  static SmartSort _parseSort(Object? json) {
    final map = _asMap(json, 'sort');
    final ascending = map['ascending'];
    if (ascending is! bool) {
      throw const FormatException('sort needs a boolean ascending');
    }
    return SmartSort(
      field: smartSortFieldFromWire(map['field']),
      ascending: ascending,
    );
  }

  static Map<String, dynamic> _asMap(Object? json, String what) {
    if (json is Map) {
      return json.cast<String, dynamic>();
    }
    throw FormatException('$what must be a JSON object');
  }
}

const _maxDepth = 4;
const _maxConditions = 64;
const _maxValues = 64;

/// Canonical colour-label names accepted by an `in` rule.
const smartColorLabels = [
  'pink',
  'red',
  'orange',
  'yellow',
  'green',
  'aqua',
  'blue',
  'purple',
];

/// Every musical key a `key` rule can list, minors first (Camelot order).
const List<String> smartKeys = [...kMinorKeys, ...kMajorKeys];

/// Camelot codes for the 24 key toggles: `1A`…`12A`, then `1B`…`12B`.
final List<String> smartCamelotCodes = [
  for (var number = 1; number <= 12; number++) '${number}A',
  for (var number = 1; number <= 12; number++) '${number}B',
];

class _RuleCounter {
  int count = 0;
}

/// Wire token → enum value, throwing for anything the app does not know.
T _fromWire<T>(
  List<T> values,
  String Function(T) wire,
  Object? raw,
  String what,
) {
  for (final value in values) {
    if (wire(value) == raw) {
      return value;
    }
  }
  throw FormatException('unknown $what $raw');
}

/// Wire token → [SmartField].
SmartField smartFieldFromWire(Object? wire) =>
    _fromWire(SmartField.values, (field) => field.wire, wire, 'field');

/// Wire token → [SmartOp].
SmartOp smartOpFromWire(Object? wire) =>
    _fromWire(SmartOp.values, (op) => op.wire, wire, 'op');

/// Wire token → [SmartSortField].
SmartSortField smartSortFieldFromWire(Object? wire) =>
    _fromWire(SmartSortField.values, (field) => field.wire, wire, 'sort field');

/// Camelot-compatible family of [key] in the same order as the Rust helper: the
/// key, one number down, one number up (same letter), then the relative
/// major/minor. Empty when [key] is not a recognized key.
List<String> harmonicFamily(String key) {
  final slot = camelotSlotForKey(canonicalKey(key) ?? key);
  if (slot == null) {
    return const [];
  }
  final (number, minor) = slot;
  final letter = minor ? 'A' : 'B';
  final opposite = minor ? 'B' : 'A';
  final codes = [
    '$number$letter',
    '${number == 1 ? 12 : number - 1}$letter',
    '${number == 12 ? 1 : number + 1}$letter',
    '$number$opposite',
  ];
  final family = <String>[];
  for (final code in codes) {
    final musical = camelotToMusical(code);
    if (musical != null && !family.contains(musical)) {
      family.add(musical);
    }
  }
  return family;
}

/// A one-press starting point for the rule editor.
class SmartPlaylistRulesPreset {
  const new({required this.name, required this.build});

  final String name;
  final SmartPlaylistRules Function() build;
}

const smartRulePresets = <SmartPlaylistRulesPreset>[
  SmartPlaylistRulesPreset(name: 'Peak time', build: _peakTimePreset),
  SmartPlaylistRulesPreset(name: 'Recently added', build: _recentlyAddedPreset),
  SmartPlaylistRulesPreset(name: 'Unrated', build: _unratedPreset),
  SmartPlaylistRulesPreset(
    name: 'Same as playing deck',
    build: _sameDeckPreset,
  ),
  SmartPlaylistRulesPreset(name: '±6% + harmonic key', build: _windowPreset),
];

SmartPlaylistRules _peakTimePreset() => const SmartPlaylistRules(
  root: SmartGroupRule(
    rules: [
      SmartConditionRule(
        field: SmartField.bpm,
        op: SmartOp.between,
        min: 126,
        max: 132,
      ),
      SmartConditionRule(
        field: SmartField.rating,
        op: SmartOp.between,
        min: 4,
        max: 5,
      ),
    ],
  ),
);

SmartPlaylistRules _recentlyAddedPreset() => const SmartPlaylistRules(
  sort: SmartSort(field: SmartSortField.addedDaysAgo, ascending: false),
  root: SmartGroupRule(
    rules: [
      SmartConditionRule(
        field: SmartField.addedDaysAgo,
        op: SmartOp.between,
        min: 0,
        max: 30,
      ),
    ],
  ),
);

SmartPlaylistRules _unratedPreset() => const SmartPlaylistRules(
  root: SmartGroupRule(
    rules: [SmartConditionRule(field: SmartField.rating, op: SmartOp.isEmpty)],
  ),
);

SmartPlaylistRules _sameDeckPreset() => const SmartPlaylistRules(
  root: SmartGroupRule(
    rules: [
      SmartConditionRule(
        field: SmartField.deckBpmDelta,
        op: SmartOp.between,
        min: -0.5,
        max: 0.5,
      ),
      SmartConditionRule(field: SmartField.deckKey, op: SmartOp.equals),
    ],
  ),
);

SmartPlaylistRules _windowPreset() => const SmartPlaylistRules(
  root: SmartGroupRule(
    rules: [
      SmartConditionRule(
        field: SmartField.deckBpmDelta,
        op: SmartOp.between,
        min: -6,
        max: 6,
      ),
      SmartConditionRule(
        field: SmartField.deckKey,
        op: SmartOp.harmonicallyCompatible,
      ),
    ],
  ),
);
