import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/shell/m_card.dart';

/// Titled settings card: required title + subtitle above optional [child].
///
/// Pass [trailing] for header actions (e.g. Update All, enable toggles). Omit
/// [child] when the card is header-only (toggle off, no fields to show).
class SettingsSectionCard extends StatelessWidget {
  const new({
    required this.title,
    this.subtitle = '',
    super.key,
    this.trailing,
    this.child,
    this.subtitleBuilder,
  });

  final String title;

  /// Muted description under the title. Empty leaves the card title-only.
  final String subtitle;

  /// Optional rich subtitle; see [SettingsSectionHeader.descriptionBuilder].
  final Widget Function(TextStyle style)? subtitleBuilder;
  final Widget? trailing;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final header = SettingsSectionHeader(
      title: title,
      description: subtitle,
      descriptionBuilder: subtitleBuilder,
    );
    return MCard(
      child: Padding(
        padding: const .fromLTRB(16, 16, 16, 16),
        child: Column(
          crossAxisAlignment: .start,
          spacing: 16,
          children: [
            if (trailing == null)
              header
            else
              Row(
                crossAxisAlignment: .start,
                spacing: 12,
                children: [
                  Expanded(child: header),
                  trailing!,
                ],
              ),
            if (child != null) child!,
          ],
        ),
      ),
    );
  }
}
