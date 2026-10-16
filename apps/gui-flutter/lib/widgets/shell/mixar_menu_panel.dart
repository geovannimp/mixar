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
    return SizedBox(
      width: minWidth,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: theme.colors.card,
          borderRadius: theme.style.borderRadius.md,
          border: Border.all(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
        ),
        // Clip rows to the rounded border, or an edge row's hover fill squares
        // off the corner and bleeds past the panel outline.
        child: ClipRRect(
          borderRadius: theme.style.borderRadius.md,
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
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
