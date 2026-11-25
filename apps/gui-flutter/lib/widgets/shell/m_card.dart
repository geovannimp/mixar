import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Bordered panel chrome (Mixar card colors; square corner language).
class MCard extends StatelessWidget {
  const new({
    required this.child,
    this.clipBehavior = Clip.none,
    this.borderColor,
    this.outlineInset = EdgeInsets.zero,
    this.sideBorders = true,
    this.topBorder = true,
    super.key,
  });

  final Widget child;
  final Clip clipBehavior;

  /// When set, replaces the default hairline color and closes all four edges
  /// (e.g. the deck's drop-hover outline), regardless of [sideBorders].
  final Color? borderColor;

  /// Insets the closed outline [borderColor] draws, away from the card's
  /// edges. The resting hairlines stay flush. Use it where the card runs into
  /// chrome painted over its edge — the window frame along a deck's outer
  /// edge, say — so the outline is not cropped there.
  final EdgeInsetsGeometry outlineInset;

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
    // the resting card may sit flush with only its horizontal rules. The
    // overridden outline moves to the inset foreground overlay below; painting
    // it here as well would double it.
    final Border? border;
    if (borderColor != null) {
      border = null;
    } else if (sideBorders) {
      border = Border.all(color: color, width: borderW);
    } else {
      border = Border(
        top: topBorder
            ? BorderSide(color: color, width: borderW)
            : BorderSide.none,
        bottom: BorderSide(color: color, width: borderW),
      );
    }
    final card = DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.card,
        border: border,
        borderRadius: radius,
      ),
      // Inset so edge-flush fills (cover art, waveforms) cannot paint over the
      // hairline — DecoratedBox draws the stroke behind its child.
      child: Padding(padding: EdgeInsets.all(borderW), child: content),
    );
    // The Stack is always the root so toggling the interaction outline does
    // not remount the card body (and lose e.g. a scroll position). Passthrough
    // keeps the parent's constraints on the card: a shrink-wrapping card must
    // keep its own size, not just let the overlay span a larger box.
    return Stack(
      fit: StackFit.passthrough,
      clipBehavior: Clip.none,
      children: [
        card,
        if (borderColor != null)
          Positioned.fill(
            child: IgnorePointer(
              child: Padding(
                padding: outlineInset,
                child: DecoratedBox(
                  key: const ValueKey('m-card-outline'),
                  decoration: BoxDecoration(
                    border: Border.all(color: color, width: borderW),
                    borderRadius: radius,
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}
