import 'package:flutter/widgets.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar checkbox over [ShadCheckbox].
class MixarCheckbox extends StatelessWidget {
  const new({
    required this.value,
    super.key,
    this.onChanged,
    this.enabled = true,
    this.label,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;
  final bool enabled;
  final Widget? label;

  @override
  Widget build(BuildContext context) {
    return ShadCheckbox(
      value: value,
      enabled: enabled && onChanged != null,
      onChanged: onChanged,
      label: label,
    );
  }
}
