import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tab_bar.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

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
  /// beside it that draws one (the library topbar). Other edges stay bare so
  /// the selected chip can sit flush to the bar.
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
    final bar = MTabBar(
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
