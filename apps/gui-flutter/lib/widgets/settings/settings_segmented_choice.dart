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
    this.selectedFill,
  });

  final T value;

  /// Visible cell text, also the cell's accessible name.
  final String label;

  /// Small mark before the label. Drops out when segments get too narrow.
  final Widget? leading;

  /// Label ink override, e.g. the accent palettes' own tone. Falls back to
  /// the theme foreground.
  final Color? labelColor;

  /// Fill behind the selected cell. `null` uses the theme's selection tint.
  final Color? selectedFill;
}

/// A rounded segmented track: one bordered pill split by hairlines, matching
/// the deck selector's grouped options.
///
/// Selection is carried by the palette tint behind the cell (or the theme
/// selection), the heavier label, and the filled leading mark; a keyboard
/// focus ring paints over the fill.
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

  /// Below this width a segment cannot hold its leading mark without
  /// crushing the label, so the mark drops out and the label keeps the space.
  static const _leadingBreakpoint = 56.0;

  @override
  Widget build(BuildContext context) {
    if (options.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = context.theme;
    final radius = theme.style.borderRadius.pill;
    return LayoutBuilder(
      builder: (context, constraints) {
        final showLeading =
            !constraints.hasBoundedWidth ||
            constraints.maxWidth / options.length >= _leadingBreakpoint;
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
            child: IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (var i = 0; i < options.length; i++) ...[
                    if (i > 0)
                      MDivider(
                        axis: Axis.vertical,
                        padding: EdgeInsets.zero,
                        color: theme.colors.border,
                      ),
                    Expanded(
                      child: _Segment<T>(
                        option: options[i],
                        selected: options[i].value == value,
                        showLeading: showLeading,
                        onPress: () => onChanged(options[i].value),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class _Segment<T> extends StatelessWidget {
  const new({
    required this.option,
    required this.selected,
    required this.showLeading,
    required this.onPress,
  });

  final SettingsSegmentedOption<T> option;
  final bool selected;
  final bool showLeading;
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
        return DecoratedBox(
          // Focus paints over the fill: the ring is always readable, and the
          // track's hairlines stay underneath it.
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(
            border: state.focused
                ? Border.all(color: theme.colors.foreground, width: 1.5)
                : null,
          ),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: selected
                  ? option.selectedFill ?? theme.colors.selection
                  : state.hovered || state.pressed
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
