import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_section_body.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_select.dart';

/// Where the waveform sits in the mixer page (Settings → Layout).
class SettingsLayoutPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  static String _positionLabel(
    AppLocalizations l10n,
    WaveformPositionSetting value,
  ) => switch (value) {
    WaveformPositionSetting.top => l10n.settingsLayoutWaveformTop,
    WaveformPositionSetting.middleHorizontal =>
      l10n.settingsLayoutWaveformMiddleHorizontal,
    WaveformPositionSetting.middleVertical =>
      l10n.settingsLayoutWaveformMiddleVertical,
    WaveformPositionSetting.bottom => l10n.settingsLayoutWaveformBottom,
  };

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSectionBody(
      title: l10n.settingsSectionLayout,
      description: l10n.settingsLayoutDescription,
      cards: [
        SettingsSectionCard(
          title: l10n.settingsLayoutWaveformTitle,
          subtitle: l10n.settingsLayoutWaveformDescription,
          trailing: SettingsHeaderSwitch(
            value: draft.showWaveform,
            semanticsLabel: l10n.settingsLayoutShowWaveform,
            onChanged: (show) =>
                onChanged(copyAppSettings(draft, showWaveform: show)),
          ),
          // The position only matters while something is drawn.
          child: draft.showWaveform
              ? SettingsField(
                  label: l10n.settingsLayoutWaveformPosition,
                  child: SettingsSelect<WaveformPositionSetting>(
                    dialogTitle: l10n.settingsLayoutWaveformPosition,
                    value: draft.waveformPosition,
                    options: WaveformPositionSetting.values,
                    labelBuilder: (position) => _positionLabel(l10n, position),
                    onChanged: (position) => onChanged(
                      copyAppSettings(draft, waveformPosition: position),
                    ),
                  ),
                )
              : null,
        ),
      ],
    );
  }
}
