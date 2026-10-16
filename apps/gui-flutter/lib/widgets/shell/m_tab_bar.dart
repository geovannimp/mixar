import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tab_header.dart';
import 'package:gui_flutter/widgets/shell/m_tabs.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

class MTabBar extends StatelessWidget {
  const new({
    required this.direction,
    required this.expands,
    required this.current,
    required this.children,
    required this.onSelect,
    required this.theme,
    this.barBottomBorder,
  });

  final Axis direction;
  final bool expands;
  final int current;
  final List<MTabEntry> children;
  final ValueChanged<int> onSelect;
  final MixarThemeData theme;
  final BorderSide? barBottomBorder;

  static const _slideDuration = Duration(milliseconds: 300);

  @override
  Widget build(BuildContext context) {
    final horizontal = direction == Axis.horizontal;
    final duration = MediaQuery.disableAnimationsOf(context)
        ? Duration.zero
        : _slideDuration;

    final headers = <Widget>[
      for (var i = 0; i < children.length; i++)
        MTabHeader(
          selected: i == current,
          expands: expands,
          onPress: () => onSelect(i),
          theme: theme,
          // Sliding chip paints selection when expands; otherwise paint on the header.
          paintSelected: !expands,
          child: children[i].label,
        ),
    ];

    final labels = horizontal
        ? Row(children: headers)
        : Column(children: headers);

    final strip = expands
        ? Stack(
            children: [
              Positioned.fill(
                child: AnimatedAlign(
                  duration: duration,
                  curve: Curves.easeInOut,
                  alignment: _slotAlignment(
                    current,
                    children.length,
                    horizontal,
                  ),
                  child: FractionallySizedBox(
                    widthFactor: horizontal ? 1 / children.length : 1,
                    heightFactor: horizontal ? 1 : 1 / children.length,
                    child: DecoratedBox(
                      key: const ValueKey('m-tabs-indicator'),
                      decoration: BoxDecoration(
                        // Soft teal — same [selection] as library rows / menus.
                        color: theme.colors.selection,
                        borderRadius: theme.style.borderRadius.md,
                      ),
                    ),
                  ),
                ),
              ),
              labels,
            ],
          )
        : labels;

    // Only paint sides that should actually show. Fill-matched "flush" borders
    // used to cover the selected chip in the bar's own muted colour and read as
    // a 1px gap between the selection wash and the bar edge.
    //
    // Horizontal bars expose [barBottomBorder]; vertical bars paint a hairline
    // on the trailing edge to separate the rail from content. Blend the
    // hairline onto background (not muted): opaque border-over-muted reads too
    // bright next to content-surface hairlines.
    final hairline = BorderSide(
      color: Color.alphaBlend(theme.colors.border, theme.colors.background),
      width: theme.style.borderWidth,
    );
    // Force the theme hairline on a caller's bottom border: the width is part
    // of the bar's alignment contract, so a wider side would misalign the rows.
    final bottom = barBottomBorder?.copyWith(width: theme.style.borderWidth);
    final border = Border(
      right: horizontal ? BorderSide.none : hairline,
      bottom: horizontal ? (bottom ?? BorderSide.none) : BorderSide.none,
    );
    // Foreground so the hairline stays above the selected chip instead of
    // being covered by it (DecoratedBox under-fill).
    return Container(
      key: kMTabBarKey,
      decoration: BoxDecoration(color: theme.colors.muted),
      foregroundDecoration: BoxDecoration(border: border),
      child: strip,
    );
  }

  /// Slot centers for equal-sized tabs: [Alignment] x/y in \[-1, 1\].
  static Alignment _slotAlignment(int index, int count, bool horizontal) {
    if (count <= 1) {
      return Alignment.center;
    }
    final t = -1.0 + (2.0 * index) / (count - 1);
    return horizontal ? Alignment(t, 0) : Alignment(0, t);
  }
}

/// Identifies the tab bar's painted surface, so callers and tests can find it
/// without matching on its fill colour (which is shared with other tokens).
const kMTabBarKey = ValueKey<String>('mTabsBar');
