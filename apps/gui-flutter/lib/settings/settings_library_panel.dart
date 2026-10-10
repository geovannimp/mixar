import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/mixxx_import_panel.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsLibraryPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const _stemsFormats = ['opus', 'flac'];

  static String _normalizedStemsFormat(String format) =>
      _stemsFormats.contains(format) ? format : 'opus';

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final analysisModes = <(AnalysisDurationSetting, String, String)>[
      (
        AnalysisDurationSetting.fast,
        l10n.settingsLibraryAnalysisFast,
        l10n.settingsLibraryAnalysisFastSubtitle,
      ),
      (
        AnalysisDurationSetting.precise,
        l10n.settingsLibraryAnalysisPrecise,
        l10n.settingsLibraryAnalysisPreciseSubtitle,
      ),
      (
        AnalysisDurationSetting.complete,
        l10n.settingsLibraryAnalysisComplete,
        l10n.settingsLibraryAnalysisCompleteSubtitle,
      ),
    ];
    final keyDisplayModes = <(KeyDisplayModeSetting, String, String)>[
      (
        KeyDisplayModeSetting.musical,
        l10n.settingsLibraryKeyDisplayMusical,
        l10n.settingsLibraryKeyDisplayMusicalSubtitle,
      ),
      (
        KeyDisplayModeSetting.camelot,
        l10n.settingsLibraryKeyDisplayCamelot,
        l10n.settingsLibraryKeyDisplayCamelotSubtitle,
      ),
    ];
    final keyColorModes = <(KeyColorModeSetting, String, String)>[
      (
        KeyColorModeSetting.off,
        l10n.settingsLibraryKeyColorOff,
        l10n.settingsLibraryKeyColorOffSubtitle,
      ),
      (
        KeyColorModeSetting.absolute,
        l10n.settingsLibraryKeyColorAbsolute,
        l10n.settingsLibraryKeyColorAbsoluteSubtitle,
      ),
      (
        KeyColorModeSetting.harmonic,
        l10n.settingsLibraryKeyColorHarmonic,
        l10n.settingsLibraryKeyColorHarmonicSubtitle,
      ),
    ];

    String analysisLabel(AnalysisDurationSetting mode) =>
        analysisModes.firstWhere((m) => m.$1 == mode).$2;
    String keyDisplayLabel(KeyDisplayModeSetting mode) =>
        keyDisplayModes.firstWhere((m) => m.$1 == mode).$2;
    String keyColorLabel(KeyColorModeSetting mode) =>
        keyColorModes.firstWhere((m) => m.$1 == mode).$2;
    String stemsFormatLabel(String format) => switch (format) {
      'flac' => l10n.settingsLibraryStemFlac,
      _ => l10n.settingsLibraryStemOpus,
    };

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionLibrary,
          description: l10n.settingsLibraryDescription,
        ),
        SettingsField(
          label: l10n.settingsLibraryAnalysisQuality,
          child: SettingsSelect(
            dialogTitle: 'Analysis quality',
            value: draft.analysisDuration,
            options: [for (final (mode, _, _) in analysisModes) mode],
            labelBuilder: analysisLabel,
            subtitleBuilder: (mode) =>
                analysisModes.firstWhere((m) => m.$1 == mode).$3,
            onChanged: (mode) =>
                onChanged(copyAppSettings(draft, analysisDuration: mode)),
          ),
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsSectionHeader(
                title: l10n.settingsLibraryMusicalKeyTitle,
                description: l10n.settingsLibraryMusicalKeyDescription,
              ),
              SettingsField(
                label: l10n.settingsLibraryKeyDisplayMode,
                child: SettingsSelect(
                  dialogTitle: 'Key display mode',
                  value: draft.keyDisplayMode,
                  options: [for (final (mode, _, _) in keyDisplayModes) mode],
                  labelBuilder: keyDisplayLabel,
                  subtitleBuilder: (mode) =>
                      keyDisplayModes.firstWhere((m) => m.$1 == mode).$3,
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, keyDisplayMode: m)),
                ),
              ),
              SettingsField(
                label: l10n.settingsLibraryKeyColorMode,
                child: SettingsSelect(
                  dialogTitle: 'Key color mode',
                  value: draft.keyColorMode,
                  options: [for (final (mode, _, _) in keyColorModes) mode],
                  labelBuilder: keyColorLabel,
                  subtitleBuilder: (mode) =>
                      keyColorModes.firstWhere((m) => m.$1 == mode).$3,
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, keyColorMode: m)),
                ),
              ),
            ],
          ),
        ),
        SettingsField(
          label: l10n.settingsLibraryStemFormat,
          child: SettingsSelect<String>(
            dialogTitle: 'Stem format',
            value: _normalizedStemsFormat(draft.stemsFormat),
            options: _stemsFormats,
            labelBuilder: stemsFormatLabel,
            onChanged: (v) => onChanged(copyAppSettings(draft, stemsFormat: v)),
          ),
        ),
        SettingsToggle(
          label: l10n.settingsLibraryDimPlayedTracks,
          value: draft.dimPlayedTracks,
          onChanged: (enabled) =>
              onChanged(copyAppSettings(draft, dimPlayedTracks: enabled)),
        ),
        SettingsField(
          label: l10n.settingsLibraryTrackRowLayout,
          child: SettingsSelect<LibraryRowDensitySetting>(
            dialogTitle: 'Track row layout',
            value: draft.libraryRowDensity,
            options: const [
              LibraryRowDensitySetting.comfortable,
              LibraryRowDensitySetting.compact,
            ],
            labelBuilder: (mode) => mode == LibraryRowDensitySetting.compact
                ? l10n.settingsLibraryRowCompact
                : l10n.settingsLibraryRowComfortable,
            subtitleBuilder: (mode) => mode == LibraryRowDensitySetting.compact
                ? l10n.settingsLibraryRowCompactSubtitle
                : l10n.settingsLibraryRowComfortableSubtitle,
            onChanged: (mode) =>
                onChanged(copyAppSettings(draft, libraryRowDensity: mode)),
          ),
        ),
        const MixxxImportPanel(),
      ],
    );
  }
}
