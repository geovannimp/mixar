import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/m_tappable.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// One row of a [SettingsVisualChoice]: a value, its label, and the picture
/// that says what the value does.
@immutable
class SettingsVisualOption<T> {
  const new({
    required this.value,
    required this.label,
    required this.preview,
    this.subtitle,
  });

  final T value;
  final String label;

  /// Muted line under [label]. Also joins the semantics label, so a screen
  /// reader hears the same explanation the row shows.
  final String? subtitle;

  /// Fills the 64×28 preview box at the end of a list row, or the tile above
  /// the label when the choice is horizontal.
  final Widget preview;
}

/// Radio list whose options carry a preview glyph, like the crossfader
/// curves.
///
/// Vertical (default) rows put the preview at the end of the row. Horizontal
/// tiles put it on top with the radio and label underneath, for choices whose
/// pictures read better side by side. Selection is carried by the border, the
/// fill, the radio mark and the label weight together; the preview never
/// changes with selection.
class SettingsVisualChoice<T> extends StatelessWidget {
  const new({
    required this.value,
    required this.options,
    required this.onChanged,
    this.axis = Axis.vertical,
    this.previewAspectRatio,
    super.key,
  });

  final T value;
  final List<SettingsVisualOption<T>> options;
  final ValueChanged<T> onChanged;

  /// `Axis.vertical` = list rows; `Axis.horizontal` = side-by-side tiles.
  final Axis axis;

  /// When set, previews take this aspect ratio (16/9 for window layouts);
  /// otherwise they keep the compact glyph box.
  final double? previewAspectRatio;

  @override
  Widget build(BuildContext context) {
    if (axis == Axis.horizontal) {
      return Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        spacing: 8,
        children: [
          for (final option in options)
            Expanded(
              child: _ChoiceTile<T>(
                option: option,
                selected: option.value == value,
                onSelected: onChanged,
                previewAspectRatio: previewAspectRatio,
              ),
            ),
        ],
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 8,
      children: [
        for (final option in options)
          _ChoiceRow<T>(
            option: option,
            selected: option.value == value,
            onSelected: onChanged,
            previewAspectRatio: previewAspectRatio,
          ),
      ],
    );
  }
}

/// Horizontal tile: preview on top, radio and label under it.
class _ChoiceTile<T> extends StatelessWidget {
  const new({
    required this.option,
    required this.selected,
    required this.onSelected,
    this.previewAspectRatio,
  });

  final SettingsVisualOption<T> option;
  final bool selected;
  final ValueChanged<T> onSelected;
  final double? previewAspectRatio;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final subtitle = option.subtitle;
    final ratio = previewAspectRatio;
    return MTappable(
      onPress: () => onSelected(option.value),
      selected: selected,
      semanticsLabel: subtitle == null
          ? option.label
          : '${option.label}. $subtitle',
      builder: (context, _) {
        final border = selected ? theme.colors.primary : theme.colors.border;
        return DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(color: border, width: selected ? 1.5 : 1),
            borderRadius: theme.style.borderRadius.md,
          ),
          child: Padding(
            padding: const EdgeInsets.all(8),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (ratio == null)
                  SizedBox(
                    width: double.infinity,
                    height: 34,
                    child: option.preview,
                  )
                else
                  ConstrainedBox(
                    constraints: const BoxConstraints(maxWidth: 88),
                    child: AspectRatio(
                      aspectRatio: ratio,
                      child: option.preview,
                    ),
                  ),
                const SizedBox(height: 10),
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    _RadioMark(selected: selected),
                    const SizedBox(width: 6),
                    Flexible(
                      child: ExcludeSemantics(
                        child: Text(
                          option.label,
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                          textAlign: TextAlign.center,
                          style: theme.typography.body.sm.copyWith(
                            color: theme.colors.foreground,
                            fontWeight: selected
                                ? FontWeight.w600
                                : FontWeight.w500,
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
                if (subtitle != null) ...[
                  const SizedBox(height: 2),
                  ExcludeSemantics(
                    child: Text(
                      subtitle,
                      textAlign: TextAlign.center,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.typography.body.xs.copyWith(
                        color: theme.colors.mutedForeground,
                        height: 1.25,
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ),
        );
      },
    );
  }
}

class _ChoiceRow<T> extends StatelessWidget {
  const new({
    required this.option,
    required this.selected,
    required this.onSelected,
    this.previewAspectRatio,
  });

  final SettingsVisualOption<T> option;
  final bool selected;
  final ValueChanged<T> onSelected;
  final double? previewAspectRatio;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final subtitle = option.subtitle;
    final ratio = previewAspectRatio;
    return MTappable(
      onPress: () => onSelected(option.value),
      selected: selected,
      semanticsLabel: subtitle == null
          ? option.label
          : '${option.label}. $subtitle',
      builder: (context, _) {
        final border = selected ? theme.colors.primary : theme.colors.border;
        return DecoratedBox(
          decoration: BoxDecoration(
            border: Border.all(color: border, width: selected ? 1.5 : 1),
            borderRadius: theme.style.borderRadius.md,
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            child: Row(
              children: [
                _RadioMark(selected: selected),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      ExcludeSemantics(
                        child: Text(
                          option.label,
                          style: theme.typography.body.sm.copyWith(
                            color: theme.colors.foreground,
                            fontWeight: selected
                                ? FontWeight.w600
                                : FontWeight.w500,
                          ),
                        ),
                      ),
                      if (subtitle != null) ...[
                        const SizedBox(height: 2),
                        ExcludeSemantics(
                          child: Text(
                            subtitle,
                            style: theme.typography.body.xs.copyWith(
                              color: theme.colors.mutedForeground,
                              height: 1.25,
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                SizedBox(
                  width: ratio == null ? 64 : 28 * ratio,
                  height: 28,
                  child: option.preview,
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _RadioMark extends StatelessWidget {
  const new({required this.selected});

  final bool selected;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Container(
      width: 16,
      height: 16,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        border: Border.all(
          color: selected ? theme.colors.primary : theme.colors.border,
          width: 1.5,
        ),
      ),
      child: Center(
        child: selected
            ? Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: theme.colors.primary,
                ),
              )
            : null,
      ),
    );
  }
}
