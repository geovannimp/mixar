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
  static const _analysisModes = AnalysisDurationSetting.values;
  static const _keyDisplayModes = KeyDisplayModeSetting.values;
  static const _keyColorModes = KeyColorModeSetting.values;

  static String _normalizedStemsFormat(String format) =>
      _stemsFormats.contains(format) ? format : 'opus';

  static String _analysisLabel(
    AppLocalizations l10n,
    AnalysisDurationSetting mode,
  ) => switch (mode) {
    AnalysisDurationSetting.fast => l10n.settingsLibraryAnalysisFast,
    AnalysisDurationSetting.precise => l10n.settingsLibraryAnalysisPrecise,
    AnalysisDurationSetting.complete => l10n.settingsLibraryAnalysisComplete,
  };

  static String _analysisSubtitle(
    AppLocalizations l10n,
    AnalysisDurationSetting mode,
  ) => switch (mode) {
    AnalysisDurationSetting.fast => l10n.settingsLibraryAnalysisFastSubtitle,
    AnalysisDurationSetting.precise =>
      l10n.settingsLibraryAnalysisPreciseSubtitle,
    AnalysisDurationSetting.complete =>
      l10n.settingsLibraryAnalysisCompleteSubtitle,
  };

  static String _keyDisplayLabel(
    AppLocalizations l10n,
    KeyDisplayModeSetting mode,
  ) => switch (mode) {
    KeyDisplayModeSetting.musical => l10n.settingsLibraryKeyDisplayMusical,
    KeyDisplayModeSetting.camelot => l10n.settingsLibraryKeyDisplayCamelot,
  };

  static String _keyDisplaySubtitle(
    AppLocalizations l10n,
    KeyDisplayModeSetting mode,
  ) => switch (mode) {
    KeyDisplayModeSetting.musical =>
      l10n.settingsLibraryKeyDisplayMusicalSubtitle,
    KeyDisplayModeSetting.camelot =>
      l10n.settingsLibraryKeyDisplayCamelotSubtitle,
  };

  static String _keyColorLabel(
    AppLocalizations l10n,
    KeyColorModeSetting mode,
  ) => switch (mode) {
    KeyColorModeSetting.off => l10n.settingsLibraryKeyColorOff,
    KeyColorModeSetting.absolute => l10n.settingsLibraryKeyColorAbsolute,
    KeyColorModeSetting.harmonic => l10n.settingsLibraryKeyColorHarmonic,
  };

  static String _keyColorSubtitle(
    AppLocalizations l10n,
    KeyColorModeSetting mode,
  ) => switch (mode) {
    KeyColorModeSetting.off => l10n.settingsLibraryKeyColorOffSubtitle,
    KeyColorModeSetting.absolute =>
      l10n.settingsLibraryKeyColorAbsoluteSubtitle,
    KeyColorModeSetting.harmonic =>
      l10n.settingsLibraryKeyColorHarmonicSubtitle,
  };

  static String _stemsFormatLabel(AppLocalizations l10n, String format) =>
      switch (format) {
        'flac' => l10n.settingsLibraryStemFlac,
        _ => l10n.settingsLibraryStemOpus,
      };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;

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
            dialogTitle: l10n.settingsLibraryAnalysisQuality,
            value: draft.analysisDuration,
            options: _analysisModes,
            labelBuilder: (mode) => _analysisLabel(l10n, mode),
            subtitleBuilder: (mode) => _analysisSubtitle(l10n, mode),
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
                  dialogTitle: l10n.settingsLibraryKeyDisplayMode,
                  value: draft.keyDisplayMode,
                  options: _keyDisplayModes,
                  labelBuilder: (mode) => _keyDisplayLabel(l10n, mode),
                  subtitleBuilder: (mode) => _keyDisplaySubtitle(l10n, mode),
                  onChanged: (m) =>
                      onChanged(copyAppSettings(draft, keyDisplayMode: m)),
                ),
              ),
              SettingsField(
                label: l10n.settingsLibraryKeyColorMode,
                child: SettingsSelect(
                  dialogTitle: l10n.settingsLibraryKeyColorMode,
                  value: draft.keyColorMode,
                  options: _keyColorModes,
                  labelBuilder: (mode) => _keyColorLabel(l10n, mode),
                  subtitleBuilder: (mode) => _keyColorSubtitle(l10n, mode),
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
            dialogTitle: l10n.settingsLibraryStemFormat,
            value: _normalizedStemsFormat(draft.stemsFormat),
            options: _stemsFormats,
            labelBuilder: (format) => _stemsFormatLabel(l10n, format),
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
            dialogTitle: l10n.settingsLibraryTrackRowLayout,
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
