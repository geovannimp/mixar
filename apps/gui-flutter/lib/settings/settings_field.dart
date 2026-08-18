import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

class SettingsSectionHeader extends StatelessWidget {
  const SettingsSectionHeader({
    super.key,
    required this.title,
    required this.description,
  });

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 4,
      children: [
        Text(
          title,
          style: theme.typography.body.md.copyWith(fontWeight: FontWeight.w600),
        ),
        Text(
          description,
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.mutedForeground,
          ),
        ),
      ],
    );
  }
}

class SettingsField extends StatelessWidget {
  const SettingsField({
    super.key,
    required this.label,
    this.hint,
    this.trailing,
    required this.child,
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
      crossAxisAlignment: CrossAxisAlignment.start,
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
