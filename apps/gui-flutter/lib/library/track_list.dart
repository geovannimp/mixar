import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/library_list_chrome.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/mixar_context_menu.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Opacity applied to rows already committed in the open history session.
const kSessionPlayedRowOpacity = 0.3;

/// The one list every place that shows tracks renders through — the library
/// list and the history detail list (and anything added later).
///
/// It owns the shared behaviors: selection, keyboard navigation, drag-to-deck,
/// the row actions menu, scroll-into-view and the artwork-window hook. Call
/// sites own their row *content* via [rowBuilder] and their source-specific
/// menu items via [extraMenuItems], so behaviors cannot drift between places.
class TrackListView<T> extends ConsumerStatefulWidget {
  const new({
    required this.items,
    required this.idOf,
    required this.payloadOf,
    required this.rowBuilder,
    required this.emptyBuilder,
    this.skipLoadingOnReload = false,
    this.extraMenuItems,
    this.overlayBuilder,
    this.dimmedOf,
    this.onVisibleRange,
    this.errorBuilder,
    super.key,
  });

  /// Rows to render. Loading / error / empty states are handled here.
  final AsyncValue<List<T>> items;

  /// Stable row identity, used for keys and to pin the focused row across a
  /// re-sort or filter change.
  final String Function(T item) idOf;

  /// Drag payload for a row. `null` disables dragging that row.
  final TrackDragPayload? Function(WidgetRef ref, T item) payloadOf;

  /// Builds a row's *content* (leading, title, pills, trailing meta). The
  /// shared interaction layers wrap it and `actionsSlot` must be placed in the
  /// row's trailing slot.
  final Widget Function(
    BuildContext context,
    WidgetRef ref,
    int index,
    T item,
    LibraryRowDensity density,
    Widget actionsSlot,
  )
  rowBuilder;

  /// Shown when there are no rows to render.
  final Widget Function(BuildContext context) emptyBuilder;

  /// Keep rendering the previous rows while a refreshed [items] is in flight,
  /// instead of swapping to the loader. Defaults to `false` (the library list's
  /// original semantics); the history list opts in.
  final bool skipLoadingOnReload;

  /// Extra menu items appended after the shared "Load to deck" group.
  final List<Widget> Function(
    BuildContext context,
    WidgetRef ref,
    T item,
    VoidCallback dismiss,
  )?
  extraMenuItems;

  /// Per-row overlay drawn over the content (analysis / stem status).
  final Widget? Function(T item)? overlayBuilder;

  /// Whether a row is dimmed.
  final bool Function(WidgetRef ref, T item)? dimmedOf;

  /// Called with the first/last visible index, for artwork prefetch.
  final void Function(int first, int last)? onVisibleRange;

  /// Error state; defaults to a destructive message.
  final Widget Function(BuildContext context, Object error)? errorBuilder;

  @override
  ConsumerState<TrackListView<T>> createState() => _TrackListViewState<T>();
}

class _TrackListViewState<T> extends ConsumerState<TrackListView<T>> {
  final ScrollController _scroll = ScrollController();

  /// Owned explicitly so the list can hold keyboard focus without relying on
  /// `autofocus`, and so a pane can hand the node to tests.
  final FocusNode _focusNode = FocusNode();
  List<T> _items = const [];

  /// Latest requested scroll target, and whether a frame callback is already
  /// queued to service it. See [_scrollToRow].
  int? _pendingScrollIndex;
  bool _scrollScheduled = false;

  /// Track the focused row is pinned to by id, not by index. A re-sort or a
  /// filter change reorders the list underneath the focus, and without this the
  /// highlighted row would silently jump to whatever now sits at the old index.
  String? _focusedId;

  /// Captured in `initState` because `ref` is unsafe to use from `dispose`.
  FocusedTrackPayload? _focusedPayload;

  /// Identifies this list as the payload owner, and the mount that owns it.
  final Object _payloadOwner = Object();

