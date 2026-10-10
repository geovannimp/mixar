import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsWaveformPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionWaveform,
          description: l10n.settingsWaveformDescription,
        ),
        const SizedBox(height: 20),
        SettingsField(
          label: l10n.settingsWaveformDisplayMode,
          child: SettingsSelect(
            dialogTitle: 'Display mode',
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
      ],
    );
  }
}
