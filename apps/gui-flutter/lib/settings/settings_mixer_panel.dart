import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_field.dart';
import 'package:gui_flutter/settings/settings_widgets.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';

class SettingsMixerPanel extends StatelessWidget {
  const SettingsMixerPanel({
    super.key,
    required this.draft,
    required this.onChanged,
  });

  final AppSettings draft;
  final ValueChanged<AppSettings> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 16,
      children: [
        const SettingsSectionHeader(
          title: 'Mixer',
          description:
              'Keep analyzed tracks near a consistent perceived loudness.',
        ),
        SettingsPanel(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 16,
            children: [
              SettingsToggle(
                label: 'Volume normalizer',
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
                  label: 'Target LUFS',
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
  const _NumericStepper({
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
        FButton(
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
        FButton(
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
