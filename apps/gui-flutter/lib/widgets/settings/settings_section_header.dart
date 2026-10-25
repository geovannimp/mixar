import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Titled card header: the section title above its muted description.
class SettingsSectionHeader extends StatelessWidget {
  const new({
    required this.title,
    this.description = '',
    this.descriptionBuilder,
    super.key,
  });

  final String title;

  /// Muted line under the title. Empty (the default) leaves the header
  /// title-only, for sections whose fields explain themselves.
  final String description;

  /// Optional rich description. Receives the muted body style so spans can
  /// emphasize parts (e.g. a bold warning) while matching the surrounding copy.
  final Widget Function(TextStyle style)? descriptionBuilder;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final descriptionStyle = theme.typography.body.sm.copyWith(
      color: theme.colors.mutedForeground,
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 4,
      children: [
        Text(
          title,
          style: theme.typography.body.md.copyWith(fontWeight: FontWeight.w600),
        ),
        if (descriptionBuilder case final builder?)
          builder(descriptionStyle)
        else if (description.isNotEmpty)
          Text(description, style: descriptionStyle),
      ],
    );
  }
}
