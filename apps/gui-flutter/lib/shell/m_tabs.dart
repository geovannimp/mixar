import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/shell/m_tappable.dart';

/// One tab label + pane for [MTabs].
@immutable
class MTabEntry {
  const MTabEntry({required this.label, required this.child});

  final Widget label;
  final Widget child;
}

/// Segmented tabs (Forui-adjacent): muted bar, selected background chip, [IndexedStack] panes.
///
/// [direction] lays out the tab bar: [Axis.horizontal] on top, [Axis.vertical] on the start edge.
/// Pass [index] to control selection; omit it for internal state.
class MTabs extends StatefulWidget {
  const MTabs({
    required this.children,
    this.direction = Axis.horizontal,
    this.index,
    this.onChange,
    this.expands = false,
    this.spacing = 0,
    super.key,
  }) : assert(children.length > 0, 'Must provide at least 1 tab.');

  final List<MTabEntry> children;
  final Axis direction;
  final int? index;
  final ValueChanged<int>? onChange;
  final bool expands;

  /// Gap between the tab bar and the content stack.
  final double spacing;

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
  const _TabBar({
    required this.direction,
    required this.expands,
    required this.current,
    required this.children,
    required this.onSelect,
    required this.theme,
  });

  final Axis direction;
  final bool expands;
  final int current;
  final List<MTabEntry> children;
  final ValueChanged<int> onSelect;
  final FThemeData theme;

  @override
  Widget build(BuildContext context) {
    final tabs = <Widget>[
      for (var i = 0; i < children.length; i++)
        _TabHeader(
          selected: i == current,
          expands: expands,
          direction: direction,
          onPress: () => onSelect(i),
          theme: theme,
          child: children[i].label,
        ),
    ];

    final strip = direction == Axis.horizontal
        ? Row(children: tabs)
        : Column(children: tabs);

    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.muted,
        border: Border.all(
          color: theme.colors.muted,
          width: theme.style.borderWidth,
        ),
      ),
      child: Padding(padding: const EdgeInsets.all(4), child: strip),
    );
  }
}

class _TabHeader extends StatelessWidget {
  const _TabHeader({
    required this.selected,
    required this.expands,
    required this.direction,
    required this.onPress,
    required this.child,
    required this.theme,
  });

  final bool selected;
  final bool expands;
  final Axis direction;
  final VoidCallback onPress;
  final Widget child;
  final FThemeData theme;

  @override
  Widget build(BuildContext context) {
    final foreground = selected
        ? theme.colors.foreground
        : theme.colors.mutedForeground;

    Widget header = MTappable(
      selected: selected,
      onPress: onPress,
      builder: (context, state) {
        final active = state.active;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: selected || active ? theme.colors.background : null,
            borderRadius: theme.style.borderRadius.md,
          ),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 28, minWidth: 28),
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
