import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Compact button for in-card actions (browse, retry).
class OnboardingSmallButton extends StatelessWidget {
  const new({required this.label, required this.icon, this.onPress, super.key});

  final String label;
  final IconData icon;
  final VoidCallback? onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return MTappable(
      onPress: onPress,
      semanticsLabel: label,
      builder: (_, state) {
        final ink = colors.primaryForeground;
        final faded = onPress == null;
        final focused = state.focused;
        return Opacity(
          opacity: faded ? colors.disabledOpacity : 1,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: colors.primary,
              // Focus has to show: the fill is the same accent the stage uses.
              border: Border.all(
                color: focused ? colors.foreground : colors.primary,
                width: focused ? 2 : theme.style.borderWidth,
              ),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 9),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                spacing: 8,
                children: [
                  Text(
                    label,
                    style: theme.typography.body.sm.copyWith(
                      color: ink,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  Icon(icon, size: 14, color: ink),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}
