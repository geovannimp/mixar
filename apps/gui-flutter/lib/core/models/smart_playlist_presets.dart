import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/models/smart_playlist_rules.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

/// A named rule document the user saved to reuse.
class SmartPreset {
  const new({required this.name, required this.rules});

  final String name;
  final SmartPlaylistRules rules;

  Map<String, Object?> toJson() => {
    'name': name,
    'rules': jsonDecode(rules.toJsonString()),
  };
}

/// Preset list stored in the settings blob, best-effort.
///
/// The host keeps the blob opaque, so this is the only place that knows the
/// shape: entries without a usable name or rules are dropped instead of
/// failing the whole list.
List<SmartPreset> parseSmartPresets(String json) {
  if (json.trim().isEmpty) {
    return const [];
  }
  final Object? decoded;
  try {
    decoded = jsonDecode(json);
  } on FormatException {
    return const [];
  }
  if (decoded is! List) {
    return const [];
  }
  final presets = <SmartPreset>[];
  for (final entry in decoded) {
    if (entry is! Map) {
      continue;
    }
    final name = entry['name'];
    final rules = entry['rules'];
    if (name is! String || name.trim().isEmpty || rules == null) {
      continue;
    }
    try {
      presets.add(
        SmartPreset(
          name: name.trim(),
          rules: SmartPlaylistRules.parse(jsonEncode(rules)),
        ),
      );
    } on FormatException {
      continue;
    }
  }
  return presets;
}

/// Canonical blob for the settings field.
String encodeSmartPresets(List<SmartPreset> presets) =>
    jsonEncode([for (final preset in presets) preset.toJson()]);

/// Presets the user saved, in the order they were added.
///
/// Falls back to the empty list while settings load, so the editor always has a
/// list to render.
final smartPlaylistPresetsProvider = Provider<List<SmartPreset>>((ref) {
  final json = ref
      .watch(appSettingsProvider)
      .maybeWhen(
        data: (settings) => settings.smartPlaylistPresetsJson,
        orElse: () => '',
      );
  return parseSmartPresets(json);
});

/// Writes one preset into the settings blob, replacing a preset with the same
/// name. Overridable so widget tests can capture the write.
typedef SmartPresetSaver = Future<void> Function(SmartPreset preset);

final smartPresetSaverProvider = Provider<SmartPresetSaver>((ref) {
  return (preset) async {
    final settings = await ref.read(settingsTransportProvider.future);
    final current = await ref.read(appSettingsProvider.future);
    final presets = withSavedPreset(
      parseSmartPresets(current.smartPlaylistPresetsJson),
      preset,
    );
    await settings.saveSettings(
      settings: saveWithPresets(current, encodeSmartPresets(presets)),
    );
    ref.invalidate(appSettingsProvider);
  };
});

/// [presets] with [preset] appended, replacing a preset of the same name.
///
/// The saved preset always lands last so the newest one is closest to the
/// picker's default position; the others keep their order.
List<SmartPreset> withSavedPreset(
  List<SmartPreset> presets,
  SmartPreset preset,
) => [
  for (final existing in presets)
    if (existing.name != preset.name) existing,
  preset,
];

/// [base] with only the preset blob replaced.
AppSettings saveWithPresets(AppSettings base, String presetsJson) =>
    copyAppSettings(base, smartPlaylistPresetsJson: presetsJson);
