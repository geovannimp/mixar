import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// One cell of a [SettingsSegmentedChoice]: a value, its label, and an
/// optional leading mark (a palette dot, a theme glyph).
@immutable
class SettingsSegmentedOption<T> {
  const new({
    required this.value,
    required this.label,
    this.leading,
    this.labelColor,
    this.selectedBorder,
  });

  final T value;

  /// Visible cell text, also the cell's accessible name.
  final String label;

  /// Small mark before the label. Drops out when segments get too narrow.
  final Widget? leading;

  /// Label ink override, e.g. the accent palettes' own tone. Falls back to
  /// the theme foreground.
  final Color? labelColor;

  /// Border colour for the selected cell, e.g. the accent palette's own tone.
  /// `null` uses the theme primary. Selection never paints a fill.
  final Color? selectedBorder;
}

/// A rounded segmented track: one bordered pill split by hairlines, matching
/// the deck selector's grouped options.
///
/// Selection is carried by a coloured border around the cell, the heavier
/// label, and the leading mark; a keyboard focus ring paints the same border
/// in the foreground colour.
class SettingsSegmentedChoice<T> extends StatelessWidget {
  const new({
    required this.value,
    required this.options,
    required this.onChanged,
    super.key,
  });

  final T value;
  final List<SettingsSegmentedOption<T>> options;
  final ValueChanged<T> onChanged;

  /// Below this width a segment cannot hold its leading mark *and* a readable
  /// label, so the mark drops out and the label keeps the space.
  static const _leadingBreakpoint = 96.0;

  /// Up to one row of cells reads as a pill. Past that, the same cells wrap
  /// into a three-column grid — a single row of six or more labels would
  /// squeeze every segment below readability.
  static const _rowLimit = 5;
  static const _gridColumns = 3;

  @override
  Widget build(BuildContext context) {
    if (options.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final singleRow = options.length <= _rowLimit;
        final columns = singleRow ? options.length : _gridColumns;
        final showLeading =
            !constraints.hasBoundedWidth ||
            constraints.maxWidth / columns >= _leadingBreakpoint;
        // A grid keeps the softer card corner: the pill's stadium radius
        // would bite into the corner cells of a multi-row track.
        final radius = singleRow
            ? theme.style.borderRadius.pill
            : theme.style.borderRadius.dialog;
        return DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(
              color: theme.colors.border,
              width: theme.style.borderWidth,
            ),
            borderRadius: radius,
          ),
          child: ClipRRect(
            borderRadius: radius,
            child: singleRow
                ? _row(theme, showLeading)
                : _grid(theme, showLeading),
          ),
        );
      },
    );
  }

  Widget _row(MixarThemeData theme, bool showLeading) {
    final pill = theme.style.borderRadius.pill;
    BorderRadius? shapeFor(int index) {
      if (options.length == 1) {
        return pill;
      }
      if (index == 0) {
        return BorderRadius.horizontal(left: pill.topLeft);
      }
      if (index == options.length - 1) {
        return BorderRadius.horizontal(right: pill.topRight);
      }
      return null;
    }

    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          for (var i = 0; i < options.length; i++) ...[
            if (i > 0) _divider(theme, Axis.vertical),
            Expanded(child: _cell(options[i], showLeading, shapeFor(i))),
          ],
        ],
      ),
    );
  }

  Widget _grid(MixarThemeData theme, bool showLeading) {
    final rows = (options.length + _gridColumns - 1) ~/ _gridColumns;
    final lastRow = rows - 1;
    final corner = theme.style.borderRadius.dialog.topLeft;
    BorderRadius? shapeFor(int row, int column) {
      final top = row == 0;
      final bottom = row == lastRow;
      final left = column == 0;
      final right = column == _gridColumns - 1;
      if (top && left) return BorderRadius.only(topLeft: corner);
      if (top && right) return BorderRadius.only(topRight: corner);
      if (bottom && left) return BorderRadius.only(bottomLeft: corner);
      if (bottom && right) return BorderRadius.only(bottomRight: corner);
      return null;
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var row = 0; row < rows; row++) ...[
          if (row > 0) _divider(theme, Axis.horizontal),
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var column = 0; column < _gridColumns; column++) ...[
                  if (column > 0) _divider(theme, Axis.vertical),
                  Expanded(
                    child: switch (row * _gridColumns + column) {
                      final index when index < options.length => _cell(
                        options[index],
                        showLeading,
                        shapeFor(row, column),
                      ),
                      // Short last row: the empty slot keeps the column
                      // hairlines running to the bottom edge.
                      _ => const SizedBox.shrink(),
                    },
                  ),
                ],
              ],
            ),
          ),
        ],
      ],
    );
  }

  Widget _cell(
    SettingsSegmentedOption<T> option,
    bool showLeading,
    BorderRadius? shape,
  ) {
    return _Segment<T>(
      option: option,
      selected: option.value == value,
      showLeading: showLeading,
      borderShape: shape,
      onPress: () => onChanged(option.value),
    );
  }

  Widget _divider(MixarThemeData theme, Axis axis) => MDivider(
    axis: axis,
    padding: EdgeInsets.zero,
    color: theme.colors.border,
  );
}

class _Segment<T> extends StatelessWidget {
  const new({
    required this.option,
    required this.selected,
    required this.showLeading,
    required this.borderShape,
    required this.onPress,
  });

  final SettingsSegmentedOption<T> option;
  final bool selected;
  final bool showLeading;

  /// Outer corners rounded when the cell sits at the track's edge, so the
  /// selection border follows the pill instead of ending in a square.
  final BorderRadius? borderShape;

  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final leading = option.leading;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: option.label,
      builder: (context, state) {
        final border = state.focused
            ? Border.all(color: theme.colors.foreground, width: 1.5)
            : selected
            ? Border.all(
                color: option.selectedBorder ?? theme.colors.primary,
                width: 1.5,
              )
            : null;
        return DecoratedBox(
          // The cell border paints over the track: the selection and the
          // focus ring stay readable above the hairlines.
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(border: border, borderRadius: borderShape),
          child: DecoratedBox(
            decoration: BoxDecoration(
              // Selection is a border only; the fill stays for hover/press.
              color: state.hovered || state.pressed
                  ? theme.hoverWash
                  : const Color(0x00000000),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 10),
              child: ExcludeSemantics(
                // The cell's Semantics already carries the name; the visible
                // label and mark stay out of the tree so they are not read
                // twice.
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    if (showLeading && leading != null) ...[
                      leading,
                      const SizedBox(width: 8),
                    ],
                    Flexible(
                      child: Text(
                        option.label,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        textAlign: TextAlign.center,
                        style: theme.typography.body.sm.copyWith(
                          color: option.labelColor ?? theme.colors.foreground,
                          fontWeight: selected
                              ? FontWeight.w600
                              : FontWeight.w500,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
