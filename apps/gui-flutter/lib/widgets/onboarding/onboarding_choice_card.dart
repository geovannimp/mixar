import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Selectable setup option: icon, label, optional detail, and a check square.
///
/// Square chrome and a hairline border match the rest of the app; selection is
/// carried by the accent border plus an accent-tinted fill, not by a shadow.
class OnboardingChoiceCard extends StatelessWidget {
  const new({
    required this.title,
    required this.onPress,
    this.subtitle,
    this.icon,
    this.selected = false,
    this.semanticsLabel,
    this.stacked = false,
    this.compact = false,
    this.trailing,
    super.key,
  });

  /// Tighter vertical padding for single-line option rows.
  final bool compact;

  final String title;
  final String? subtitle;
  final IconData? icon;
  final bool selected;
  final VoidCallback? onPress;
  final String? semanticsLabel;

  /// Icon above the text instead of beside it (compact 3-up grids).
  final bool stacked;

  /// Extra widget under the text (e.g. a live-preview strip).
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: semanticsLabel ?? title,
      builder: (_, state) {
        // A tinted fill reads as a solid block on a dark stage; an accent edge
        // plus a barely-there wash keeps the card's own surface.
        final fill = selected
            ? Color.alphaBlend(
                colors.primary.withValues(alpha: 0.10),
                colors.card,
              )
            : state.hovered
            ? colors.muted
            : colors.card;
        final borderColor = selected
            ? colors.primary
            : state.focused
            ? colors.accent
            : colors.border;
        final iconColor = selected ? colors.primary : colors.mutedForeground;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: fill,
            border: Border.all(
              color: borderColor,
              width: selected ? 2 : theme.style.borderWidth,
            ),
          ),
          child: Padding(
            padding: EdgeInsets.symmetric(
              horizontal: 16,
              vertical: compact ? 11 : 16,
            ),
            child: stacked
                ? Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Row(
                        children: [
                          if (icon case final icon?)
                            Icon(icon, size: 18, color: iconColor),
                          const Spacer(),
                          _CheckSquare(selected: selected),
                        ],
                      ),
                      const SizedBox(height: 18),
                      _cardText(context),
                      if (trailing case final trailingWidget?) ...[
                        const SizedBox(height: 14),
                        trailingWidget,
                      ],
                    ],
                  )
                : Row(
                    spacing: 14,
                    children: [
                      if (icon case final iconData?)
                        Icon(iconData, size: 20, color: iconColor),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          spacing: 3,
                          children: [_cardText(context), ?trailing],
                        ),
                      ),
                      _CheckSquare(selected: selected),
                    ],
                  ),
          ),
        );
      },
    );
  }

  Widget _cardText(BuildContext context) {
    final theme = context.theme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      spacing: 3,
      children: [
        Text(
          title,
          style: theme.typography.body.md.copyWith(
            fontWeight: FontWeight.w600,
            height: 1.25,
          ),
        ),
        if (subtitle case final subtitleText?)
          Text(
            subtitleText,
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.mutedForeground,
              height: 1.4,
            ),
          ),
      ],
    );
  }
}

class _CheckSquare extends StatelessWidget {
  const new({required this.selected});

  final bool selected;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    if (!selected) {
      return SizedBox.square(
        dimension: 18,
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
        ),
      );
    }
    return SizedBox.square(
      dimension: 18,
      child: ColoredBox(
        color: theme.colors.primary,
        child: Icon(
          LucideIcons.check,
          size: 12,
          color: theme.colors.primaryForeground,
        ),
      ),
    );
  }
}
