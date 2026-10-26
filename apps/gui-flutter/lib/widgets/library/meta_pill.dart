import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// A metadata chip under a comfortable row's title.
class MetaPill extends StatelessWidget {
  const new({
    required this.text,
    this.leading,
    this.textColor,
    this.backgroundColor,
    this.borderColor,
    this.fontWeight = FontWeight.w500,
    this.fillWidth = true,
    this.keepLeadingGlyph = false,
    super.key,
  });

  final String text;

  /// Optional leading widget, used by the metadata pills for their glyphs.
  final Widget? leading;

  /// Overrides the muted default, used by the key pill for its key colour.
  final Color? textColor;

  /// Overrides the default row-surface fill. Set this (with [borderColor]) when
  /// the pill sits on a `card` surface, where the default `card` fill would be
  /// invisible.
  final Color? backgroundColor;

  /// Optional hairline border drawn around the chip.
  final Color? borderColor;
  final FontWeight fontWeight;

  /// When true (default), expand to the parent's max width so trailing BPM/key
  /// columns stay aligned. When false, size to the label and only ellipsize
  /// under pressure — status chips use this so a wide max constraint cannot
  /// stretch an invisible card fill across the details column.
  final bool fillWidth;

  /// Keep [leading] even when the pill is too narrow for glyph + label.
  /// Rating uses this because the star is the value, not decoration.
  final bool keepLeadingGlyph;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final innerWidth = constraints.maxWidth - kMetaPillPadding.horizontal;
        // Drop the glyph in a squeezed pill: below this the icon alone consumes
        // the whole box and pushes the label into an overflow. The text carries
        // the meaning, so the icon is the part that goes.
        final showLeading =
            leading != null &&
            (keepLeadingGlyph || innerWidth >= kMetaPillIconMinWidth);
        // Hide instead of clip, and only when the chrome itself cannot fit: the
        // label would have no width to ellipsize into. A wider box keeps the
        // pill and lets the text ellipsize.
        final chrome =
            kMetaPillPadding.horizontal +
            (showLeading ? kMetaPillLeadingExtent : 0.0);
        if (constraints.maxWidth < chrome) {
          return const SizedBox.shrink();
        }
        final label = Text(
          text,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: theme.typography.body.xs.copyWith(
            color: textColor ?? theme.colors.mutedForeground,
            fontWeight: fontWeight,
          ),
        );
        // Leading budgets the label max when fillWidth is false.
        return DecoratedBox(
          decoration: BoxDecoration(
            color: backgroundColor ?? theme.colors.card,
            borderRadius: theme.style.borderRadius.pill,
            border: borderColor == null
                ? null
                : Border.all(color: borderColor!),
          ),
          child: Padding(
            padding: kMetaPillPadding,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (showLeading) ...[leading!, const SizedBox(width: 4)],
                // Flexible fills trailing meta slots (column alignment).
                // ConstrainedBox (not Flexible) keeps status chips intrinsic:
                // Flexible would expand the min-sized Row to the parent's max
                // width and stretch an invisible card fill across the details
                // column.
                if (fillWidth)
                  Flexible(child: label)
                else
                  ConstrainedBox(
                    constraints: BoxConstraints(
                      maxWidth:
                          (innerWidth -
                                  (showLeading ? kMetaPillLeadingExtent : 0.0))
                              .clamp(0.0, double.infinity),
                    ),
                    child: label,
                  ),
              ],
            ),
          ),
        );
      },
    );
  }
}
