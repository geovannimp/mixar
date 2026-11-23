import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Bordered panel chrome (Mixar card colors; square corner language).
class MCard extends StatelessWidget {
  const new({
    required this.child,
    this.clipBehavior = Clip.none,
    this.borderColor,
    this.sideBorders = true,
    this.topBorder = true,
    super.key,
  });

  final Widget child;
  final Clip clipBehavior;

  /// When set, replaces the default hairline color and closes all four edges
  /// (e.g. the deck's drop-hover outline), regardless of [sideBorders].
  final Color? borderColor;

  /// Whether the hairline also runs down the left and right edges. Off, the
  /// card keeps only its top and bottom rules so its content can sit flush
  /// against the surface around it.
  final bool sideBorders;

  /// Whether the card carries its top hairline. Ignored while [sideBorders]
  /// is on, which paints all four.
  final bool topBorder;

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
    final color = borderColor ?? theme.colors.border;
    // Interaction chrome (an overridden color) must read as a closed outline;
    // the resting card may sit flush with only its horizontal rules.
    final closed = borderColor != null || sideBorders;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        border: closed
            ? Border.all(color: color, width: borderW)
            : Border(
                top: topBorder
                    ? BorderSide(color: color, width: borderW)
                    : BorderSide.none,
                bottom: BorderSide(color: color, width: borderW),
              ),
        borderRadius: radius,
      ),
      // Inset so edge-flush fills (cover art, waveforms) cannot paint over the
      // hairline — DecoratedBox draws the stroke behind its child.
      child: Padding(padding: EdgeInsets.all(borderW), child: content),
    );
  }
}
