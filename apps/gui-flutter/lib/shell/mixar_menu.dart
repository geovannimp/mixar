import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/m_divider.dart';
import 'package:gui_flutter/shell/m_tappable.dart';

/// Shared panel chrome for Mixar menus and content popovers.
class MixarMenuPanel extends StatelessWidget {
  const MixarMenuPanel({required this.child, this.minWidth = 180, super.key});

  final Widget child;
  final double minWidth;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return ConstrainedBox(
      constraints: BoxConstraints(minWidth: minWidth),
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: theme.colors.card,
          borderRadius: theme.style.borderRadius.md,
          border: Border.all(
            color: theme.colors.border,
            width: theme.style.borderWidth,
          ),
        ),
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
    );
  }
}

/// Column of [MixarMenuGroup]s with hairline separators between groups.
class MixarMenuBody extends StatelessWidget {
  const MixarMenuBody({required this.groups, super.key});

  final List<Widget> groups;

  @override
  Widget build(BuildContext context) {
    if (groups.isEmpty) {
      return const SizedBox.shrink();
    }
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var i = 0; i < groups.length; i++) ...[
          if (i > 0) const MDivider(padding: EdgeInsets.symmetric(vertical: 4)),
          groups[i],
        ],
      ],
    );
  }
}

class MixarMenuGroup extends StatelessWidget {
  const MixarMenuGroup({required this.children, super.key});

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

/// Tappable menu row. Prefer [title] for text actions; [child] for custom
/// blocks (e.g. load-to-deck chips).
class MixarMenuItem extends StatelessWidget {
  const MixarMenuItem({
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
    final content =
        child ??
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
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
      return content;
    }

    return MTappable(
      onPress: enabled ? onPress : null,
      builder: (context, state) {
        final paint = state.active
            ? theme.colors.secondary
            : const Color(0x00000000);
        return ColoredBox(color: paint, child: content);
      },
    );
  }
}
