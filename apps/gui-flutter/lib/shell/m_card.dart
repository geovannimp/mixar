import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Bordered panel chrome (Forui card colors; Mixar button-style rounded rect).
class MCard extends StatelessWidget {
  const MCard({required this.child, this.clipBehavior = Clip.none, super.key});

  final Widget child;
  final Clip clipBehavior;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final radius = theme.style.borderRadius.lg;
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
          color: theme.colors.border,
          width: theme.style.borderWidth,
        ),
        borderRadius: radius,
      ),
      child: content,
    );
  }
}
