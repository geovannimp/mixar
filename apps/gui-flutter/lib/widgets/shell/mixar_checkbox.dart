import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
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
    final theme = context.theme;
    return ShadCheckbox(
      value: value,
      enabled: enabled && onChanged != null,
      onChanged: onChanged,
      label: label,
      // Shad's stock outline is `primary` in both states, so an empty box
      // reads as already on beside the quiet hairline of every other field.
      // Unchecked is a hollow box on that hairline, matching the unfilled
      // inputs it sits beside; the fill and outline still turn primary once
      // it is on.
      uncheckedColor: theme.colors.card,
      decoration: ShadDecoration(
        border: ShadBorder.all(
          color: value ? theme.colors.primary : theme.colors.border,
          width: theme.style.borderWidth,
          radius: theme.style.borderRadius.md,
        ),
      ),
    );
  }
}
