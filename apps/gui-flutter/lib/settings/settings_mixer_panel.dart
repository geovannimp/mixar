import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsMixerPanel extends StatelessWidget {
  const new({required this.draft, required this.onChanged, super.key});

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final l10n = AppLocalizations.of(context)!;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        SettingsSectionHeader(
          title: l10n.settingsSectionMixer,
          description: l10n.settingsMixerDescription,
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsToggle(
                label: l10n.settingsMixerVolumeNormalizer,
                labelStyle: theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w600,
                ),
                value: draft.volumeNormalizerEnabled,
                onChanged: (v) => onChanged(
                  copyAppSettings(draft, volumeNormalizerEnabled: v),
                ),
              ),
              if (draft.volumeNormalizerEnabled)
                SettingsField(
                  label: l10n.settingsMixerTargetLufs,
                  child: _NumericStepper(
                    value: draft.targetLufs,
                    min: kMinTargetLufs,
                    max: kMaxTargetLufs,
                    step: 0.5,
                    format: (v) => v.toStringAsFixed(1),
                    onChanged: (v) =>
                        onChanged(copyAppSettings(draft, targetLufs: v)),
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

class _NumericStepper extends StatelessWidget {
  const new({
    required this.value,
    required this.min,
    required this.max,
    required this.step,
    required this.onChanged,
    this.format,
  });

  final double value;
  final double min;
  final double max;
  final double step;
  final ValueChanged<double> onChanged;
  final String Function(double value)? format;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final label = format?.call(value) ?? value.toString();
    final atMin = value <= min + step / 2;
    final atMax = value >= max - step / 2;
    return Row(
      children: [
        AppButton(
          variant: .outline,
          size: .sm,
          onPress: atMin
              ? null
              : () => onChanged((value - step).clamp(min, max)),
          child: const Text('−'),
        ),
        const SizedBox(width: 12),
        Text(
          label,
          style: theme.typography.body.sm.copyWith(
            fontFeatures: const [FontFeature.tabularFigures()],
          ),
        ),
        const SizedBox(width: 12),
        AppButton(
          variant: .outline,
          size: .sm,
          onPress: atMax
              ? null
              : () => onChanged((value + step).clamp(min, max)),
          child: const Text('+'),
        ),
      ],
    );
  }
}
