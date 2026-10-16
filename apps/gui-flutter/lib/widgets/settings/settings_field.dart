import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class SettingsField extends StatelessWidget {
  const new({
    required this.label,
    required this.child,
    super.key,
    this.hint,
    this.trailing,
  });

  final String label;
  final String? hint;
  final Widget? trailing;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final labelText = Text(
      label.toUpperCase(),
      style: theme.typography.body.xs.copyWith(
        color: theme.colors.mutedForeground,
        fontWeight: FontWeight.w600,
        letterSpacing: 0.8,
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (trailing == null)
          labelText
        else
          Row(
            children: [
              Expanded(child: labelText),
              trailing!,
            ],
          ),
        const SizedBox(height: 6),
        child,
        if (hint != null) ...[
          const SizedBox(height: 4),
          Text(
            hint!,
            style: theme.typography.body.xs.copyWith(
              color: theme.colors.mutedForeground,
            ),
          ),
        ],
      ],
    );
  }
}
