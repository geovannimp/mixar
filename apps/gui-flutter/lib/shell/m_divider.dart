import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Hairline separator (Forui divider visual parity).
class MDivider extends StatelessWidget {
  const new({
    this.axis = Axis.horizontal,
    this.padding,
    this.color,
    this.thickness,
    super.key,
  });

  final Axis axis;
  final EdgeInsets? padding;
  final Color? color;
  final double? thickness;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final pad =
        padding ??
        (axis == Axis.horizontal
            ? const EdgeInsets.symmetric(vertical: 16)
            : const EdgeInsets.symmetric(horizontal: 16));
    final width = thickness ?? theme.style.borderWidth;
    return Container(
      margin: pad,
      color: color ?? theme.colors.secondary,
      height: axis == Axis.horizontal ? width : null,
      width: axis == Axis.horizontal ? null : width,
    );
  }
}
