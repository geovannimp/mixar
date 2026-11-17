import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Uppercase group label used inside a step's content column.
class OnboardingGroupLabel extends StatelessWidget {
  const new({required this.label, super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Text(
      label.toUpperCase(),
      style: theme.typography.mono.xs.copyWith(
        color: theme.colors.mutedForeground,
        letterSpacing: 1.8,
        height: 1,
      ),
    );
  }
}
