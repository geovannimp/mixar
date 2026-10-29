import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/settings/settings_section_header.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Settings card body: required title + subtitle above optional [child].
///
/// Pass [trailing] for header actions (e.g. Update All, enable toggles). Omit
/// [child] when the card is header-only (toggle off, no fields to show).
///
/// Full-bleed band: no side chrome, only a top and bottom hairline, so cards
/// stack as a striped list rather than separate boxes. It already owns its
/// edges — do not wrap it in `MCard`, which would double the hairlines.
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
    final theme = context.theme;
    final header = SettingsSectionHeader(
      title: title,
      description: subtitle,
      descriptionBuilder: subtitleBuilder,
    );
    final border = BorderSide(
      color: theme.colors.border,
      width: theme.style.borderWidth,
    );
    // Foreground border: the card fill would otherwise paint over the stroke,
    // and it costs no layout space between stacked cards.
    return DecoratedBox(
      position: DecorationPosition.foreground,
      decoration: BoxDecoration(
        border: Border(top: border, bottom: border),
      ),
      child: ColoredBox(
        color: theme.colors.card,
        child: Padding(
          padding: const EdgeInsets.all(16),
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
      ),
    );
  }
}
