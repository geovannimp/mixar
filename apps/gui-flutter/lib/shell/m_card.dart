import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Bordered panel chrome (Forui card visual parity).
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
        : ClipRSuperellipse(
            borderRadius: radius,
            clipBehavior: clipBehavior,
            child: child,
          );
    return DecoratedBox(
      decoration: ShapeDecoration(
        color: theme.colors.card,
        shape: RoundedSuperellipseBorder(
          side: BorderSide(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
          borderRadius: radius,
        ),
      ),
      child: content,
    );
  }
}
