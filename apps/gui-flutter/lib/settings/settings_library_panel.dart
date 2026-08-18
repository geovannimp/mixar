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

  static String _labelFor(AnalysisDurationSetting mode) => switch (mode) {
    AnalysisDurationSetting.fast => 'Fast',
    AnalysisDurationSetting.precise => 'Precise',
    AnalysisDurationSetting.complete => 'Complete',
  };

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const SettingsSectionHeader(
          title: 'Library',
          description: 'Track import and offline analysis behavior.',
        ),
        const SizedBox(height: 20),
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
        const SizedBox(height: 16),
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
