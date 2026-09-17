import 'package:flutter/widgets.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar slider over [ShadSlider].
class MixarSlider extends StatelessWidget {
  const MixarSlider({
    super.key,
    required this.value,
    required this.onChanged,
    this.min = 0,
    this.max = 1,
    this.divisions,
    this.enabled = true,
    this.semanticFormatterCallback,
  });

  final double value;
  final ValueChanged<double> onChanged;
  final double min;
  final double max;
  final int? divisions;
  final bool enabled;
  final String Function(double value)? semanticFormatterCallback;

  @override
  Widget build(BuildContext context) {
    return ShadSlider(
      initialValue: value.clamp(min, max),
      min: min,
      max: max,
      divisions: divisions,
      enabled: enabled,
      onChanged: onChanged,
      semanticFormatterCallback: semanticFormatterCallback,
    );
  }
}