  @override
  void initState() {
    super.initState();
    _focusedPayload = ref.read(focusedTrackPayloadProvider);
    _scroll.addListener(_requestVisibleRange);
    // The first result set arrives after this widget's first build, so the row
    // count and focus pin are applied in a post-frame callback. Doing it inline
    // from `build` would mutate a provider mid-build.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _syncFromItems();
      }
    });
  }

  @override
  void didUpdateWidget(TrackListView<T> oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.items, widget.items)) {
      // Deferred for the same reason as `initState`.
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          _syncFromItems();
        }
      });
    }
  }

  @override
  void dispose() {
    // Relinquish the shared payload so a controller "load focused row" after
    // this list is gone no-ops instead of loading the last highlighted row.
    // Plain field writes: a provider must not be modified during teardown.
    final holder = _focusedPayload;
    if (holder != null && identical(holder.owner, _payloadOwner)) {
      holder.owner = null;
      holder.payload = null;
    }
    _scroll.removeListener(_requestVisibleRange);
    _scroll.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  /// Row height is constant per density, so the visible window is plain offset
  /// arithmetic.
  void _requestVisibleRange() {
    if (_items.isEmpty || !_scroll.hasClients) {
      return;
    }
    final density = ref.read(libraryRowDensityProvider);
    final first = (_scroll.offset / density.height).floor().clamp(
      0,
      _items.length - 1,
    );
    final count =
        (_scroll.position.viewportDimension / density.height).ceil() + 2;
    final last = (first + count).clamp(0, _items.length);
    widget.onVisibleRange?.call(first, last);
  }

  /// Take a new result set: update the row count, re-pin focus onto the
  /// previously focused row, and publish the focused payload.
  void _syncFromItems() {
    ref.read(focusedTrackRowIndexProvider.notifier).setCount(_items.length);
    _restoreFocusedId();
    _publishFocusedPayload();
    _requestVisibleRange();
  }

  void _select(int index) {
    ref.read(focusedTrackRowIndexProvider.notifier).set(index);
  }

  /// Remember which row holds focus, so a re-sort or a new filter result can
  /// put it back on that same row rather than leaving the highlight on whatever
  /// now sits at the old index.
  void _pinFocusedId(int index) {
    if (index < 0 || index >= _items.length) {
      return;
    }
    _focusedId = widget.idOf(_items[index]);
    _publishFocusedPayload();
  }

  void _restoreFocusedId() {
    final id = _focusedId;
    if (id == null || _items.isEmpty) {
      return;
    }
    final next = _items.indexWhere((item) => widget.idOf(item) == id);
    if (next < 0) {
      // The focused row was filtered out; fall back to the top rather than
      // pointing at an index that no longer means it.
      ref.read(focusedTrackRowIndexProvider.notifier).set(0);
      return;
    }
    if (next != ref.read(focusedTrackRowIndexProvider)) {
      ref.read(focusedTrackRowIndexProvider.notifier).set(next);
    }
  }

  /// Publish the focused row's payload so controller "load focused row" works
  /// for whichever list is on screen.
  void _publishFocusedPayload() {
    final holder = _focusedPayload;
    if (holder == null) {
      return;
    }
    final index = ref.read(focusedTrackRowIndexProvider);
    holder.owner = _payloadOwner;
    holder.payload = (index >= 0 && index < _items.length)
        ? widget.payloadOf(ref, _items[index])
        : null;
  }

  /// Keyboard focus movement. [delta] is null for the absolute Home/End cases,
  /// where [edge] picks which end of the list to land on. The notifier clamps,
  /// so a stale row count cannot walk focus past either end.
  void _moveFocus(int? delta, _FocusEdge edge) {
    final focus = ref.read(focusedTrackRowIndexProvider.notifier);
    if (delta != null) {
      focus.navigate(delta);
      return;
    }
    final count = _items.length;
    focus.set(
      edge == _FocusEdge.first
          ? 0
          : count == 0
          ? 0
          : count - 1,
    );
  }

  void _loadFocusedDeck(int deckId) {
    final index = ref.read(focusedTrackRowIndexProvider);
    if (index < 0 || index >= _items.length) {
      return;
    }
    final payload = widget.payloadOf(ref, _items[index]);
    if (payload == null) {
      return;
    }
    unawaited(loadPayloadToDeck(ref, deckId, payload));
  }

  /// Bring [index] into view without jumping past a row the user can still see.
  ///
  /// Deferred to the next frame: the row extent comes from the density
  /// provider, and changing it (the toolbar's density toggle) leaves the
  /// controller measuring the *old* itemExtent until the list re-lays out.
  /// Scrolling synchronously would compute the target against a stale extent
  /// and land on the wrong row.
  ///
  /// Coalesced: rapid focus moves (held arrow key) or a focus change plus a
  /// density change in one frame would otherwise queue one callback each, and
  /// every stale target would still be jumped to.
  void _scrollToRow(int index) {
    if (index < 0) {
      return;
    }
    _pendingScrollIndex = index;
    if (_scrollScheduled) {
      return;
    }
    _scrollScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollScheduled = false;
      final target = _pendingScrollIndex;
      _pendingScrollIndex = null;
      if (mounted && target != null) {
        _scrollToRowNow(target);
      }
    });
  }

  void _scrollToRowNow(int index) {
    if (!_scroll.hasClients) {
      return;
    }
    final height = ref.read(libraryRowDensityProvider).height;
    final target = index * height;
    final position = _scroll.position;
    if (target < position.pixels) {
      _scroll.jumpTo(target.clamp(0.0, position.maxScrollExtent));
    } else if (target + height > position.pixels + position.viewportDimension) {
      _scroll.jumpTo(
        (target + height - position.viewportDimension).clamp(
          0.0,
          position.maxScrollExtent,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final density = ref.watch(libraryRowDensityProvider);

    ref.listen(focusedTrackRowIndexProvider, (_, index) {
      _scrollToRow(index);
      _pinFocusedId(index);
    });
    // A density switch changes every row's extent, so the focused row can end
    // up outside the viewport even though the offset is unchanged.
    ref.listen(libraryRowDensityProvider, (_, _) {
      _scrollToRow(ref.read(focusedTrackRowIndexProvider));
    });

    return widget.items.when(
      skipLoadingOnReload: widget.skipLoadingOnReload,
      loading: () => const Center(child: MLoader()),
      error: (e, _) =>
          widget.errorBuilder?.call(context, e) ??
          LibraryListMessage('$e', color: theme.colors.destructive),
      data: (items) {
        // Assigned here too so the very first paint already has the list
        // available for focus and the visible-window hook.
        _items = items;
        if (items.isEmpty) {
          return widget.emptyBuilder(context);
        }
        // Shortcuts and Actions wrap the focusable, never the other way round:
        // key events are dispatched from the focused node *up* the focus tree,
        // so a Shortcuts widget below the focused node never sees them.
        return LibraryListSurface(
          theme: theme,
          child: RepaintBoundary(
            child: Shortcuts(
              shortcuts: const {
                SingleActivator(LogicalKeyboardKey.arrowDown):
                    _MoveDownIntent(),
                SingleActivator(LogicalKeyboardKey.arrowUp): _MoveUpIntent(),
                SingleActivator(LogicalKeyboardKey.home): _MoveFirstIntent(),
                SingleActivator(LogicalKeyboardKey.end): _MoveLastIntent(),
                SingleActivator(LogicalKeyboardKey.enter): _LoadDeckAIntent(),
                SingleActivator(LogicalKeyboardKey.enter, shift: true):
                    _LoadDeckBIntent(),
              },
              child: Actions(
                actions: {
                  _MoveDownIntent: CallbackAction<_MoveDownIntent>(
                    onInvoke: (_) {
                      _moveFocus(1, _FocusEdge.first);
                      return null;
                    },
                  ),
                  _MoveUpIntent: CallbackAction<_MoveUpIntent>(
                    onInvoke: (_) {
                      _moveFocus(-1, _FocusEdge.first);
                      return null;
                    },
                  ),
                  _MoveFirstIntent: CallbackAction<_MoveFirstIntent>(
                    onInvoke: (_) {
                      _moveFocus(null, _FocusEdge.first);
                      return null;
                    },
                  ),
                  _MoveLastIntent: CallbackAction<_MoveLastIntent>(
                    onInvoke: (_) {
                      _moveFocus(null, _FocusEdge.last);
                      return null;
                    },
                  ),
                  _LoadDeckAIntent: CallbackAction<_LoadDeckAIntent>(
                    onInvoke: (_) {
                      _loadFocusedDeck(0);
                      return null;
                    },
                  ),
                  _LoadDeckBIntent: CallbackAction<_LoadDeckBIntent>(
                    onInvoke: (_) {
                      _loadFocusedDeck(1);
                      return null;
                    },
                  ),
                },
                child: Focus(
                  focusNode: _focusNode,
                  // A list has no built-in focus ring; the focused row fill is
                  // the cue. Focus is requested on row press rather than via
                  // autofocus, so typing in the filter box does not steal the
                  // arrow keys.
                  child: ListView.builder(
                    controller: _scroll,
                    itemExtent: density.height,
                    padding: EdgeInsets.zero,
                    itemCount: items.length,
                    itemBuilder: (context, index) {
                      final item = items[index];
                      return TrackListRow(
                        key: ValueKey(widget.idOf(item)),
                        index: index,
                        payloadOf: (ref) => widget.payloadOf(ref, item),
                        menuBuilder: (context, ref, dismiss) =>
                            _menuBody(context, ref, item, dismiss),
                        contentBuilder: (context, ref, actionsSlot) =>
                            widget.rowBuilder(
                              context,
                              ref,
                              index,
                              item,
                              density,
                              actionsSlot,
                            ),
                        overlay: widget.overlayBuilder?.call(item),
                        dimmedOf: widget.dimmedOf == null
                            ? null
                            : (ref) => widget.dimmedOf!(ref, item),
                        onSelect: () {
                          // Clicking a row also grants the list keyboard focus,
                          // so the arrow keys keep working after a press.
                          _focusNode.requestFocus();
                          _select(index);
                        },
                      );
                    },
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  /// Shared menu body: "Load to deck" plus the caller's extra items.
  Widget _menuBody(
    BuildContext context,
    WidgetRef ref,
    T item,
    VoidCallback dismiss,
  ) {
    return buildTrackListMenuBody(
      context: context,
      ref: ref,
      payload: widget.payloadOf(ref, item),
      extras:
          widget.extraMenuItems?.call(context, ref, item, dismiss) ??
          const <Widget>[],
      dismiss: dismiss,
    );
  }
}

/// The shared row menu body: a "Load to deck" group (engine-gated) followed by
/// the caller's [extras]. Public so panes and tests compose the same menu.
Widget buildTrackListMenuBody({
  required BuildContext context,
  required WidgetRef ref,
  required TrackDragPayload? payload,
  required List<Widget> extras,
  required VoidCallback dismiss,
}) {
  final engineRunning = ref.watch(engineRunningProvider);
  void load(int deckId) {
    if (payload == null) {
      return;
    }
    unawaited(loadPayloadToDeck(ref, deckId, payload));
  }

  return MixarMenuBody(
    groups: [
      MixarMenuGroup(
        children: [
          const MixarMenuLabel(child: Text('Load to deck')),
          MixarMenuSegments(
            segments: [
              MixarMenuSegment(
                label: 'A',
                color: FaderColors.a.grip,
                semanticsLabel: 'Load to A',
                onPress: engineRunning && payload != null
                    ? () {
                        dismiss();
                        load(0);
                      }
                    : null,
              ),
              MixarMenuSegment(
                label: 'B',
                color: FaderColors.b.grip,
                semanticsLabel: 'Load to B',
                onPress: engineRunning && payload != null
                    ? () {
                        dismiss();
                        load(1);
                      }
                    : null,
              ),
            ],
          ),
        ],
      ),
      if (extras.isNotEmpty) MixarMenuGroup(children: extras),
    ],
  );
}

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
    final content = DecoratedBox(
      decoration: BoxDecoration(
        color: focused ? libraryListSelectedRowColor(theme) : theme.colors.card,
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

/// The 3-dot row menu button. The menu body is built lazily so its
/// `ref.watch`es only run while it is open. Public so a pane or test can mount
/// the menu on its own, not just inside a [TrackListView].
class TrackListMenuButton extends StatelessWidget {
  const new({
    required this.menuBuilder,
    this.enableSecondaryPress = true,
    super.key,
  });

  final Widget Function(
    BuildContext context,
    WidgetRef ref,
    VoidCallback dismiss,
  )
  menuBuilder;
  final bool enableSecondaryPress;

  @override
  Widget build(BuildContext context) {
    return MixarMenuAnchor(
      menuBuilder: (context, controller) => Consumer(
        builder: (context, ref, _) =>
            menuBuilder(context, ref, controller.hide),
      ),
      childBuilder: (context, controller) => AppButton.icon(
        variant: .ghost,
        size: .xs,
        semanticsLabel: 'Track actions',
        onPress: controller.toggle,
        onSecondaryPress: enableSecondaryPress ? controller.toggle : null,
        child: const Icon(LucideIcons.ellipsisVertical),
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

/// Which end of the list an absolute focus move (Home / End) targets.
enum _FocusEdge { first, last }

/// Keyboard intents for the list. Focus movement and deck load are separate
/// intent types because a `CallbackAction` resolves to one concrete type.
class _MoveUpIntent extends Intent {
  const new();
}

class _MoveDownIntent extends Intent {
  const new();
}

class _MoveFirstIntent extends Intent {
  const new();
}

class _MoveLastIntent extends Intent {
  const new();
}

class _LoadDeckAIntent extends Intent {
  const new();
}

class _LoadDeckBIntent extends Intent {
  const new();
}
