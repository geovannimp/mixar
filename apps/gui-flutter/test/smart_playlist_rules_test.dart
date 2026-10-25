//! Model parity with `library_core::smart`: the exact stored bytes, the deck
//! selector round-trip, and the validation messages the library would return.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';

/// Where the Rust suite pins the same document. One fixture, read by both
/// suites, so the two serializers cannot drift onto different documents.
const _fixturePath =
    '../../crates/library-core/tests/fixtures/smart_playlist_rules_v1.json';

late String _fixture;

void main() {
  setUpAll(() {
    final file = File(_fixturePath);
    if (!file.existsSync()) {
      fail('missing shared fixture at ${file.absolute.path}');
    }
    _fixture = file.readAsStringSync().trimRight();
  });

  test('parses and re-encodes the stored document byte for byte', () {
    final rules = SmartPlaylistRules.parse(_fixture);
    expect(rules.toJsonString(), _fixture);
    expect(rules.version, 1);
    expect(rules.sort?.field, SmartSortField.bpm);
    expect(rules.sort?.ascending, isTrue);
    expect(rules.usesDeckReference, isTrue);
  });

  test('an empty document is valid and matches no tracks', () {
    const rules = SmartPlaylistRules.empty();
    expect(rules.validate(), isNull);
    expect(rules.root.rules, isEmpty);
    expect(rules.usesDeckReference, isFalse);
    expect(
      rules.toJsonString(),
      '{"version":1,"root":{"match":"all","rules":[]}}',
    );
  });

  test('deck selector round-trips and is omitted when absent', () {
    const pinned = SmartConditionRule(
      field: SmartField.deckKey,
      op: SmartOp.equals,
      deckId: 0,
    );
    expect(pinned.toJson()['deck'], 0);

    const anyDeck = SmartConditionRule(
      field: SmartField.deckKey,
      op: SmartOp.equals,
    );
    expect(anyDeck.toJson().containsKey('deck'), isFalse);

    final parsed = SmartPlaylistRules.parse(_fixture);
    final conditions = parsed.root.rules.whereType<SmartConditionRule>();
    final deckConditions = conditions.where((rule) => rule.field.isDeck);
    expect(deckConditions.where((rule) => rule.deckId == 0), hasLength(1));
    expect(deckConditions.where((rule) => rule.deckId == null), hasLength(2));
  });

  test('harmonicFamily matches the Rust helper order', () {
    expect(harmonicFamily('8A'), ['Am', 'Dm', 'Em', 'C']);
    expect(harmonicFamily('C'), ['C', 'F', 'G', 'Am']);
    expect(harmonicFamily('nonsense'), isEmpty);
  });

  test('each field exposes its own operator list', () {
    const text = [
      SmartOp.contains,
      SmartOp.equals,
      SmartOp.isEmpty,
      SmartOp.isNotEmpty,
    ];
    // `||` in these switch labels is Dart's logical-or *pattern*: it matches
    // either constant, so all three text fields and both tag fields share a
    // list instead of only the first one.
    expect(smartFieldOps(SmartField.title), text);
    expect(smartFieldOps(SmartField.artist), text);
    expect(smartFieldOps(SmartField.album), text);
    expect(smartFieldOps(SmartField.genre), [
      SmartOp.contains,
      SmartOp.equals,
      SmartOp.inList,
      SmartOp.isEmpty,
      SmartOp.isNotEmpty,
    ]);
    expect(smartFieldOps(SmartField.comment), smartFieldOps(SmartField.genre));
    expect(smartFieldOps(SmartField.bpm), [
      SmartOp.between,
      SmartOp.isEmpty,
      SmartOp.isNotEmpty,
    ]);
    expect(smartFieldOps(SmartField.rating), smartFieldOps(SmartField.bpm));
    expect(smartFieldOps(SmartField.key), [
      SmartOp.inList,
      SmartOp.isEmpty,
      SmartOp.isNotEmpty,
    ]);
    expect(smartFieldOps(SmartField.colorLabel), smartFieldOps(SmartField.key));
    expect(smartFieldOps(SmartField.deckBpmDelta), [SmartOp.between]);
    expect(smartFieldOps(SmartField.deckKey), [
      SmartOp.equals,
      SmartOp.harmonicallyCompatible,
    ]);
  });

  test('presets are all valid documents', () {
    expect(smartRulePresets, hasLength(5));
    for (final preset in smartRulePresets) {
      final rules = preset.build();
      expect(rules.validate(), isNull, reason: preset.name);
      expect(rules.root.rules, isNotEmpty, reason: preset.name);
    }
    final deckPresets = smartRulePresets
        .where((preset) => preset.build().usesDeckReference)
        .map((preset) => preset.name);
    expect(deckPresets, ['Same as playing deck', '±6% + harmonic key']);
  });

  test('validation mirrors the library messages', () {
    SmartPlaylistRules single(SmartConditionRule condition) =>
        SmartPlaylistRules(root: SmartGroupRule(rules: [condition]));

    expect(
      single(
        const SmartConditionRule(
          field: SmartField.rating,
          op: SmartOp.between,
          max: 6,
        ),
      ).validate(),
      'rating bounds must be within 1-5',
    );
    expect(
      single(
        const SmartConditionRule(
          field: SmartField.title,
          op: SmartOp.between,
          min: 1,
        ),
      ).validate(),
      'op between is not allowed on title',
    );
    expect(
      single(
        const SmartConditionRule(field: SmartField.title, op: SmartOp.contains),
      ).validate(),
      'values must contain exactly one entry for contains on title',
    );
    expect(
      single(
        const SmartConditionRule(
          field: SmartField.key,
          op: SmartOp.inList,
          values: ['8A'],
          deckId: 0,
        ),
      ).validate(),
      'deck only applies to deck rules',
    );
    expect(
      single(
        const SmartConditionRule(
          field: SmartField.bpm,
          op: SmartOp.between,
          min: 130,
          max: 120,
        ),
      ).validate(),
      'min must not exceed max on bpm',
    );
    expect(
      const SmartPlaylistRules(
        root: SmartGroupRule(rules: [SmartGroupRule(rules: [])]),
      ).validate(),
      'groups must contain at least one rule',
    );
    expect(
      const SmartPlaylistRules(version: 2).validate(),
      'unsupported rules version 2',
    );
  });

  test('key rules accept the spellings the library accepts', () {
    const accepted = SmartPlaylistRules(
      root: SmartGroupRule(
        rules: [
          SmartConditionRule(
            field: SmartField.key,
            op: SmartOp.inList,
            values: ['bm', ' gb ', '8A'],
          ),
        ],
      ),
    );
    expect(accepted.validate(), isNull);

    const unknown = SmartPlaylistRules(
      root: SmartGroupRule(
        rules: [
          SmartConditionRule(
            field: SmartField.key,
            op: SmartOp.inList,
            values: ['H'],
          ),
        ],
      ),
    );
    expect(unknown.validate(), 'unknown key H in key rule');
  });

  test('malformed documents are rejected instead of coerced', () {
    const malformed = [
      // version is required, as in the Rust parser
      '{"root":{"match":"all","rules":[]}}',
      '{"version":"1","root":{"match":"all","rules":[]}}',
      '{"version":1,"root":{"rules":[{"kind":"condition","field":"bpm",'
          '"op":"between","min":"124"}]}}',
      '{"version":1,"root":{"rules":[{"kind":"condition","field":"key",'
          '"op":"in","values":[8]}]}}',
      '{"version":1,"root":{"rules":[{"kind":"condition","field":"deck_key",'
          '"op":"equals","deck":"0"}]}}',
      '{"version":1,"root":{"rules":[{"kind":"condition","field":"deck_key",'
          '"op":"equals"}]},"sort":{"field":"bpm"}}',
    ];
    for (final json in malformed) {
      expect(
        () => SmartPlaylistRules.parse(json),
        throwsFormatException,
        reason: json,
      );
    }
  });

  test('unknown tokens are rejected instead of guessed', () {
    expect(
      () => SmartPlaylistRules.parse(
        '{"version":1,"root":{"match":"all","rules":['
        '{"kind":"condition","field":"tempo","op":"between","min":1.0}]}}',
      ),
      throwsFormatException,
    );
    expect(() => SmartPlaylistRules.parse('not json'), throwsFormatException);
  });
}
