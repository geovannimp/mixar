import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/m_divider.dart';
import 'package:gui_flutter/shell/mixar_menu_dialog_chrome.dart';
import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

export 'package:gui_flutter/shell/mixar_menu_dialog_chrome.dart';
export 'package:gui_flutter/shell/mixar_menu_segments.dart'
    show MixarMenuSegment, MixarMenuSegments;

/// Shared panel chrome for Mixar menus and content popovers.
///
/// Fixed [minWidth] — Anchor overlays get viewport-max constraints, so a
/// min-only [ConstrainedBox] + [Expanded] chips expands to full screen.
class MixarMenuPanel extends StatelessWidget {
  const new({required this.child, this.minWidth = 180, super.key});

  final Widget child;
  final double minWidth;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox(
      width: minWidth,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: theme.colors.card,
          borderRadius: theme.style.borderRadius.md,
          border: Border.all(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
        ),
        // Clip rows to the rounded border, or an edge row's hover fill squares
        // off the corner and bleeds past the panel outline.
        child: ClipRRect(
          borderRadius: theme.style.borderRadius.md,
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 4),
            child: DefaultTextStyle(
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.foreground,
              ),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}

/// Column of [MixarMenuGroup]s with hairline separators between groups.
class MixarMenuBody extends StatelessWidget {
  const new({required this.groups, super.key});

  final List<Widget> groups;

  @override
  Widget build(BuildContext context) {
    if (groups.isEmpty) {
      return const SizedBox.shrink();
    }
    final dialogChrome = MixarMenuDialogChrome.isActive(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var i = 0; i < groups.length; i++) ...[
          if (i > 0)
            dialogChrome
                // Card rows already own a 6px bottom gap; keep the same rhythm
                // between groups instead of a hairline + extra padding.
                ? const SizedBox.shrink()
                : const MDivider(padding: EdgeInsets.symmetric(vertical: 4)),
          groups[i],
        ],
      ],
    );
  }
}

class MixarMenuGroup extends StatelessWidget {
  const new({required this.children, super.key});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: children,
    );
  }
}

/// Static, non-interactive menu row (section header / disabled label).
///
/// Unlike [MixarMenuItem] this never wraps in an interactive widget, so it is
/// not announced as a button and stays in normal reading order. Use it for
/// labels like "Load to deck" that head a group of actions.
class MixarMenuLabel extends StatelessWidget {
  const new({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final dialogChrome = MixarMenuDialogChrome.isActive(context);
    return Padding(
      // Dialog title has no horizontal inset; section labels share that edge.
      // Action card labels keep their own 12px content padding inside the card.
      padding: dialogChrome
          ? const EdgeInsets.fromLTRB(0, 0, 0, 10)
          : const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: DefaultTextStyle(
        style: theme.typography.body.sm.copyWith(
          color: theme.colors.mutedForeground,
        ),
        child: child,
      ),
    );
  }
}

/// Tappable menu row. Prefer [title] for text actions; [child] for custom
/// blocks (e.g. load-to-deck chips).
class MixarMenuItem extends StatelessWidget {
  const new({
    this.title,
    this.child,
    this.enabled = true,
    this.onPress,
    this.destructive = false,
    super.key,
  }) : assert(title != null || child != null, 'title or child required');

  final Widget? title;
  final Widget? child;
  final bool enabled;
  final VoidCallback? onPress;
  final bool destructive;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final dialogChrome = MixarMenuDialogChrome.isActive(context);
    final content =
        child ??
        Padding(
          padding: EdgeInsets.symmetric(
            horizontal: 12,
            vertical: dialogChrome ? 10 : 8,
          ),
          child: DefaultTextStyle(
            style: theme.typography.body.sm.copyWith(
              color: !enabled
                  ? theme.colors.mutedForeground
                  : destructive
                  ? theme.colors.destructive
                  : theme.colors.foreground,
            ),
            child: title!,
          ),
        );

    if (child != null && onPress == null) {
      if (!dialogChrome) {
        return content;
      }
      return Padding(
        padding: const EdgeInsets.only(bottom: 6),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: theme.colors.card,
            borderRadius: theme.style.borderRadius.md,
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
          ),
          child: content,
        ),
      );
    }

    return Padding(
      padding: dialogChrome
          ? const EdgeInsets.only(bottom: 6)
          : EdgeInsets.zero,
      child: MTappable(
        onPress: enabled ? onPress : null,
        builder: (context, state) {
          if (!dialogChrome) {
            final paint = state.active
                ? theme.colors.selection
                : const Color(0x00000000);
            return ColoredBox(color: paint, child: content);
          }
          // Match mobile MixarSelect option cards.
          final fill = state.active
              ? theme.colors.secondary
              : theme.colors.card;
          return DecoratedBox(
            decoration: BoxDecoration(
              color: fill,
              borderRadius: theme.style.borderRadius.md,
              border: Border.all(
                color: theme.colors.border,
                width: theme.style.borderWidth,
              ),
            ),
            child: content,
          );
        },
      ),
    );
  }
}
