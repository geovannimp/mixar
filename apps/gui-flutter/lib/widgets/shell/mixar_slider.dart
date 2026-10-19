import 'package:flutter/widgets.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar slider over [ShadSlider].
class MixarSlider extends StatelessWidget {
  const new({
    required this.value,
    required this.onChanged,
    super.key,
    this.min = 0,
    this.max = 1,
    this.divisions,
    this.enabled = true,
    this.semanticFormatterCallback,
    this.trackHeight,
    this.thumbRadius,
  });

  final double value;
  final ValueChanged<double> onChanged;
  final double min;
  final double max;
  final int? divisions;
  final bool enabled;
  final String Function(double value)? semanticFormatterCallback;

  /// Track thickness; `null` uses the theme default (8).
  final double? trackHeight;

  /// Thumb radius; `null` uses the theme default (10). Compact rows pass
  /// smaller values so the thumb stops dominating the strip.
  final double? thumbRadius;

  @override
  Widget build(BuildContext context) {
    return ShadSlider(
      initialValue: value.clamp(min, max),
      min: min,
      max: max,
      divisions: divisions,
      enabled: enabled,
      trackHeight: trackHeight,
      thumbRadius: thumbRadius,
      onChanged: onChanged,
      semanticFormatterCallback: semanticFormatterCallback,
    );
  }
}
