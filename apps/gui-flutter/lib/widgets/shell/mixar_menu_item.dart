import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_dialog_chrome.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

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
