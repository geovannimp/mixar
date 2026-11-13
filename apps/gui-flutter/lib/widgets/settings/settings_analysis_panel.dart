import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';

/// Settings → Analysis: how much audio is scanned, how the cue is detected.
class SettingsAnalysisPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static const _analysisModes = AnalysisDurationSetting.values;

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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;

    return SettingsSectionBody(
      title: l10n.settingsSectionAnalysis,
      description: l10n.settingsAnalysisDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsLibraryAnalysisTitle,
          subtitle: l10n.settingsLibraryAnalysisDescription,
          child: SettingsField(
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
        ),
        SettingsSectionCard(
          title: l10n.settingsAnalysisCueTitle,
          subtitle: l10n.settingsAnalysisCueDescription,
          child: SettingsField(
            label: l10n.settingsAnalysisCueThreshold,
            hint: l10n.settingsAnalysisCueThresholdHint,
            child: SettingsNumericStepper(
              value: draft.cueThresholdDb,
              min: kMinCueThresholdDb,
              max: kMaxCueThresholdDb,
              step: 1,
              format: (v) => '${v.toStringAsFixed(0)} dB',
              fieldLabel: l10n.settingsAnalysisCueThreshold,
              onChanged: (v) =>
                  onChanged(copyAppSettings(draft, cueThresholdDb: v)),
            ),
          ),
        ),
      ],
    );
  }
}
