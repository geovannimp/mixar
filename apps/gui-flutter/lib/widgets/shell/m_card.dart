import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Bordered panel chrome (Mixar card colors; square corner language).
class MCard extends StatelessWidget {
  const new({
    required this.child,
    this.clipBehavior = Clip.none,
    this.borderColor,
    super.key,
  });

  final Widget child;
  final Clip clipBehavior;

  /// When set, replaces the default hairline color (e.g. drop-hover accent).
  final Color? borderColor;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final radius = theme.style.borderRadius.lg;
    final borderW = theme.style.borderWidth;
    final content = clipBehavior == Clip.none
        ? child
        : ClipRRect(
            borderRadius: radius,
            clipBehavior: clipBehavior,
            child: child,
          );
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        border: Border.all(
          color: borderColor ?? theme.colors.border,
          width: borderW,
        ),
        borderRadius: radius,
      ),
      // Inset so edge-flush fills (cover art, waveforms) cannot paint over the
      // hairline — DecoratedBox draws the stroke behind its child.
      child: Padding(padding: EdgeInsets.all(borderW), child: content),
    );
  }
}
