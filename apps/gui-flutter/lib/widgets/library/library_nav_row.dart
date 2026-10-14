import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Selected fill shared by library sidebar rows and track/history list rows.
///
/// Solid [MixarColors.selection] (teal-100 / teal-950) — same on every
/// surface, no alphaBlend that drifts with the base.
Color librarySelectedRowColor(MixarThemeData theme) => theme.colors.selection;

/// Full-width sidebar row: left accent, hover/selected fill from MixarTheme colors.
class LibraryNavRow extends StatelessWidget {
  const new({
    required this.title,
    super.key,
    this.subtitle,
    this.icon,
    this.trailing,
    this.selected = false,
    this.focused = false,
    this.indented = false,
    this.onPress,
  });

  final String title;
  final String? subtitle;
  final IconData? icon;
  final Widget? trailing;
  final bool selected;

  /// Whether the sidebar owns browser focus, which marks this row as the cursor
  /// the browse rotary moves. Without it a selected row and a focused row read
  /// the same while the track list owns focus.
  final bool focused;
  final bool indented;
  final VoidCallback? onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final colors = theme.colors;

    Widget row({required bool hovered}) {
      // The cursor wears the same fill as a selection: a focused Drive volume
      // is not the active volume, and an outline with no fill reads as chrome
      // rather than as the row the rotary is on.
      final marked = selected || focused;
      final fill = marked
          ? librarySelectedRowColor(theme)
          : hovered
          ? colors.card
          : null;
      return DecoratedBox(
        decoration: BoxDecoration(
          color: fill,
          // Focused cursor: the left accent grows into a full outline, so a
          // selected row and a focused row read differently at a glance.
          border: focused
              ? Border.all(color: colors.primary, width: 2)
              : Border(
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
                Icon(
                  icon,
                  size: 16,
                  color: marked
                      ? colors.selectionForeground
                      : colors.mutedForeground,
                ),
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
                        color: marked
                            ? colors.selectionForeground
                            : colors.foreground,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    if (subtitle != null)
                      Text(
                        subtitle!,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.typography.body.xs.copyWith(
                          color: marked
                              ? colors.selectionForeground.withValues(
                                  alpha: 0.75,
                                )
                              : colors.mutedForeground,
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

    // The sidebar's cursor can move onto rows past the viewport, so a row that
    // takes the cursor brings itself into view.
    return _ScrollFocusedIntoView(
      focused: focused,
      child: onPress == null
          ? row(hovered: false)
          : MTappable(
              selected: selected,
              onPress: onPress,
              builder: (context, state) => row(hovered: state.hovered),
            ),
    );
  }
}

/// Scrolls its child into view when [focused] turns on, so a cursor that moves
/// off the visible rows (a rotary step in a long sidebar list) is not lost.
///
/// One step at a time the newly focused row is adjacent to the one that had the
/// cursor, so it is already laid out and a single [Scrollable.ensureVisible]
/// reaches it — no fixed row extent to keep in step with the two row layouts.
class _ScrollFocusedIntoView extends StatefulWidget {
  const _ScrollFocusedIntoView({required this.focused, required this.child});

  final bool focused;
  final Widget child;

  @override
  State<_ScrollFocusedIntoView> createState() => _ScrollFocusedIntoViewState();
}

class _ScrollFocusedIntoViewState extends State<_ScrollFocusedIntoView> {
  @override
  void didUpdateWidget(_ScrollFocusedIntoView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.focused && !oldWidget.focused) {
      // After this frame's layout: the row is in the tree by then.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        // The cursor may already have stepped off this row in the meantime —
        // scrolling it in would be churn for a row nobody is looking at.
        if (!mounted || !widget.focused) {
          return;
        }
        Scrollable.ensureVisible(
          context,
          duration: const Duration(milliseconds: 120),
          alignment: 0.5,
          curve: Curves.easeOutCubic,
        );
      });
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
