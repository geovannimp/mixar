import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Shared panel chrome for Mixar menus and content popovers.
///
/// Fixed [minWidth] — Anchor overlays get viewport-max constraints, so a
/// min-only [ConstrainedBox] + [Expanded] chips expands to full screen.
class MixarMenuPanel extends StatelessWidget {
  const new({required this.child, this.minWidth = 180, super.key});

  final Widget child;
  final double minWidth;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    // Same shell radius as the mobile menu dialog, so a dropdown reads as the
    // same surface in both styles.
    final radius = theme.style.borderRadius.dialog;
    return SizedBox(
      width: minWidth,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: theme.colors.card,
          borderRadius: radius,
        ),
        child: DecoratedBox(
          // Stroke in the foreground: flush rows paint edge to edge, so a
          // background border would be covered by the hover fill.
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(
            borderRadius: radius,
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
          // Clip rows to the rounded border, or an edge row's hover fill
          // squares off the corner and bleeds past the panel outline.
          child: ClipRRect(
            borderRadius: radius,
            // No vertical padding: rows are flush so a hovered first/last row
            // paints to the panel edge instead of leaving a strip of card.
            child: DefaultTextStyle(
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.foreground,
              ),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}
