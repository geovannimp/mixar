import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// Minimum size of a single tab header (it is a square).
///
/// Keeps the library header / filter row at 36px with the selected chip flush
/// to the bar edge (square chrome — no inset padding).
const kMTabHeaderMinSize = 36.0;

/// Minimum height of a horizontal tab bar (= [kMTabHeaderMinSize]).
///
/// The hairline is painted via [Container.foregroundDecoration] so it does not
/// enlarge the box or get covered by the selected chip. A label taller than
/// [kMTabHeaderMinSize] grows the bar past this value — it is a floor, so align
/// with a `minHeight` unless the consumer deliberately pins a height.
const double kMTabBarMinHeight = kMTabHeaderMinSize;

/// Identifies the tab bar's painted surface, so callers and tests can find it
/// without matching on its fill colour (which is shared with other tokens).
const kMTabBarKey = ValueKey<String>('mTabsBar');

/// One tab label + pane for [MTabs].
@immutable
class MTabEntry {
  const new({required this.label, required this.child});

  final Widget label;
  final Widget child;
}

/// Segmented tabs (Forui-adjacent): muted bar, sliding selected chip, [IndexedStack] panes.
///
/// [direction] lays out the tab bar: [Axis.horizontal] on top, [Axis.vertical] on the start edge.
/// Pass [index] to control selection; omit it for internal state.
///
/// When [expands] is true, the selected chip slides between equal slots (Material [TabBar]
/// parity). When false, the chip snaps onto the selected label without a slide.
class MTabs extends StatefulWidget {
  const new({
    required this.children,
    this.direction = Axis.horizontal,
    this.index,
    this.onChange,
    this.expands = false,
    this.spacing = 0,
    this.barBottomBorder,
    super.key,
  }) : assert(children.length > 0, 'Must provide at least 1 tab.');

  final List<MTabEntry> children;
  final Axis direction;
  final int? index;
  final ValueChanged<int>? onChange;
  final bool expands;

  /// Gap between the tab bar and the content stack.
  final double spacing;

  /// Optional bottom border on the tab bar, so it can line up with a header row
  /// beside it that draws one (the library topbar). The bar's other edges are
  /// painted in its own fill colour, so only this one shows.
  ///
  /// Only the colour (and style) are used: the width is forced to the theme
  /// hairline, since the width is part of [kMTabBarMinHeight]'s alignment
  /// contract and a wider side would overlap the header strip.
  final BorderSide? barBottomBorder;

  @override
  State<MTabs> createState() => _MTabsState();
}

class _MTabsState extends State<MTabs> {
  late int _uncontrolledIndex;

  @override
  void initState() {
    super.initState();
    _uncontrolledIndex = widget.index ?? 0;
  }

  @override
  void didUpdateWidget(MTabs oldWidget) {
    super.didUpdateWidget(oldWidget);
    final index = widget.index;
    if (index != null) {
      _uncontrolledIndex = index;
    }
  }

  int get _current {
    final index = widget.index;
    if (index != null) {
      return index.clamp(0, widget.children.length - 1);
    }
    return _uncontrolledIndex.clamp(0, widget.children.length - 1);
  }

  void _select(int i) {
    if (i == _current) {
      return;
    }
    if (widget.index == null) {
      setState(() => _uncontrolledIndex = i);
    }
    widget.onChange?.call(i);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final current = _current;
    final bar = _TabBar(
      direction: widget.direction,
      expands: widget.expands,
      current: current,
      children: widget.children,
      onSelect: _select,
      theme: theme,
      barBottomBorder: widget.barBottomBorder,
    );
    final stack = IndexedStack(
      index: current,
      sizing: StackFit.expand,
      children: [for (final entry in widget.children) entry.child],
    );

    if (widget.direction == Axis.horizontal) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          bar,
          if (widget.spacing > 0) SizedBox(height: widget.spacing),
          Expanded(child: stack),
        ],
      );
    }

    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        bar,
        if (widget.spacing > 0) SizedBox(width: widget.spacing),
        Expanded(child: stack),
      ],
    );
  }
}

class _TabBar extends StatelessWidget {
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
        _TabHeader(
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
                        // Soft emerald — same [selection] as library rows / menus.
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

    // Outer edges match the muted fill so they don't double the parent card
    // stroke. Horizontal bars expose [barBottomBorder]; vertical bars always
    // paint a hairline on the trailing edge to separate the rail from content.
    final flush = BorderSide(
      color: theme.colors.muted,
      width: theme.style.borderWidth,
    );
    // Blend onto background (not muted): opaque border-over-muted reads too
    // bright next to content-surface hairlines. Background match was chosen
    // after a visual pass — keep it even though the rail fill is muted.
    final hairline = BorderSide(
      color: Color.alphaBlend(theme.colors.border, theme.colors.background),
      width: theme.style.borderWidth,
    );
    // Force the theme hairline on a caller's bottom border: the width is part
    // of the bar's alignment contract, so a wider side would misalign the rows.
    final bottom =
        barBottomBorder?.copyWith(width: theme.style.borderWidth) ?? flush;
    final border = Border(
      top: flush,
      left: flush,
      right: horizontal ? flush : hairline,
      bottom: horizontal ? bottom : flush,
    );
    // Foreground border stays above the selected chip (flush headers would
    // otherwise paint over the hairline — same DecoratedBox under-fill issue).
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

class _TabHeader extends StatelessWidget {
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
    // Selected chrome stays on the emerald brand scale (not teal accent) so
    // tabs, library rows, and + chips read as one palette.
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
