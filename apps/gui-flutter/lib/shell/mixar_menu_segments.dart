import 'package:flutter/widgets.dart';
import 'package:gui_flutter/shell/m_divider.dart';
import 'package:gui_flutter/shell/m_tappable.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';

/// One cell of a [MixarMenuSegments] group.
@immutable
class MixarMenuSegment {
  const new({
    required this.label,
    this.onPress,
    this.semanticsLabel,
    this.color,
  }) : assert(
         label.length > 2 || semanticsLabel != null,
         'Short labels like "A"/"B" need a semanticsLabel: the visible text '
         'alone is not a meaningful accessible name.',
       );

  /// Visible cell text. May be a single letter, in which case
  /// [semanticsLabel] is required.
  final String label;

  /// `null` renders the segment disabled.
  final VoidCallback? onPress;

  /// Accessible name for the segment. Required whenever [label] is a short
  /// glyph ("A"/"B") that a screen reader could not interpret on its own.
  final String? semanticsLabel;

  /// Label colour override, e.g. the deck A/B accents. Falls back to the menu
  /// foreground.
  final Color? color;
}

/// A segmented group of menu cells: one bordered, rounded track split by
/// hairlines, matching the shuffle/repeat button groups. Full width, so
/// segments stay vertically aligned with the text actions around them.
class MixarMenuSegments extends StatelessWidget {
  const new({required this.segments, super.key});

  /// Horizontal inset matching MixarMenuItem's text rows (12px), so the
  /// group's track starts at the same x as "Load to deck" and the other
  /// actions.
  static const _inset = 12.0;

  final List<MixarMenuSegment> segments;

  @override
  Widget build(BuildContext context) {
    if (segments.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = context.theme;
    final radius = theme.style.borderRadius.sm;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: _inset, vertical: 4),
      child: DecoratedBox(
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
                for (var i = 0; i < segments.length; i++) ...[
                  if (i > 0)
                    MDivider(
                      axis: Axis.vertical,
                      padding: EdgeInsets.zero,
                      color: theme.colors.border,
                    ),
                  Expanded(child: _Segment(segment: segments[i])),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _Segment extends StatelessWidget {
  const new({required this.segment});

  final MixarMenuSegment segment;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final enabled = segment.onPress != null;
    final color = segment.color ?? theme.colors.foreground;
    return MTappable(
      onPress: segment.onPress,
      semanticsLabel: segment.semanticsLabel,
      builder: (context, state) {
        // `state.active` is `!disabled && (hovered || pressed || selected)`, so
        // a disabled segment never paints the highlight.
        return ColoredBox(
          color: state.active
              ? theme.colors.secondary
              : const Color(0x00000000),
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Center(
              child: Text(
                segment.label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w600,
                  color: enabled ? color : color.withValues(alpha: 0.4),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
