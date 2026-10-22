import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/settings/settings_field.dart';
import 'package:gui_flutter/widgets/settings/settings_header_switch.dart';
import 'package:gui_flutter/widgets/settings/settings_numeric_stepper.dart';
import 'package:gui_flutter/widgets/settings/settings_section_card.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';

class SettingsMixerPanel extends StatelessWidget {
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
          title: l10n.settingsSectionMixer,
          description: l10n.settingsMixerDescription,
        ),
        SettingsSectionCard(
          title: l10n.settingsMixerNormalizerTitle,
          subtitle: l10n.settingsMixerNormalizerDescription,
          trailing: SettingsHeaderSwitch(
            value: draft.volumeNormalizerEnabled,
            semanticsLabel: l10n.settingsMixerVolumeNormalizer,
            onChanged: (v) =>
                onChanged(copyAppSettings(draft, volumeNormalizerEnabled: v)),
          ),
          child: draft.volumeNormalizerEnabled
              ? SettingsField(
                  label: l10n.settingsMixerTargetLufs,
                  child: SettingsNumericStepper(
                    value: draft.targetLufs,
                    min: kMinTargetLufs,
                    max: kMaxTargetLufs,
                    step: 0.5,
                    format: (v) => v.toStringAsFixed(1),
                    onChanged: (v) =>
                        onChanged(copyAppSettings(draft, targetLufs: v)),
                  ),
                )
              : null,
        ),
        SettingsSectionCard(
          title: l10n.settingsMixerRealtimeStemsTitle,
          subtitle: l10n.settingsMixerRealtimeStemsDescription,
          subtitleBuilder: (style) => Text.rich(
            TextSpan(
              children: [
                TextSpan(text: l10n.settingsMixerRealtimeStemsDescription),
                TextSpan(
                  text: '\n${l10n.settingsMixerRealtimeStemsNotRecommended}',
                  style: style.copyWith(fontWeight: FontWeight.w700),
                ),
                TextSpan(text: ' ${l10n.settingsMixerRealtimeStemsWarning}'),
              ],
            ),
            style: style,
          ),
          trailing: SettingsHeaderSwitch(
            value: draft.realtimeStems,
            semanticsLabel: l10n.settingsMixerRealtimeStemsTitle,
            onChanged: (enabled) =>
                onChanged(copyAppSettings(draft, realtimeStems: enabled)),
          ),
        ),
      ],
    );
  }
}
