import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Rounded, hairline-bordered card with an accent wash that is strongest at
/// its leading edge and reaches the card colour by half its width.
///
/// Shared by the toast card and the onboarding closing note, so both wear one
/// decoration that cannot drift apart when either is retuned.
class MixarWashCard extends StatelessWidget {
  const new({
    required this.accent,
    required this.child,
    this.padding = EdgeInsets.zero,
    super.key,
  });

  /// Colour of the wash. It is blended over the card surface, never painted
  /// raw.
  final Color accent;
  final Widget child;
  final EdgeInsetsGeometry padding;

  /// Strength of the wash at the leading edge.
  static const washAlpha = 0.16;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    final radius = theme.style.borderRadius.dialog;
    return DecoratedBox(
      // Foreground: the content runs full-bleed, so the outline has to paint
      // above it to stay visible at the rounded corners.
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        borderRadius: radius,
        border: Border.all(
          color: colors.border,
          width: theme.style.borderWidth,
        ),
      ),
      child: ClipRRect(
        borderRadius: radius,
        child: DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              colors: [
                Color.alphaBlend(
                  accent.withValues(alpha: washAlpha),
                  colors.card,
                ),
                colors.card,
              ],
              stops: const [0, 0.5],
            ),
          ),
          child: Padding(padding: padding, child: child),
        ),
      ),
    );
  }
}
