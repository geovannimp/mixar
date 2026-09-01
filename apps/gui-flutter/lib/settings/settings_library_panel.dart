import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsLibraryPanel extends StatelessWidget {
  const SettingsLibraryPanel({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const _analysisModes = [
    (
      AnalysisDurationSetting.fast,
      'Fast',
      'Analyze a short preview for quick library scans.',
    ),
    (
      AnalysisDurationSetting.precise,
      'Precise',
      'Balanced analysis for most libraries.',
    ),
    (
      AnalysisDurationSetting.complete,
      'Complete',
      'Analyze the full track (slowest, most accurate).',
    ),
  ];

  static const _keyDisplayModes = [
    (
      KeyDisplayModeSetting.musical,
      'Musical',
      'Note names in deck chip and library key column — e.g. C, Am, F#m.',
    ),
    (
      KeyDisplayModeSetting.camelot,
      'Camelot',
      'Mixed In Key codes — e.g. 8B (C major), 8A (A minor), 11B.',
    ),
  ];

  static const _keyColorModes = [
    (
      KeyColorModeSetting.off,
      'Off',
      'Key labels use the default text color everywhere.',
    ),
    (
      KeyColorModeSetting.absolute,
      'Absolute (circle of fifths)',
      'Fixed color per key on the wheel — majors vivid, minors muted (e.g. 8B bright, 8A softer).',
    ),
    (
      KeyColorModeSetting.harmonic,
      'Harmonic (playing deck)',
      'Green/yellow vs the playing deck — e.g. with 2A playing, 1A/2A/3A/2B green, 1B/3B yellow.',
    ),
  ];

  static String _labelFor(AnalysisDurationSetting mode) => switch (mode) {
    AnalysisDurationSetting.fast => 'Fast',
    AnalysisDurationSetting.precise => 'Precise',
    AnalysisDurationSetting.complete => 'Complete',
  };

  static String _keyDisplayLabel(KeyDisplayModeSetting mode) =>
      _keyDisplayModes.firstWhere((m) => m.$1 == mode).$2;

  static String _keyColorLabel(KeyColorModeSetting mode) =>
      _keyColorModes.firstWhere((m) => m.$1 == mode).$2;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'Library',
          description: 'Track import, offline analysis, and table display.',
        ),
        SettingsField(
          label: 'Analysis quality',
          child: SettingsSelect(
            value: draft.analysisDuration,
            options: [for (final (mode, _, _) in _analysisModes) mode],
            labelBuilder: _labelFor,
            subtitleBuilder: (mode) =>
                _analysisModes.firstWhere((m) => m.$1 == mode).$3,
            onChanged: (mode) =>
                onChanged(copyAppSettings(draft, analysisDuration: mode)),
          ),
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              const SettingsSectionHeader(
                title: 'Musical key',
                description:
                    'How keys are labeled and color-coded in deck chrome and the library table.',
              ),
              SettingsField(
                label: 'Key display mode',
                child: SettingsSelect(
                  value: draft.keyDisplayMode,
                  options: [for (final (mode, _, _) in _keyDisplayModes) mode],
                  labelBuilder: _keyDisplayLabel,
                  subtitleBuilder: (mode) =>
                      _keyDisplayModes.firstWhere((m) => m.$1 == mode).$3,
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, keyDisplayMode: m)),
                ),
              ),
              SettingsField(
                label: 'Key color mode',
                child: SettingsSelect(
                  value: draft.keyColorMode,
                  options: [for (final (mode, _, _) in _keyColorModes) mode],
                  labelBuilder: _keyColorLabel,
                  subtitleBuilder: (mode) =>
                      _keyColorModes.firstWhere((m) => m.$1 == mode).$3,
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, keyColorMode: m)),
                ),
              ),
            ],
          ),
        ),
        SettingsToggle(
          label: 'Dim played tracks',
          value: draft.dimPlayedTracks,
          onChanged: (enabled) => onChanged(
            copyAppSettings(draft, dimPlayedTracks: enabled),
          ),
        ),
        SettingsField(
          label: 'Track table columns',
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: Border.all(color: theme.colors.border),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                children: [
                  for (final col in kLibraryColumnDefs)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 4),
                      child: Row(
                        children: [
                          FCheckbox(
                            value:
                                col.required ||
                                draft.libraryTableColumns.contains(col.id),
                            onChange: col.required
                                ? null
                                : (checked) {
                                    final next = List<String>.from(
                                      draft.libraryTableColumns,
                                    );
                                    if (checked) {
                                      if (!next.contains(col.id)) {
                                        next.add(col.id);
                                      }
                                    } else {
                                      next.remove(col.id);
                                    }
                                    onChanged(
                                      copyAppSettings(
                                        draft,
                                        libraryTableColumns: next,
                                      ),
                                    );
                                  },
                          ),
                          const SizedBox(width: 8),
                          Text(col.label, style: theme.typography.body.sm),
                        ],
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }
}
