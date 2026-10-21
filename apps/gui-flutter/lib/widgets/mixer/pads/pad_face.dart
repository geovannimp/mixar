import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Shared small-caps treatment for a pad's action caption.
TextStyle padActionStyle(MixarThemeData theme) => theme.typography.mono.xs2
    .copyWith(fontWeight: FontWeight.w500, letterSpacing: 1.4);

/// Two-part pad face: a header strip (action caption left, status dot right)
/// over a body that centres its [label] in the remaining space.
///
/// The header is omitted when neither [action] nor [indicatorColor] is set, so
/// simple pads keep their label centred in the full pad.
///
/// Text renders at its nominal size whenever it fits; [FittedBox] shrinks it
/// only on pads too narrow to hold it. Pads must be laid out with tight
/// constraints (`PadButton.fillChild`).
class PadFace extends StatelessWidget {
  const new({
    required this.label,
    this.action,
    this.actionColor,
    this.indicatorColor,
    this.labelWidth,
    this.footer,
    super.key,
  });

  /// Centred body content.
  final Widget label;

  /// Optional uppercase caption in the top-left corner.
  final String? action;

  /// Caption colour; defaults to the inherited foreground.
  final Color? actionColor;

  /// Optional top-right status dot; omitted when null.
  final Color? indicatorColor;

  /// Fixed body width. Sharing one value across a grid keeps every pad's
  /// [FittedBox] on the same scale instead of shrinking only the wide labels.
  final double? labelWidth;

  /// Optional content pinned to the bottom-right of the body area.
  final Widget? footer;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final action = this.action;
    final indicatorColor = this.indicatorColor;
    // The mono face gives every four-character caption the same width, so a
    // shared slot makes all captions scale to exactly one size.
    final actionWidth = action == null ? 0.0 : _actionSlot(context, theme);

    final header = action == null && indicatorColor == null
        ? null
        : Row(
            children: [
              if (action == null)
                const Spacer()
              else
                Expanded(
                  child: FittedBox(
                    fit: BoxFit.scaleDown,
                    alignment: Alignment.centerLeft,
                    child: SizedBox(
                      width: actionWidth,
                      child: Text(
                        action,
                        style: padActionStyle(theme).copyWith(
                          color: actionColor ?? theme.colors.mutedForeground,
                        ),
                      ),
                    ),
                  ),
                ),
              // The dot slot is reserved on every pad, with or without a
              // status dot, so the caption always gets the same width.
              const SizedBox(width: 6),
              SizedBox(
                width: 8,
                height: 8,
                child: indicatorColor == null
                    ? null
                    : DecoratedBox(
                        decoration: BoxDecoration(
                          shape: BoxShape.circle,
                          color: indicatorColor,
                        ),
                      ),
              ),
            ],
          );

    return Padding(
      padding: const EdgeInsets.all(8),
      child: Column(
        children: [
          ?header,
          Expanded(
            child: Stack(
              children: [
                Positioned.fill(
                  // Keep the centred label above the footer so they never
                  // overlap on small pads.
                  child: Padding(
                    padding: EdgeInsets.only(bottom: footer == null ? 0 : 14),
                    child: Center(
                      child: FittedBox(
                        fit: BoxFit.scaleDown,
                        child: labelWidth == null
                            ? label
                            : SizedBox(width: labelWidth, child: label),
                      ),
                    ),
                  ),
                ),
                if (footer != null)
                  Positioned(right: 0, bottom: 0, child: footer!),
              ],
            ),
          ),
        ],
      ),
    );
  }

  /// Width of the longest caption at the nominal type scale.
  ///
  /// The mono face gives equal-length captions equal width, so one slot keeps
  /// every caption on one scale. Uses the longest caption currently shipped
  /// (`SAMPLE`).
  double _actionSlot(BuildContext context, MixarThemeData theme) {
    final painter = TextPainter(
      text: TextSpan(text: 'SAMPLE', style: padActionStyle(theme)),
      textDirection: TextDirection.ltr,
      textScaler: MediaQuery.textScalerOf(context),
    )..layout();
    final width = painter.width;
    painter.dispose();
    return width.ceilToDouble();
  }
}
