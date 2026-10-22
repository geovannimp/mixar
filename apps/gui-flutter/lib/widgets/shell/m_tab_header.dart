import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Minimum height of a horizontal tab bar (= [kMTabHeaderMinSize]).
const double kMTabBarMinHeight = kMTabHeaderMinSize;

/// Minimum size of a single tab header (it is a square).
const kMTabHeaderMinSize = 36.0;

class MTabHeader extends StatelessWidget {
  const new({
    required this.selected,
    required this.expands,
    required this.onPress,
    required this.child,
    required this.theme,
    required this.paintSelected,
  });

  final bool selected;
  final bool expands;
  final VoidCallback onPress;
  final Widget child;
  final MixarThemeData theme;
  final bool paintSelected;

  @override
  Widget build(BuildContext context) {
    // Selected chrome stays on the teal brand scale so tabs, library rows,
    // and + chips read as one palette.
    final foreground = selected
        ? theme.colors.primary
        : theme.colors.mutedForeground;

    Widget header = MTappable(
      selected: selected,
      onPress: onPress,
      builder: (context, state) {
        return DecoratedBox(
          decoration: BoxDecoration(
            color: paintSelected && selected ? theme.colors.selection : null,
            borderRadius: theme.style.borderRadius.md,
          ),
          child: ConstrainedBox(
            constraints: const BoxConstraints(
              minHeight: kMTabHeaderMinSize,
              minWidth: kMTabHeaderMinSize,
            ),
            child: Center(
              child: IconTheme.merge(
                data: IconThemeData(color: foreground, size: 16),
                child: DefaultTextStyle.merge(
                  style: theme.typography.body.sm.copyWith(
                    fontWeight: FontWeight.w500,
                    color: foreground,
                  ),
                  child: child,
                ),
              ),
            ),
          ),
        );
      },
    );

    if (expands) {
      header = Expanded(child: header);
    }
    return header;
  }
}
