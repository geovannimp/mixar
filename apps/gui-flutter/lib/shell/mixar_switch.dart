import 'package:flutter/widgets.dart';
import 'package:shadcn_ui/shadcn_ui.dart';

/// Mixar switch over [ShadSwitch].
class MixarSwitch extends StatelessWidget {
  const new({
    required this.value,
    super.key,
    this.onChanged,
    this.enabled = true,
    this.label,
    this.semanticsLabel,
  });

  final bool value;
  final ValueChanged<bool>? onChanged;
  final bool enabled;
  final Widget? label;
  final String? semanticsLabel;

  @override
  Widget build(BuildContext context) {
    final switcher = ShadSwitch(
      value: value,
      enabled: enabled && onChanged != null,
      onChanged: onChanged,
      label: label,
    );
    if (semanticsLabel == null) {
      return switcher;
    }
    return Semantics(label: semanticsLabel, toggled: value, child: switcher);
  }
}
