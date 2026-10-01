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
  });

  final String label;

  /// `null` renders the segment disabled.
  final VoidCallback? onPress;
  final String? semanticsLabel;

  /// Label colour override, e.g. the deck A/B accents. Falls back to the menu
  /// foreground.
  final Color? color;
}

/// A segmented group of menu cells: one bordered, rounded track split by
/// hairlines, matching the shuffle/repeat button groups. Full width, so
/// segments stay vertically aligned with the text actions around them.
class MixarMenuSegments extends StatefulWidget {
  const new({required this.segments, super.key});

  final List<MixarMenuSegment> segments;

  @override
  State<MixarMenuSegments> createState() => _MixarMenuSegmentsState();
}

class _MixarMenuSegmentsState extends State<MixarMenuSegments> {
  var _hovered = -1;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final radius = theme.style.borderRadius.sm;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
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
                for (var i = 0; i < widget.segments.length; i++) ...[
                  if (i > 0)
                    MDivider(
                      axis: Axis.vertical,
                      padding: EdgeInsets.zero,
                      color: theme.colors.border,
                    ),
                  Expanded(
                    child: _Segment(
                      segment: widget.segments[i],
                      hovered: _hovered == i,
                      onHover: (value) =>
                          setState(() => _hovered = value ? i : -1),
                    ),
                  ),
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
  const new({
    required this.segment,
    required this.hovered,
    required this.onHover,
  });

  final MixarMenuSegment segment;
  final bool hovered;
  final ValueChanged<bool> onHover;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final enabled = segment.onPress != null;
    final color = segment.color ?? theme.colors.foreground;
    return MTappable(
      onPress: segment.onPress,
      semanticsLabel: segment.semanticsLabel,
      builder: (context, state) {
        return MouseRegion(
          onEnter: (_) => onHover(true),
          onExit: (_) => onHover(false),
          child: ColoredBox(
            color: state.active || hovered
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
          ),
        );
      },
    );
  }
}
