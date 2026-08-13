import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// Uppercase sidebar section label (Tauri `LibraryPaneHeader`).
class LibraryPaneLabel extends StatelessWidget {
  const LibraryPaneLabel(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Text(
      text.toUpperCase(),
      style: theme.typography.body.xs.copyWith(
        color: theme.colors.mutedForeground,
        fontWeight: FontWeight.w600,
        letterSpacing: 1.4,
      ),
    );
  }
}

/// Full-width sidebar row: left accent, hover/selected fill from the Forui palette.
class LibraryNavRow extends StatelessWidget {
  const LibraryNavRow({
    super.key,
    required this.title,
    this.subtitle,
    this.icon,
    this.trailing,
    this.selected = false,
    this.indented = false,
    this.onPress,
  });

  final String title;
  final String? subtitle;
  final IconData? icon;
  final Widget? trailing;
  final bool selected;
  final bool indented;
  final VoidCallback? onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;

    Widget row(Set<WidgetState> states) {
      final hovered = states.contains(WidgetState.hovered);
      final fill = selected
          ? colors.primary.withValues(alpha: 0.10)
          : hovered
          ? colors.foreground.withValues(alpha: 0.05)
          : null;
      return DecoratedBox(
        decoration: BoxDecoration(
          color: fill,
          border: Border(
            left: BorderSide(
              width: 2,
              color: selected ? colors.primary : const Color(0x00000000),
            ),
          ),
        ),
        child: Padding(
          padding: EdgeInsets.fromLTRB(indented ? 20 : 10, 8, 4, 8),
          child: Row(
            children: [
              if (icon != null) ...[
                Icon(icon, size: 16, color: colors.mutedForeground),
                const SizedBox(width: 8),
              ],
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.typography.body.sm.copyWith(
                        color: colors.foreground,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    if (subtitle != null)
                      Text(
                        subtitle!,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.typography.body.xs.copyWith(
                          color: colors.mutedForeground,
                        ),
                      ),
                  ],
                ),
              ),
              ?trailing,
            ],
          ),
        ),
      );
    }

    if (onPress == null) {
      return row({if (selected) WidgetState.selected});
    }

    return FTappable(
      selected: selected,
      onPress: onPress,
      builder: (context, variants, _) {
        final states = <WidgetState>{
          if (selected) WidgetState.selected,
          if (variants.contains(FTappableVariant.hovered)) WidgetState.hovered,
        };
        return row(states);
      },
    );
  }
}
