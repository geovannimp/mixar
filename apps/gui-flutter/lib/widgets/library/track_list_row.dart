import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/widgets/library/track_list_menu_button.dart';
import 'package:gui_flutter/widgets/library/track_list_view.dart';
import 'package:gui_flutter/widgets/shell/mixar_context_menu.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// One list row: selection fill, right-click menu, drag-to-deck and dimming.
///
/// Public and non-generic so tests (and future panes) can find it by type. The
/// owning [TrackListView] supplies the content and menu via builders.
class TrackListRow extends ConsumerWidget {
  const new({
    required this.index,
    required this.payloadOf,
    required this.menuBuilder,
    required this.contentBuilder,
    required this.onSelect,
    this.overlay,
    this.dimmedOf,
    super.key,
  });

  final int index;
  final TrackDragPayload? Function(WidgetRef ref) payloadOf;
  final Widget Function(
    BuildContext context,
    WidgetRef ref,
    VoidCallback dismiss,
  )
  menuBuilder;
  final Widget Function(BuildContext context, WidgetRef ref, Widget actionsSlot)
  contentBuilder;
  final Widget? overlay;
  final bool Function(WidgetRef ref)? dimmedOf;
  final VoidCallback onSelect;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final focused = ref.watch(
      focusedTrackRowIndexProvider.select((i) => i == index),
    );
    // The cursor only wears the brand tint while this list owns browser focus;
    // otherwise it dims and the sidebar's focused row takes the accent.
    final listFocused =
        ref.watch(libraryFocusZoneProvider) == LibraryFocusZone.trackList;
    final content = DecoratedBox(
      decoration: BoxDecoration(
        color: !focused
            ? theme.colors.card
            : listFocused
            ? libraryListSelectedRowColor(theme)
            : libraryListCursorRowColor(theme),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6),
        child: contentBuilder(
          context,
          ref,
          _RowActionsSlot(menuBuilder: menuBuilder),
        ),
      ),
    );
    final stacked = overlay == null
        ? content
        : Stack(children: [content, overlay!]);
    final payload = payloadOf(ref);
    // Watch engine here so drag attaches after start without rebuilding the
    // surrounding list.
    final row = payload != null && ref.watch(engineRunningProvider)
        ? libraryTrackDragSource(payload: payload, child: stacked)
        : stacked;
    final dimmed = dimmedOf?.call(ref) ?? false;
    // Pointer-down (not tap): super_dnd's drag recognizer often wins the
    // gesture arena, so a tap handler never fires on a drag. Any button
    // selects, so the pressed row is the one highlighted as it is dragged and
    // the one a controller "load focused row" command acts on.
    return Listener(
      behavior: HitTestBehavior.translucent,
      onPointerDown: (_) => onSelect(),
      child: _RowContextMenu(
        menuBuilder: menuBuilder,
        child: AnimatedOpacity(
          opacity: dimmed ? kSessionPlayedRowOpacity : 1,
          duration: const Duration(milliseconds: 120),
          child: row,
        ),
      ),
    );
  }
}

/// The 3-dot actions menu, right-aligned in the reserved trailing slot.
class _RowActionsSlot extends StatelessWidget {
  const new({required this.menuBuilder});

  final Widget Function(
    BuildContext context,
    WidgetRef ref,
    VoidCallback dismiss,
  )
  menuBuilder;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: kActionsColumnWidth,
      child: Center(
        // The row's context menu already handles secondary-press, so the
        // button only owns the primary toggle.
        child: TrackListMenuButton(
          menuBuilder: menuBuilder,
          enableSecondaryPress: false,
        ),
      ),
    );
  }
}

/// Right-click / long-press menu wrapper. The menu body is built lazily so its
/// `ref.watch`es only run while it is open.
class _RowContextMenu extends StatelessWidget {
  const new({required this.menuBuilder, required this.child});

  final Widget Function(
    BuildContext context,
    WidgetRef ref,
    VoidCallback dismiss,
  )
  menuBuilder;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return MixarContextMenu(
      menuBuilder: (context, handle) => Consumer(
        builder: (context, ref, _) => menuBuilder(context, ref, handle.hide),
      ),
      childBuilder: (context, handle) => GestureDetector(
        onSecondaryTapDown: (details) => handle.showAt(details.globalPosition),
        onLongPressStart: (details) => handle.showAt(details.globalPosition),
        child: child,
      ),
    );
  }
}
