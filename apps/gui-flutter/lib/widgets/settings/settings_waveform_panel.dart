import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';

class SettingsWaveformPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionWaveform,
          description: l10n.settingsWaveformDescription,
        ),
        SettingsSectionCard(
          title: l10n.settingsWaveformDisplayTitle,
          subtitle: l10n.settingsWaveformDisplayDescription,
          child: SettingsField(
            label: l10n.settingsWaveformDisplayMode,
            child: SettingsSelect(
              dialogTitle: l10n.settingsWaveformDisplayMode,
              value: draft.waveformDisplayMode,
              options: WaveformDisplayModeSetting.values,
              labelBuilder: (m) => switch (m) {
                WaveformDisplayModeSetting.rgb => l10n.settingsWaveformModeRgb,
                WaveformDisplayModeSetting.filtered =>
                  l10n.settingsWaveformModeFiltered,
              },
              onChanged: (m) =>
                  onChanged(copyAppSettings(draft, waveformDisplayMode: m)),
            ),
          ),
        ),
      ],
    );
  }
}
