import 'package:flutter/widgets.dart';
import 'package:gui_flutter/l10n/app_localizations.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsNumericStepper extends StatelessWidget {
  const SettingsNumericStepper({
    required this.value,
    required this.min,
    required this.max,
    required this.step,
    required this.onChanged,
    this.format,
    this.fieldLabel,
    super.key,
  });

  final double value;
  final double min;
  final double max;
  final double step;
  final ValueChanged<double> onChanged;
  final String Function(double value)? format;

  /// Localized name of the field, used for the step-button semantics labels so
  /// screen readers announce "Decrease Stop time" instead of a bare "minus".
  final String? fieldLabel;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final locale = AppLocalizations.of(context);
    final field = fieldLabel;
    final decreaseLabel = field == null || locale == null
        ? null
        : locale.settingsStepperDecrease(field);
    final increaseLabel = field == null || locale == null
        ? null
        : locale.settingsStepperIncrease(field);
    final label = format?.call(value) ?? value.toString();
    final atMin = value <= min + step / 2;
    final atMax = value >= max - step / 2;
    return Row(
      children: [
        AppButton(
          variant: .outline,
          size: .sm,
          semanticsLabel: decreaseLabel,
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
          semanticsLabel: increaseLabel,
          onPress: atMax
              ? null
              : () => onChanged((value + step).clamp(min, max)),
          child: const Text('+'),
        ),
      ],
    );
  }
}
