import 'package:flutter/widgets.dart';
import 'package:gui_flutter/widgets/shell/app_tooltip.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Width a reorderable row reserves for [TrackReorderHandle].
const kReorderHandleWidth = 20.0;

/// Drag grip that moves a row within a sortable playlist.
///
/// It sits outside the row's deck drag source: the body still drags a track
/// onto a deck, the grip only reorders. [ReorderableDragStartListener] wires
/// the grip to the enclosing `ReorderableListView`, so no callback is needed
/// here.
class TrackReorderHandle extends StatefulWidget {
  const new({required this.index, super.key});

  /// Row index the enclosing list is reordering.
  final int index;

  @override
  State<TrackReorderHandle> createState() => _TrackReorderHandleState();
}

class _TrackReorderHandleState extends State<TrackReorderHandle> {
  var _hovering = false;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return AppTooltip(
      tip: 'Drag to reorder',
      child: ReorderableDragStartListener(
        index: widget.index,
        child: MouseRegion(
          cursor: SystemMouseCursors.grab,
          onEnter: (_) => setState(() => _hovering = true),
          onExit: (_) => setState(() => _hovering = false),
          child: SizedBox(
            width: kReorderHandleWidth,
            child: Center(
              child: Icon(
                LucideIcons.gripVertical,
                size: 14,
                color: _hovering ? colors.foreground : colors.mutedForeground,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
