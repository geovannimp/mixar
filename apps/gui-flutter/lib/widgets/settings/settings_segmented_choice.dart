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
    this.description,
    this.leading,
    this.labelColor,
    this.selectedBorder,
  });

  final T value;

  /// Visible cell text, also the cell's accessible name.
  final String label;

  /// Longer explanation read after [label] by assistive tech; the cell itself
  /// shows only the label.
  final String? description;

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
/// Cells stay in one row while each one keeps [_minCellWidth]; otherwise the
/// track wraps into as many columns as the pane can hold (three, two, or one),
/// and the last row's final cell spans the leftover columns. Selection is
/// carried by a coloured border around the cell, the heavier label, and the
/// leading mark; keyboard focus thickens that border.
///
/// Needs a bounded width: every cell is an [Expanded] inside a row.
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

  /// The narrowest cell that still reads: a 36px mark, an 8px gap, a short
  /// label, and the cell padding.
  static const _minCellWidth = 120.0;

  /// Up to one row of cells reads as a pill. Past that, the same cells wrap
  /// into a grid — a single row of six or more labels would squeeze every
  /// segment below readability.
  static const _rowLimit = 5;
  static const _maxGridColumns = 3;

  @override
  Widget build(BuildContext context) {
    if (options.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) {
        assert(
          constraints.hasBoundedWidth,
          'SettingsSegmentedChoice needs a bounded width for its Expanded '
          'cells.',
        );
        final width = constraints.maxWidth;
        final singleRow =
            options.length <= _rowLimit &&
            width / options.length >= _minCellWidth;
        final columns = singleRow
            ? options.length
            : width >= _maxGridColumns * _minCellWidth
            ? _maxGridColumns
            : width >= 2 * _minCellWidth
            ? 2
            : 1;
        final dividers = columns - 1;
        final cellWidth =
            (width - dividers * theme.style.borderWidth) / columns;
        // At pathological widths the padding would eat the whole cell; shrink
        // it so a truncated label still shows.
        final inset = cellWidth >= 44 ? 6.0 : 2.0;
        // The mark only fits when the cell keeps a readable label beside it.
        final contentWidth = cellWidth - 2 * inset;
        final showLeading = contentWidth >= _leadingBreakpoint;
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
                ? _row(theme, showLeading, inset)
                : _grid(theme, showLeading, columns, inset),
          ),
        );
      },
    );
  }

  Widget _row(MixarThemeData theme, bool showLeading, double inset) {
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
            Expanded(child: _cell(options[i], showLeading, inset, shapeFor(i))),
          ],
        ],
      ),
    );
  }

  Widget _grid(
    MixarThemeData theme,
    bool showLeading,
    int columns,
    double inset,
  ) {
    final rows = (options.length + columns - 1) ~/ columns;
    final lastRow = rows - 1;
    final corner = theme.style.borderRadius.dialog.topLeft;
    BorderRadius? shapeFor(int row, int column, int count) {
      final top = row == 0;
      final bottom = row == lastRow;
      final left = column == 0;
      final right = column == count - 1;
      final shape = BorderRadius.only(
        topLeft: top && left ? corner : Radius.zero,
        topRight: top && right ? corner : Radius.zero,
        bottomLeft: bottom && left ? corner : Radius.zero,
        bottomRight: bottom && right ? corner : Radius.zero,
      );
      return shape == BorderRadius.zero ? null : shape;
    }

    return Column(
      // A grid sits in loose, bounded contexts too; without this it would
      // stretch to the parent's height and detach the last row from the
      // track's bottom edge.
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var row = 0; row < rows; row++) ...[
          if (row > 0) _divider(theme, Axis.horizontal),
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (
                  var column = 0;
                  column < _rowCount(row, columns);
                  column++
                ) ...[
                  if (column > 0) _divider(theme, Axis.vertical),
                  Expanded(
                    // The last row's final cell fills the leftover columns,
                    // so no inert placeholder owns a track corner.
                    flex: _flex(row, column, columns, rows),
                    child: _cell(
                      options[row * columns + column],
                      showLeading,
                      inset,
                      shapeFor(row, column, _rowCount(row, columns)),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ],
      ],
    );
  }

  int _rowCount(int row, int columns) =>
      (options.length - row * columns).clamp(1, columns);

  int _flex(int row, int column, int columns, int rows) {
    if (row != rows - 1) {
      return 1;
    }
    final count = _rowCount(row, columns);
    return column == count - 1 ? columns - (count - 1) : 1;
  }

  Widget _cell(
    SettingsSegmentedOption<T> option,
    bool showLeading,
    double inset,
    BorderRadius? shape,
  ) {
    return _Segment<T>(
      option: option,
      selected: option.value == value,
      showLeading: showLeading,
      borderShape: shape,
      horizontalInset: inset,
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
    required this.horizontalInset,
    required this.onPress,
  });

  final SettingsSegmentedOption<T> option;
  final bool selected;
  final bool showLeading;

  /// Outer corners rounded when the cell sits at the track's edge, so the
  /// selection border follows the pill instead of ending in a square.
  final BorderRadius? borderShape;

  final double horizontalInset;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final leading = option.leading;
    final selectionColor = option.selectedBorder ?? theme.colors.primary;
    return MTappable(
      onPress: onPress,
      selected: selected,
      semanticsLabel: option.description == null
          ? option.label
          : '${option.label}. ${option.description}',
      builder: (context, state) {
        // Focus never erases the selection's colour: a selected cell keeps
        // its border colour at a heavier weight. An unselected cell gains
        // its palette tone when it has one, otherwise the foreground.
        final focusColor = selected
            ? selectionColor
            : option.selectedBorder ?? theme.colors.foreground;
        final border = state.focused
            ? Border.all(color: focusColor, width: 2.5)
            : selected
            ? Border.all(color: selectionColor, width: 1.5)
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
              padding: EdgeInsets.symmetric(
                horizontal: horizontalInset,
                vertical: 10,
              ),
              child: ExcludeSemantics(
                // The cell's Semantics already carries the name; the visible
                // label and mark stay out of the tree so they are not read
                // twice.
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    if (showLeading && leading != null) ...[
                      // Icons follow the selected/unselected ink without
                      // every picker passing a colour.
                      IconTheme.merge(
                        data: IconThemeData(
                          size: 14,
                          color: selected
                              ? theme.colors.foreground
                              : theme.colors.mutedForeground,
                        ),
                        child: leading,
                      ),
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
