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
    this.selected = false,
    super.key,
  }) : assert(title != null || child != null, 'title or child required');

  final Widget? title;
  final Widget? child;
  final bool enabled;
  final VoidCallback? onPress;
  final bool destructive;

  /// Latched state for choice rows (e.g. the active FX kind).
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final dialogChrome = MixarMenuDialogChrome.isActive(context);
    // Custom blocks own their look; only the text rows add row padding.
    final content =
        child ??
        Padding(
          padding: EdgeInsets.symmetric(
            horizontal: dialogChrome ? 24 : 12,
            vertical: dialogChrome ? 12 : 8,
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
      return content;
    }

    return MTappable(
      onPress: enabled ? onPress : null,
      selected: selected,
      builder: (context, state) {
        // Latch state is semantics-only (choice rows announce it); the wash is
        // the momentary hover/press feedback shared by every presentation.
        final paint = state.hovered || state.pressed
            ? theme.colors.secondary
            : const Color(0x00000000);
        return ColoredBox(color: paint, child: content);
      },
    );
  }
}
