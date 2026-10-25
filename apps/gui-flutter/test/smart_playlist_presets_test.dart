import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gui_flutter/core/models/smart_playlist_presets.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';

const _rules = SmartPlaylistRules(
  root: SmartGroupRule(
    rules: [
      SmartConditionRule(
        field: SmartField.bpm,
        op: SmartOp.between,
        min: 120,
        max: 130,
      ),
    ],
  ),
);

const _house = SmartPreset(name: 'House', rules: _rules);
const _techno = SmartPreset(name: 'Techno', rules: _rules);

void main() {
  test('a blank, broken or non-list blob reads as no presets', () {
    expect(parseSmartPresets(''), isEmpty);
    expect(parseSmartPresets('   '), isEmpty);
    expect(parseSmartPresets('{not json'), isEmpty);
    expect(parseSmartPresets('{"name":"House"}'), isEmpty);
    expect(parseSmartPresets('[1, "House", null]'), isEmpty);
  });

  test('entries without a usable name or rules are dropped', () {
    final blob = jsonEncode([
      {'name': '   ', 'rules': jsonDecode(_rules.toJsonString())},
      {'rules': jsonDecode(_rules.toJsonString())},
      {'name': 'No rules'},
      {
        'name': 'Broken rules',
        'rules': {'root': 'nope'},
      },
      {'name': ' Keep me ', 'rules': jsonDecode(_rules.toJsonString())},
    ]);

    final presets = parseSmartPresets(blob);

    expect(presets.map((preset) => preset.name), ['Keep me']);
    expect(presets.single.rules.toJsonString(), _rules.toJsonString());
  });

  test('encode round-trips through parse', () {
    final parsed = parseSmartPresets(
      encodeSmartPresets(const [_house, _techno]),
    );

    expect(parsed.map((preset) => preset.name), ['House', 'Techno']);
    expect(parsed.first.rules.toJsonString(), _rules.toJsonString());
  });

  test('saving keeps the others in order and moves the saved one last', () {
    final replaced = withSavedPreset(const [_house, _techno], _house);
    expect(replaced.map((preset) => preset.name), ['Techno', 'House']);

    const peak = SmartPreset(name: 'Peak', rules: _rules);
    final appended = withSavedPreset(const [_house, _techno], peak);
    expect(appended.map((preset) => preset.name), ['House', 'Techno', 'Peak']);
  });
}
