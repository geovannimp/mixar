import 'dart:async';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_detail_dialog.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/app_tooltip.dart';
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/mixar_context_menu.dart';
import 'package:gui_flutter/shell/mixar_input.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

/// Focused-row fill. Forui neutral dark uses the same hex for `muted` and
/// `secondary`, so `theme.colors.muted` is invisible on the list surface.
Color libraryListSelectedRowColor(MixarThemeData theme) => Color.alphaBlend(
  theme.colors.primary.withValues(alpha: 0.14),
  theme.colors.secondary,
);

/// Opacity applied to rows already committed in the open history session.
const kSessionPlayedRowOpacity = 0.3;

/// Trailing actions column width. The row status overlay keeps its pill clear
/// of it, so the two must not drift apart.
const kActionsColumnWidth = 44.0;

/// Gap between the status pill group and the actions button, and between
/// adjacent pills.
const kStatusPillGap = 8.0;

/// Progress bar thickness, and the gap that keeps stacked per-job bars apart.
/// The vertical stride is their sum, so changing one stays consistent.
const kStatusBarHeight = 2.0;
const kStatusBarGap = 1.0;

// --- Row layout ---
// Fixed trailing widths keep BPM / key aligned down the list in both
// densities, which is what made the old column table scannable. The title (and
// the pills in comfortable) take whatever is left.
const kRowGutter = 8.0;
const kArtworkGap = 10.0;

// Maxima for the trailing pills, sized for the widest value plus its glyph and
// padding. They are maxima, not fixed widths: `trailingMetaWidth` shrinks the
// group in a narrow pane instead of overflowing, and the pill labels ellipsize.
// Measured at body.xs: "109.7 BPM" is ~110px and a 3-char key ~37px, each plus
// 32px of pill chrome (padding + glyph + gap). Slack is deliberately small —
// these are maxima for a wide pane, and `trailingMetaWidth` shrinks them below
// that when it must.
const kDurationSlotWidth = 84.0;
const kBpmSlotWidth = 144.0;
const kKeySlotWidth = 72.0;

// Flex weights matching the widths above. Equal flexes would split the meta
// evenly, starving the wider BPM pill and ellipsizing it even in a wide pane;
// weighting by natural width gives each its full size and shrinks them
// proportionally when the pane is narrow.
const kDurationSlotFlex = 84;
const kBpmSlotFlex = 144;
const kKeySlotFlex = 72;

/// Natural width of the trailing group without the duration pill. Comfortable
/// uses this; compact adds [kDurationSlotWidth] and a gap on top.
const kTrailingMetaBaseWidth = kBpmSlotWidth + kMetaPillGap + kKeySlotWidth;

/// Narrowest the track name may get before the trailing meta gives width back.
/// Below it the row would be all metadata and no track name.
const kMinTitleWidth = 64.0;

/// Identifies the right-hand BPM/key pill group, so a test can scope to it
/// rather than to the metadata pills on the left.
const kTrailingMetaKey = ValueKey<String>('libraryTrailingMeta');

/// Width for the trailing meta at [available] px of row: its natural size
/// whenever the row is wide enough, shrinking only once the title would drop
/// below [kMinTitleWidth].
///
/// Computed rather than left to flex, because `Expanded` title + `Flexible`
/// meta splits the free space evenly — the meta would claim half the row and
/// the title would wrap its pills at widths that have plenty of room.
double trailingMetaWidth(
  double available,
  double artSize, {
  double natural = kTrailingMetaBaseWidth,
}) {
  final budget =
      available -
      artSize -
      kArtworkGap -
      kRowGutter -
      kActionsColumnWidth -
      kMinTitleWidth;
  return budget.clamp(0.0, natural);
}

// Comfortable metadata pills: subtle chips under the title.
const kMetaPillGap = 6.0;
const kMetaPillPadding = EdgeInsets.symmetric(horizontal: 8, vertical: 2);

/// Height reserved for pills: two runs of a ~20px pill plus one 6px gap.
const kMetaPillAreaHeight = 46.0;

/// Widest a pill may get before its padding leaves no room for a glyph.
const kMetaPillIconMinWidth = 40.0;

/// Filter + sortable track list.
class TrackListPane extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<TrackListPane> createState() => _TrackListPaneState();
}

class _TrackListPaneState extends ConsumerState<TrackListPane> {
  final ScrollController _scroll = ScrollController();

  /// Owned explicitly so the list can hold keyboard focus without relying on
  /// `autofocus`, and so the pane can hand the node to tests.
  final FocusNode _focusNode = FocusNode();
  List<LibraryTrackSummary> _tracks = const [];

  /// Latest requested scroll target, and whether a frame callback is already
  /// queued to service it. See [_scrollToRow].
  int? _pendingScrollIndex;
  bool _scrollScheduled = false;

  /// Track the focused row is pinned to by id, not by index. A re-sort or a
  /// filter change reorders the list underneath the focus, and without this the
  /// highlighted row would silently jump to whatever track now sits at the old
  /// index.
  String? _focusedTrackId;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_requestVisibleArtwork);
    // The first result set arrives after this widget's first build, so the
    // row count and focus pin are applied in a post-frame callback. Doing it
    // inline from `build` would mutate a provider mid-build.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _syncFromTracks();
      }
    });
  }

  @override
  void dispose() {
    _scroll.removeListener(_requestVisibleArtwork);
    _scroll.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  /// Artwork prefetch for the visible window. Row height is constant per
  /// density, so the window is plain offset arithmetic.
  void _requestVisibleArtwork() {
    if (_tracks.isEmpty || !_scroll.hasClients) {
      return;
    }
    final density = ref.read(libraryRowDensityProvider);
    final tab = ref.read(librarySourceTabProvider);
    final resolved =
        ref.read(driveResolvedByPathProvider).asData?.value ?? const {};
    final first = (_scroll.offset / density.height).floor().clamp(
      0,
      _tracks.length - 1,
    );
    final count =
        (_scroll.position.viewportDimension / density.height).ceil() + 2;
    final last = (first + count).clamp(0, _tracks.length);
    final visible = [
      for (var i = first; i < last; i++)
        if (trackIsInLibrary(
          _tracks[i],
          tab: tab,
          driveResolvedByPath: resolved,
        ))
          _tracks[i].id,
    ];
    if (visible.isEmpty) {
      return;
    }
    ref.read(artworkCacheProvider.notifier).ensureLoaded(visible);
  }

  @override
  Widget build(BuildContext context) {
    ref.watch(libraryEventsBootstrapProvider);
    final theme = context.theme;
    final selectedId = ref.watch(activeCollectionIdProvider);
    final drive = ref.watch(librarySourceTabProvider) == LibrarySourceTab.drive;
    final drivePath = ref.watch(driveCurrentPathProvider);
    final tracksAsync = ref.watch(libraryTableTracksProvider);
    final density = ref.watch(libraryRowDensityProvider);
    // Watched, not read: rows derive `inLibrary` from this, and hoisted above
    // the `when` below so the subscription is unconditional rather than
    // registered only while the track list happens to be in its data state.
    final focused = ref.watch(focusedTrackRowIndexProvider);
    final driveResolved =
        ref.watch(driveResolvedByPathProvider).asData?.value ??
        const <String, LibraryTrackSummary>{};
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s, orElse: defaultAppSettings);
    final keyDisplayMode = keyModeFromSettings(settings.keyDisplayMode);
    final keyColorMode = keyColorModeFromSettings(settings.keyColorMode);

    // No listener on analyzing / track progress: both render in the per-row
    // status overlay. Regenerating rows on every fraction tick rebuilt the list
    // on every stem report.

    ref.listen(focusedTrackRowIndexProvider, (_, index) {
      _scrollToRow(index);
      _pinFocusedTrackId(index);
    });
    // A density switch changes every row's extent, so the focused row can end
    // up outside the viewport even though the offset is unchanged.
    ref.listen(libraryRowDensityProvider, (_, _) {
      _scrollToRow(ref.read(focusedTrackRowIndexProvider));
    });
    // Runs after the current build, so the provider writes inside it are safe.
    ref.listen(libraryTableTracksProvider, (_, _) => _syncFromTracks());

    return Padding(
      padding: const EdgeInsets.fromLTRB(6, 0, 0, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: MixarInput(
                  hint: 'Filter tracks…',
                  onChanged: (value) =>
                      ref.read(trackFilterProvider.notifier).set(value),
                ),
              ),
              const SizedBox(width: 6),
              _SortMenu(),
              const SizedBox(width: 4),
              _DensityToggleButton(density: density),
            ],
          ),
          const SizedBox(height: 8),
          Expanded(
            child: !drive && selectedId == null
                ? _Message(
                    'Select a collection',
                    color: theme.colors.mutedForeground,
                  )
                : drive && drivePath == null
                ? _Message(
                    'Select a drive or folder to browse audio files',
                    color: theme.colors.mutedForeground,
                  )
                : tracksAsync.when(
                    loading: () => const Center(child: MLoader()),
                    error: (e, _) => _Message(
                      'Tracks error: $e',
                      color: theme.colors.destructive,
                    ),
                    data: (tracks) {
                      // `initState`'s post-frame callback and `ref.listen` both
                      // feed `_tracks`; assign here too so the very first paint
                      // already has the list available for focus and artwork.
                      _tracks = tracks;
                      if (tracks.isEmpty) {
                        return _Message(
                          drive ? 'No audio files in this folder' : 'No tracks',
                          color: theme.colors.mutedForeground,
                        );
                      }
                      final tab = ref.read(librarySourceTabProvider);
                      return _ListSurface(
                        theme: theme,
                        child: RepaintBoundary(
                          // Isolate list paint from overlay tooltips / meters.
                          child: LibraryListScope(
                            tab: tab,
                            resolved: driveResolved,
                            child: _FocusScope(
                              index: focused,
                              child: _FocusableTrackList(
                                tracks: tracks,
                                density: density,
                                keyDisplayMode: keyDisplayMode,
                                keyColorMode: keyColorMode,
                                controller: _scroll,
                                onSelect: _selectRow,
                                onLoadDeck: (index, deckId) => unawaited(
                                  loadListRowToDeck(ref, index, deckId),
                                ),
                                onMove: _moveFocus,
                                focusNode: _focusNode,
                              ),
                            ),
                          ),
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  /// Take a new result set: update the row count, re-pin focus onto the
  /// previously focused track, and prefetch artwork for the new window.
  void _syncFromTracks() {
    final tracks = ref.read(libraryTableTracksProvider).asData?.value;
    if (tracks == null) {
      return;
    }
    _tracks = tracks;
    ref.read(focusedTrackRowIndexProvider.notifier).setCount(tracks.length);
    _restoreFocusedTrack();
    _requestVisibleArtwork();
  }

  void _selectRow(int index) {
    ref.read(focusedTrackRowIndexProvider.notifier).set(index);
  }

  /// Remember which track holds focus, so a re-sort or a new filter result can
  /// put it back on that same track rather than leaving the highlight on
  /// whatever now sits at the old index.
  void _pinFocusedTrackId(int index) {
    if (index < 0 || index >= _tracks.length) {
      return;
    }
    _focusedTrackId = _tracks[index].id;
  }

  void _restoreFocusedTrack() {
    final id = _focusedTrackId;
    if (id == null || _tracks.isEmpty) {
      return;
    }
    final next = _tracks.indexWhere((t) => t.id == id);
    if (next < 0) {
      // The focused track was filtered out; fall back to the top rather than
      // pointing at an index that no longer means it.
      ref.read(focusedTrackRowIndexProvider.notifier).set(0);
      return;
    }
    if (next != ref.read(focusedTrackRowIndexProvider)) {
      ref.read(focusedTrackRowIndexProvider.notifier).set(next);
    }
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
    final count = ref.read(libraryTrackCountProvider);
    focus.set(
      edge == _FocusEdge.first
          ? 0
          : count == 0
          ? 0
          : count - 1,
    );
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
}

class _Message extends StatelessWidget {
  const new(this.text, {required this.color});

  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Text(
        text,
        style: context.theme.typography.body.sm.copyWith(color: color),
      ),
    );
  }
}

class _ListSurface extends StatelessWidget {
  const new({required this.theme, required this.child});

  final MixarThemeData theme;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.secondary,
        borderRadius: theme.style.borderRadius.md,
      ),
      child: ClipRRect(
        borderRadius: theme.style.borderRadius.md,
        // Foreground: rows paint an opaque `colors.secondary` fill edge to
        // edge, so a background border sits underneath them and only shows in
        // the empty area below the last row.
        child: DecoratedBox(
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(
            borderRadius: theme.style.borderRadius.md,
            border: Border.all(color: theme.colors.border),
          ),
          child: child,
        ),
      ),
    );
  }
}

/// Sort field + direction, as one menu button next to the filter.
class _SortMenu extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final (field, ascending) = ref.watch(librarySortProvider);
    return MixarMenuAnchor(
      // The default 200px is too narrow for "Sort by Artist" at body.sm: the
      // label wraps and the row doubles in height.
      minWidth: 220,
      menuBuilder: (context, handle) => MixarMenuBody(
        groups: [
          MixarMenuGroup(
            children: [
              for (final option in LibrarySortField.values)
                MixarMenuItem(
                  // `title`, not `child`: `child` skips the row padding, which
                  // would leave these rows a different height from the padded
                  // direction item below.
                  title: Row(
                    children: [
                      Expanded(
                        // One line per row: a wrapped label doubles the row
                        // height and breaks the menu's rhythm.
                        child: Text(
                          'Sort by ${option.label}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (option == field)
                        Icon(
                          LucideIcons.check,
                          size: 14,
                          color: context.theme.colors.primary,
                        ),
                    ],
                  ),
                  onPress: () {
                    handle.hide();
                    ref.read(librarySortProvider.notifier).setField(option);
                  },
                ),
            ],
          ),
          MixarMenuGroup(
            children: [
              MixarMenuItem(
                title: Text(ascending ? 'Ascending' : 'Descending'),
                onPress: () {
                  handle.hide();
                  ref.read(librarySortProvider.notifier).toggleDirection();
                },
              ),
            ],
          ),
        ],
      ),
      childBuilder: (context, handle) => ConstrainedBox(
        // Content-sized by default, capped when the pane is narrow. A `Flexible`
        // here instead would make the button a flex child, which lets it grow
        // to a share of the toolbar instead of hugging its label.
        constraints: const BoxConstraints(maxWidth: 120),
        child: AppButton(
          semanticsLabel: 'Sort tracks',
          onPress: handle.toggle,
          variant: .ghost,
          size: .xs,
          // `AppButton` defaults to MainAxisSize.max, which would stretch the
          // button to the cap above instead of hugging its label.
          mainAxisSize: MainAxisSize.min,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: Text(
                  field.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: context.theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(width: 4),
              Icon(
                ascending ? LucideIcons.arrowUp : LucideIcons.arrowDown,
                size: 12,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Session-only density switch. Deliberately does not write settings.
class _DensityToggleButton extends ConsumerWidget {
  const new({required this.density});

  final LibraryRowDensity density;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final target = density.other;
    return AppTooltip(
      tip: target == LibraryRowDensity.compact
          ? 'Compact rows'
          : 'Comfortable rows',
      child: AppButton.icon(
        semanticsLabel: 'Toggle row layout',
        onPress: () => ref
            .read(libraryRowDensityOverrideProvider.notifier)
            .toggle(density),
        variant: .ghost,
        size: .xs,
        child: Icon(
          density.isCompact ? LucideIcons.list : LucideIcons.rows3,
          size: 14,
        ),
      ),
    );
  }
}

/// Which end of the list an absolute focus move (Home / End) targets.
enum _FocusEdge { first, last }

/// Restores the keyboard focus TrinaGrid provided: Up/Down/Home/End move the
/// focused row, Enter loads it to deck A, Shift+Enter to deck B.
class _FocusableTrackList extends StatelessWidget {
  const new({
    required this.tracks,
    required this.density,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.controller,
    required this.onSelect,
    required this.onLoadDeck,
    required this.onMove,
    required this.focusNode,
  });

  final List<LibraryTrackSummary> tracks;
  final LibraryRowDensity density;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final ScrollController controller;
  final ValueChanged<int> onSelect;
  final void Function(int index, int deckId) onLoadDeck;
  final void Function(int? delta, _FocusEdge edge) onMove;
  final FocusNode focusNode;

  @override
  Widget build(BuildContext context) {
    final tab = LibraryListScope.tabOf(context);
    final resolved = LibraryListScope.resolvedOf(context);
    final focused = focusedIndexOf(context);
    // Shortcuts and Actions wrap the focusable, never the other way round:
    // key events are dispatched from the focused node *up* the focus tree, so
    // a Shortcuts widget below the focused node never sees them.
    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.arrowDown): _MoveDownIntent(),
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
              onMove(1, _FocusEdge.first);
              return null;
            },
          ),
          _MoveUpIntent: CallbackAction<_MoveUpIntent>(
            onInvoke: (_) {
              onMove(-1, _FocusEdge.first);
              return null;
            },
          ),
          _MoveFirstIntent: CallbackAction<_MoveFirstIntent>(
            onInvoke: (_) {
              onMove(null, _FocusEdge.first);
              return null;
            },
          ),
          _MoveLastIntent: CallbackAction<_MoveLastIntent>(
            onInvoke: (_) {
              onMove(null, _FocusEdge.last);
              return null;
            },
          ),
          _LoadDeckAIntent: CallbackAction<_LoadDeckAIntent>(
            onInvoke: (_) {
              onLoadDeck(focused, 0);
              return null;
            },
          ),
          _LoadDeckBIntent: CallbackAction<_LoadDeckBIntent>(
            onInvoke: (_) {
              onLoadDeck(focused, 1);
              return null;
            },
          ),
        },
        child: Focus(
          focusNode: focusNode,
          // A list has no built-in focus ring; the focused row fill is the
          // cue. Focus is requested on row tap rather than via autofocus,
          // so typing in the filter box does not steal the arrow keys.
          child: ListView.builder(
            controller: controller,
            itemExtent: density.height,
            padding: EdgeInsets.zero,
            itemCount: tracks.length,
            itemBuilder: (context, index) {
              final track = tracks[index];
              return TrackListRow(
                key: ValueKey(track.id),
                index: index,
                track: track,
                density: density,
                keyDisplayMode: keyDisplayMode,
                keyColorMode: keyColorMode,
                inLibrary: trackIsInLibrary(
                  track,
                  tab: tab,
                  driveResolvedByPath: resolved,
                ),
                onSelect: () {
                  // Clicking a row also grants the list keyboard focus, so
                  // the arrow keys keep working after a tap.
                  focusNode.requestFocus();
                  onSelect(index);
                },
                onLoadDeck: (deckId) => onLoadDeck(index, deckId),
              );
            },
          ),
        ),
      ),
    );
  }
}

/// Current focused index, for the keyboard load actions.
int focusedIndexOf(BuildContext context) =>
    context.dependOnInheritedWidgetOfExactType<_FocusScope>()?.index ?? 0;

/// Publishes the focused index to the keyboard layer, which lives outside the
/// per-row [Consumer]s.
class _FocusScope extends InheritedWidget {
  const new({required this.index, required super.child});

  final int index;

  @override
  bool updateShouldNotify(_FocusScope oldWidget) => oldWidget.index != index;
}

/// Inherits the tab + drive-resolution snapshot for the current list build, so
/// the focusable list does not need a second `ref.watch` per row.
class LibraryListScope extends InheritedWidget {
  const new({
    required this.tab,
    required this.resolved,
    required super.child,
    super.key,
  });

  final LibrarySourceTab tab;
  final Map<String, LibraryTrackSummary> resolved;

  static LibrarySourceTab tabOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryListScope>()!.tab;

  static Map<String, LibraryTrackSummary> resolvedOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<LibraryListScope>()!.resolved;

  @override
  bool updateShouldNotify(LibraryListScope oldWidget) =>
      oldWidget.tab != tab || !identical(oldWidget.resolved, resolved);
}

/// One track row. A [Consumer] so job state, session dim and the focus fill
/// rebuild this row only.
class TrackListRow extends ConsumerWidget {
  const new({
    required this.index,
    required this.track,
    required this.density,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.inLibrary,
    required this.onSelect,
    required this.onLoadDeck,
    super.key,
  });

  final int index;
  final LibraryTrackSummary track;
  final LibraryRowDensity density;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final bool inLibrary;
  final VoidCallback onSelect;
  final ValueChanged<int> onLoadDeck;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final title = trackTitleLabel(track);
    final dimmed = ref.watch(
      sessionTrackDimmedProvider((track.id, track.path)),
    );
    // Pointer-down (not tap): super_dnd's drag recognizer often wins the
    // gesture arena, so a tap handler never fires on a drag.
    //
    // Deliberately not gated on `event.buttons`: any press focuses the row,
    // including a right-click (which then opens the context menu) and the start
    // of a drag. Focusing what the user just pressed is what makes the MIDI
    // "load focused row" command and the focused-row fill agree with the row
    // under the pointer.
    return Listener(
      behavior: HitTestBehavior.translucent,
      onPointerDown: (_) => onSelect(),
      child: _TrackActionsContextMenu(
        trackId: track.id,
        path: track.path,
        title: title,
        inLibrary: inLibrary,
        onLoadDeck: onLoadDeck,
        // Watch engine here so drag attaches after start without rebuilding
        // the surrounding list.
        child: Consumer(
          builder: (context, ref, child) {
            final row = Stack(
              children: [
                child ?? const SizedBox.shrink(),
                _TrackStatusOverlay(trackId: track.id),
              ],
            );
            final content = ref.watch(engineRunningProvider)
                ? _dragRow(context, row, title)
                : row;
            return AnimatedOpacity(
              opacity: dimmed ? kSessionPlayedRowOpacity : 1,
              duration: const Duration(milliseconds: 120),
              child: content,
            );
          },
          child: _RowSurface(
            index: index,
            track: track,
            density: density,
            inLibrary: inLibrary,
            keyDisplayMode: keyDisplayMode,
            keyColorMode: keyColorMode,
            title: title,
          ),
        ),
      ),
    );
  }

  Widget _dragRow(BuildContext context, Widget row, String title) {
    final payload = TrackDragPayload(
      source: inLibrary ? TrackDragSource.library : TrackDragSource.filesystem,
      trackId: inLibrary ? track.id : null,
      path: track.path,
      title: trackDisplayTitle(title: title, path: track.path),
    );
    return DragItemWidget(
      dragItemProvider: (_) async {
        final item = DragItem(
          localData: payload.toLocalData(),
          suggestedName: payload.title,
        );
        item.add(Formats.plainText(encodeTrackDragPlainText(payload)));
        return item;
      },
      allowedOperations: () => [DropOperation.copy],
      dragBuilder: (context, child) => _TrackDragCard(title: payload.title),
      child: DraggableWidget(
        hitTestBehavior: HitTestBehavior.opaque,
        child: row,
      ),
    );
  }
}

/// Row background + focused fill. The fill is the only selection cue a list
/// needs, so it stands in for the grid's current-cell highlight.
class _RowSurface extends ConsumerWidget {
  const new({
    required this.index,
    required this.track,
    required this.density,
    required this.inLibrary,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.title,
  });

  /// Index in list order, captured at build. Stable per row, because
  /// `ListView.builder` children are keyed by track id and a re-sort replaces
  /// the element rather than reusing it with a new index.
  final int index;
  final LibraryTrackSummary track;
  final LibraryRowDensity density;
  final bool inLibrary;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final String title;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final focused = ref.watch(
      focusedTrackRowIndexProvider.select((i) => i == index),
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        color: focused
            ? libraryListSelectedRowColor(theme)
            : theme.colors.secondary,
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6),
        child: density.isCompact
            ? _CompactRow(
                track: track,
                title: title,
                inLibrary: inLibrary,
                keyDisplayMode: keyDisplayMode,
                keyColorMode: keyColorMode,
                artSize: density.artSize,
              )
            : _ComfortableRow(
                track: track,
                title: title,
                inLibrary: inLibrary,
                keyDisplayMode: keyDisplayMode,
                keyColorMode: keyColorMode,
                artSize: density.artSize,
              ),
      ),
    );
  }
}

/// Right-aligned BPM + key, shared by both densities so the two layouts line
/// up identically on the right. Fixed widths keep the values in columns down
/// the list; the title and pills absorb whatever width is left.
class _TrailingMeta extends StatelessWidget {
  const new({
    required this.bpm,
    required this.rawKey,
    required this.keyDisplayMode,
    required this.keyColorMode,
    this.durationMs,
  });

  final double? bpm;
  final String rawKey;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;

  /// Rendered as a leading pill when set. Compact has no metadata pills row, so
  /// the trailing group is the only place its length can appear; comfortable
  /// already carries the duration on the left and passes null to avoid showing
  /// it twice.
  final int? durationMs;

  @override
  Widget build(BuildContext context) {
    // End-aligned so the group hugs the right edge and stays put when only some
    // of the pills are present.
    // Duration first, then BPM, then key: BPM and key stay flush right in both
    // densities, so they hold their position when the density is toggled.
    final duration = formatTrackDuration(durationMs);
    final children = <Widget>[
      if (duration.isNotEmpty)
        Flexible(
          flex: kDurationSlotFlex,
          child: _MetaPill(
            text: duration,
            leading: const _MetaPillGlyph(LucideIcons.clock),
          ),
        ),
      if (bpm != null)
        Flexible(
          flex: kBpmSlotFlex,
          child: _MetaPill(
            text: '${bpm!.toStringAsFixed(1)} BPM',
            leading: const _MetaPillGlyph(LucideIcons.metronome),
          ),
        ),
      if (rawKey.isNotEmpty)
        Flexible(
          flex: kKeySlotFlex,
          child: _KeyPill(
            rawKey: rawKey,
            keyDisplayMode: keyDisplayMode,
            keyColorMode: keyColorMode,
          ),
        ),
    ];
    return Row(
      key: kTrailingMetaKey,
      mainAxisAlignment: MainAxisAlignment.end,
      children: [
        for (var i = 0; i < children.length; i++) ...[
          if (i > 0) const SizedBox(width: kMetaPillGap),
          children[i],
        ],
      ],
    );
  }
}

/// Muted pill glyph, sized for the metadata pills.
class _MetaPillGlyph extends StatelessWidget {
  const new(this.icon);

  final IconData icon;

  @override
  Widget build(BuildContext context) =>
      Icon(icon, size: 12, color: context.theme.colors.mutedForeground);
}

/// Key pill: the same chip as the rest of the metadata, with the key label
/// taking the configured key colour. Watches the harmonic reference so a deck
/// key change repaints the row without rebuilding the list.
class _KeyPill extends ConsumerWidget {
  const new({
    required this.rawKey,
    required this.keyDisplayMode,
    required this.keyColorMode,
  });

  final String rawKey;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final reference = ref.watch(harmonicReferenceKeyProvider);
    final color = colorForKey(
      rawKey,
      keyColorMode,
      harmonicReferenceKey: reference,
    );
    return _MetaPill(
      text: formatDeckKey(rawKey, keyDisplayMode),
      leading: const _MetaPillGlyph(LucideIcons.music2),
      textColor: color ?? theme.colors.mutedForeground,
      fontWeight: color != null ? FontWeight.w600 : FontWeight.w500,
    );
  }
}

/// A metadata chip under the comfortable row's title.
class _MetaPill extends StatelessWidget {
  const new({
    required this.text,
    this.leading,
    this.textColor,
    this.fontWeight = FontWeight.w500,
  });

  final String text;

  /// Optional leading widget, used by the metadata pills for their glyphs.
  final Widget? leading;

  /// Overrides the muted default, used by the key pill for its key colour.
  final Color? textColor;
  final FontWeight fontWeight;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) {
        // Drop the glyph in a squeezed pill: below this the icon alone consumes
        // the whole box and pushes the label into an overflow. The text carries
        // the meaning, so the icon is the part that goes.
        final innerWidth = constraints.maxWidth - kMetaPillPadding.horizontal;
        final showLeading =
            leading != null && innerWidth >= kMetaPillIconMinWidth;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: theme.colors.muted,
            borderRadius: theme.style.borderRadius.pill,
          ),
          child: Padding(
            padding: kMetaPillPadding,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                if (showLeading) ...[leading!, const SizedBox(width: 4)],
                // Flexible, not a bare Text: inside a min-size Row a loose child
                // keeps its intrinsic width and overflows the pill, so a long
                // artist name would spill across the row instead of ellipsizing.
                Flexible(
                  child: Text(
                    text,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.xs.copyWith(
                      color: textColor ?? theme.colors.mutedForeground,
                      fontWeight: fontWeight,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

/// The 3-dot actions menu, right-aligned in the reserved trailing slot.
class _RowActionsSlot extends StatelessWidget {
  const new({
    required this.trackId,
    required this.path,
    required this.title,
    required this.inLibrary,
  });

  final String trackId;
  final String path;
  final String title;
  final bool inLibrary;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: kActionsColumnWidth,
      child: Center(
        child: Consumer(
          builder: (context, ref, _) {
            // Watched here so the menu re-enables the moment a job ends. The
            // list does not regenerate rows for job state.
            final analyzing = ref.watch(
              analyzingTrackIdsProvider.select((ids) => ids.contains(trackId)),
            );
            final stemsGenerating = ref.watch(
              stemGeneratingTrackIdsProvider.select(
                (ids) => ids.contains(trackId),
              ),
            );
            return TrackActionsMenu(
              trackId: trackId,
              path: path,
              title: title,
              inLibrary: inLibrary,
              analyzing: analyzing,
              stemsGenerating: stemsGenerating,
              enableSecondaryPress: false,
            );
          },
        ),
      ),
    );
  }
}

class _CompactRow extends StatelessWidget {
  const new({
    required this.track,
    required this.title,
    required this.inLibrary,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.artSize,
  });

  final LibraryTrackSummary track;
  final String title;
  final bool inLibrary;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final double artSize;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final textStyle = theme.typography.body.sm.copyWith(
      color: theme.colors.foreground,
    );
    return LayoutBuilder(
      builder: (context, constraints) => Row(
        children: [
          _ArtworkThumb(trackId: track.id, size: artSize),
          const SizedBox(width: kArtworkGap),
          Expanded(
            child: Text(
              title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: textStyle,
            ),
          ),
          const SizedBox(width: kRowGutter),
          SizedBox(
            width: trailingMetaWidth(
              constraints.maxWidth,
              artSize,
              // Compact has no metadata pills, so the trailing group carries
              // the length too and must reserve room for it.
              natural:
                  kDurationSlotWidth + kMetaPillGap + kTrailingMetaBaseWidth,
            ),
            child: _TrailingMeta(
              bpm: track.bpm,
              rawKey: track.key ?? '',
              keyDisplayMode: keyDisplayMode,
              keyColorMode: keyColorMode,
              durationMs: track.durationMs,
            ),
          ),
          _RowActionsSlot(
            trackId: track.id,
            path: track.path,
            title: title,
            inLibrary: inLibrary,
          ),
        ],
      ),
    );
  }
}

class _ComfortableRow extends StatelessWidget {
  const new({
    required this.track,
    required this.title,
    required this.inLibrary,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.artSize,
  });

  final LibraryTrackSummary track;
  final String title;
  final bool inLibrary;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final double artSize;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final titleStyle = theme.typography.body.sm.copyWith(
      color: theme.colors.foreground,
      fontWeight: FontWeight.w600,
    );
    // The metadata the old column table showed, less BPM and key, which are
    // aligned in the trailing meta instead. Each renders as its own pill.
    final duration = formatTrackDuration(track.durationMs);
    Widget? artistPill() =>
        (track.artist ?? '').isNotEmpty ? _MetaPill(text: track.artist!) : null;
    Widget? albumPill() =>
        (track.album ?? '').isNotEmpty ? _MetaPill(text: track.album!) : null;
    Widget? genrePill() =>
        (track.genre ?? '').isNotEmpty ? _MetaPill(text: track.genre!) : null;
    Widget? durationPill() => duration.isNotEmpty
        ? _MetaPill(
            text: duration,
            leading: Icon(
              LucideIcons.clock,
              size: 11,
              color: theme.colors.mutedForeground,
            ),
          )
        : null;
    final pills = <Widget?>[
      artistPill(),
      albumPill(),
      genrePill(),
      durationPill(),
    ].whereType<Widget>().toList();

    return LayoutBuilder(
      builder: (context, constraints) {
        // Bound each pill to half the details column, so at most two share a
        // run and four pills can never need a third one. Without this a long
        // artist name consumes a whole run and the row grows past the fixed
        // `itemExtent`, where the extra runs are silently clipped.
        final metaWidth = trailingMetaWidth(constraints.maxWidth, artSize);
        final detailsWidth =
            constraints.maxWidth -
            artSize -
            kArtworkGap -
            kRowGutter -
            metaWidth -
            kRowGutter -
            kActionsColumnWidth;
        final maxPillWidth = (detailsWidth - kMetaPillGap) / 2;
        return Row(
          children: [
            _ArtworkThumb(trackId: track.id, size: artSize),
            const SizedBox(width: kArtworkGap),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: titleStyle,
                  ),
                  // Wrap, not a single ellipsized line: a long artist name
                  // pushes the remaining pills onto a second line instead of
                  // hiding them. Two runs is the cap (see `maxPillWidth`); the
                  // clip is belt-and-braces so no label can paint outside the
                  // row. Skipped entirely when there is no room left: a
                  // zero-width cap would render the pills as an empty gap.
                  if (pills.isNotEmpty && maxPillWidth > 0)
                    Padding(
                      padding: const EdgeInsets.only(top: 3),
                      // maxHeight, not a fixed height. Reserving two runs when
                      // a row only needs one leaves dead space under the pills,
                      // which pushes the title + pills group off centre. The
                      // cap still bounds the area at the two runs the width cap
                      // above guarantees.
                      child: ConstrainedBox(
                        constraints: const BoxConstraints(
                          maxHeight: kMetaPillAreaHeight,
                        ),
                        child: Wrap(
                          spacing: kMetaPillGap,
                          runSpacing: kMetaPillGap,
                          clipBehavior: Clip.hardEdge,
                          children: [
                            for (final pill in pills)
                              ConstrainedBox(
                                constraints: BoxConstraints(
                                  maxWidth: maxPillWidth,
                                ),
                                child: pill,
                              ),
                          ],
                        ),
                      ),
                    ),
                ],
              ),
            ),
            const SizedBox(width: kRowGutter),
            SizedBox(
              width: metaWidth,
              child: _TrailingMeta(
                bpm: track.bpm,
                rawKey: track.key ?? '',
                keyDisplayMode: keyDisplayMode,
                keyColorMode: keyColorMode,
              ),
            ),
            const SizedBox(width: kRowGutter),
            _RowActionsSlot(
              trackId: track.id,
              path: track.path,
              title: title,
              inLibrary: inLibrary,
            ),
          ],
        );
      },
    );
  }
}

class _ArtworkThumb extends ConsumerWidget {
  const new({required this.trackId, required this.size});

  final String trackId;
  final double size;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    Widget placeholder() => ColoredBox(
      color: theme.colors.muted,
      child: Center(
        child: Icon(
          LucideIcons.disc2,
          size: size * 0.57,
          color: theme.colors.mutedForeground,
        ),
      ),
    );
    final bytes = ref.watch(artworkCacheProvider.select((c) => c[trackId]));
    // One square box for both states. The placeholder needs the explicit
    // `SizedBox.square` too: left unconstrained it shrank to the icon's width
    // while the row stretched it to the full row height, so the thumb rendered
    // as a tall sliver instead of a square.
    return SizedBox.square(
      dimension: size,
      child: bytes != null && bytes.isNotEmpty
          ? Image.memory(
              bytes,
              width: size,
              height: size,
              fit: BoxFit.cover,
              cacheWidth: (size * 2).round(),
              cacheHeight: (size * 2).round(),
              errorBuilder: (_, _, _) => placeholder(),
            )
          : placeholder(),
    );
  }
}

/// `m:ss` for [ms], or empty when the duration is unknown.
///
/// Empty, not `0:00`, for null and non-positive values: those come from missing
/// metadata, and the caller treats empty as "no duration to show". A *known*
/// sub-second length is not unknown, so it renders as `0:00` rather than
/// vanishing. Seconds are floored, not rounded, matching how file managers and
/// the pre-refactor table displayed the same values.
String formatTrackDuration(int? ms) {
  if (ms == null || ms <= 0) {
    return '';
  }
  final totalSec = ms ~/ 1000;
  final m = totalSec ~/ 60;
  final s = totalSec % 60;
  return '$m:${s.toString().padLeft(2, '0')}';
}

/// In-flight analysis / stem job chrome drawn over a track row.
///
/// Lives in the row rather than the title line so a progress tick rebuilds one
/// row instead of the whole list.
class _TrackStatusOverlay extends ConsumerWidget {
  const new({required this.trackId});

  final String trackId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final jobs =
        ref.watch(trackProgressProvider.select((byId) => byId[trackId])) ??
        const <TrackProgressInfo>[];
    final analyzing = ref.watch(
      analyzingTrackIdsProvider.select((ids) => ids.contains(trackId)),
    );
    final stemsGenerating = ref.watch(
      stemGeneratingTrackIdsProvider.select((ids) => ids.contains(trackId)),
    );
    // Queued work has no phase yet. Reuse the label vocabulary with a null
    // fraction so it reads indeterminate instead of inventing a percentage.
    // At most one job per lane, so at most two pills. Partitioned rather than
    // sorted: the two-lane comparator ties within a lane, and `List.sort` is
    // only stable below its insertion-sort threshold, so a sort could swap
    // same-lane pills between frames on a progress tick.
    final analysisLane = <TrackProgressInfo>[
      for (final job in jobs)
        if (!isStemProgressPhase(job.phase)) job,
      if (analyzing && !jobs.any((i) => !isStemProgressPhase(i.phase)))
        const TrackProgressInfo(phase: 'analyze'),
    ];
    final stemLane = <TrackProgressInfo>[
      for (final job in jobs)
        if (isStemProgressPhase(job.phase)) job,
      if (stemsGenerating && !jobs.any((i) => isStemProgressPhase(i.phase)))
        const TrackProgressInfo(phase: 'stems_queued'),
    ];
    final infos = [...analysisLane, ...stemLane];
    if (infos.isEmpty) {
      return const SizedBox.shrink();
    }
    final fractions = [for (final info in infos) ?info.fraction];
    final theme = context.theme;
    // Decorative only. Without this the bar's ColoredBox and the pill's own
    // widgets are the topmost hit target, so Stack hit testing stops there
    // and the row's pointer-down selection and drag-to-deck never fire.
    return IgnorePointer(
      child: Stack(
        children: [
          // Span the row so the pill group stays flush right as it grows —
          // a right-only Positioned hands its child loose constraints, which
          // then centres inside the inset box and drifts left.
          Positioned.fill(
            child: Padding(
              padding: const EdgeInsets.only(
                right: kActionsColumnWidth + kStatusPillGap,
              ),
              child: Align(
                alignment: Alignment.centerRight,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    for (var i = 0; i < infos.length; i++) ...[
                      if (i > 0) const SizedBox(width: kStatusPillGap),
                      // Flexible so long labels on two lanes ellipsize in a
                      // narrow pane instead of overflowing the row.
                      Flexible(child: _TrackStatusPill(info: infos[i])),
                    ],
                  ],
                ),
              ),
            ),
          ),
          // One bar per job that reported a fraction, stacked in pill order.
          // The ordinal counts rendered bars, not entries in `infos`: a lane
          // with no fraction draws nothing, and counting it would leave a
          // spurious stride gap under the first real bar.
          for (var i = 0; i < fractions.length; i++)
            Positioned(
              left: 0,
              right: 0,
              bottom: i * (kStatusBarHeight + kStatusBarGap),
              child: SizedBox(
                height: kStatusBarHeight,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: theme.colors.primary.withValues(alpha: 0.18),
                  ),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: FractionallySizedBox(
                      widthFactor: fractions[i].clamp(0.0, 1.0),
                      child: SizedBox.expand(
                        child: ColoredBox(color: theme.colors.primary),
                      ),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _TrackStatusPill extends StatelessWidget {
  const new({required this.info});

  final TrackProgressInfo info;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final fg = theme.colors.mutedForeground;
    return Semantics(
      label: info.label,
      container: true,
      // Announce the phase once instead of also reading the spinner + text.
      excludeSemantics: true,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: theme.colors.background,
          borderRadius: theme.style.borderRadius.pill,
          border: Border.all(color: theme.colors.border),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (info.fraction == null) ...[
                MLoader(size: MLoaderSize.xs, color: fg),
                const SizedBox(width: 5),
              ],
              Flexible(
                // Semantics above still carry the full label; the ellipsis is
                // only for when two lanes crowd a narrow pane.
                child: Text(
                  info.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.typography.body.xs.copyWith(
                    color: fg,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class TrackActionsMenu extends ConsumerWidget {
  const new({
    required this.trackId,
    required this.path,
    required this.title,
    required this.inLibrary,
    required this.analyzing,
    required this.stemsGenerating,
    this.enableSecondaryPress = true,
    super.key,
  });

  final String trackId;
  final String path;
  final String title;
  final bool inLibrary;
  final bool analyzing;
  final bool stemsGenerating;
  final bool enableSecondaryPress;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final engineRunning = ref.watch(engineRunningProvider);
    // Always reachable: a running job disables its own menu item, and the
    // row status pill is what shows progress.
    return MixarMenuAnchor(
      menuBuilder: (context, controller) => _trackActionsMenuBody(
        context: context,
        ref: ref,
        dismiss: controller.hide,
        trackId: trackId,
        path: path,
        title: title,
        inLibrary: inLibrary,
        analyzing: analyzing,
        stemsGenerating: stemsGenerating,
        engineRunning: engineRunning,
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

class _TrackActionsContextMenu extends ConsumerWidget {
  const new({
    required this.trackId,
    required this.path,
    required this.title,
    required this.inLibrary,
    required this.onLoadDeck,
    required this.child,
  });

  final String trackId;
  final String path;
  final String title;
  final bool inLibrary;
  final ValueChanged<int> onLoadDeck;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final engineRunning = ref.watch(engineRunningProvider);
    // Watched here rather than read in the row: the list does not regenerate
    // rows when a job starts or ends, so a read would go stale.
    final analyzing = ref.watch(
      analyzingTrackIdsProvider.select((ids) => ids.contains(trackId)),
    );
    final stemsGenerating = ref.watch(
      stemGeneratingTrackIdsProvider.select((ids) => ids.contains(trackId)),
    );
    return MixarContextMenu(
      menuBuilder: (context, handle) => _trackActionsMenuBody(
        context: context,
        ref: ref,
        dismiss: handle.hide,
        trackId: trackId,
        path: path,
        title: title,
        inLibrary: inLibrary,
        analyzing: analyzing,
        stemsGenerating: stemsGenerating,
        engineRunning: engineRunning,
        onLoadDeck: (deckId) {
          handle.hide();
          onLoadDeck(deckId);
        },
      ),
      childBuilder: (context, handle) => GestureDetector(
        onSecondaryTapDown: (details) => handle.showAt(details.globalPosition),
        onLongPressStart: (details) => handle.showAt(details.globalPosition),
        child: child,
      ),
    );
  }
}

Widget _trackActionsMenuBody({
  required BuildContext context,
  required WidgetRef ref,
  required VoidCallback dismiss,
  required String trackId,
  required String path,
  required String title,
  required bool inLibrary,
  required bool analyzing,
  required bool stemsGenerating,
  required bool engineRunning,
  void Function(int deckId)? onLoadDeck,
}) {
  void load(int deckId) {
    // Route through the caller's index when the row is in a list, otherwise
    // fall back to loading this track by id.
    if (onLoadDeck != null) {
      onLoadDeck(deckId);
      return;
    }
    unawaited(
      loadPayloadToDeck(
        ref,
        deckId,
        TrackDragPayload(
          source: inLibrary
              ? TrackDragSource.library
              : TrackDragSource.filesystem,
          trackId: inLibrary ? trackId : null,
          path: path,
          title: trackDisplayTitle(title: title, path: path),
        ),
      ),
    );
  }

  return MixarMenuBody(
    groups: [
      MixarMenuGroup(
        children: [
          MixarMenuItem(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(8, 8, 8, 2),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                spacing: 4,
                children: [
                  const Text('Load to deck'),
                  Row(
                    spacing: 4,
                    children: [
                      _LoadDeckChip(
                        letter: 'A',
                        color: FaderColors.a.grip,
                        enabled: engineRunning,
                        onPress: () {
                          dismiss();
                          load(0);
                        },
                      ),
                      _LoadDeckChip(
                        letter: 'B',
                        color: FaderColors.b.grip,
                        enabled: engineRunning,
                        onPress: () {
                          dismiss();
                          load(1);
                        },
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
      MixarMenuGroup(
        children: [
          MixarMenuItem(
            title: Text(analyzing ? 'Analyzing…' : 'Analyze'),
            enabled: inLibrary && !analyzing,
            onPress: !inLibrary || analyzing
                ? null
                : () {
                    dismiss();
                    unawaited(analyzeTrackAction(ref, trackId));
                  },
          ),
          MixarMenuItem(
            title: Text(
              stemsGenerating ? 'Generating stems…' : 'Generate stems',
            ),
            // Stems are a separate, explicit action: analysis never triggers
            // them. Safe to fire against a track that already has a cache —
            // the worker no-ops when the cache is valid.
            enabled: inLibrary && !stemsGenerating,
            onPress: !inLibrary || stemsGenerating
                ? null
                : () {
                    dismiss();
                    unawaited(generateStemsAction(ref, trackId));
                  },
          ),
          MixarMenuItem(
            title: const Text('Refresh'),
            enabled: inLibrary,
            onPress: inLibrary
                ? () {
                    dismiss();
                    unawaited(refreshTrackAction(ref, trackId));
                  }
                : null,
          ),
          MixarMenuItem(
            title: const Text('Track details…'),
            enabled: inLibrary,
            onPress: inLibrary
                ? () {
                    dismiss();
                    unawaited(
                      showTrackDetailDialog(context, ref, trackId: trackId),
                    );
                  }
                : null,
          ),
        ],
      ),
    ],
  );
}

class _LoadDeckChip extends StatelessWidget {
  const new({
    required this.letter,
    required this.color,
    required this.enabled,
    required this.onPress,
  });

  final String letter;
  final Color color;
  final bool enabled;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Expanded(
      child: AppButton(
        semanticsLabel: 'Load to $letter',
        onPress: enabled ? onPress : null,
        variant: .ghost,
        size: .xs,
        child: Text(
          letter,
          style: theme.typography.body.xs.copyWith(
            fontWeight: FontWeight.w700,
            color: enabled ? color : color.withValues(alpha: 0.4),
          ),
        ),
      ),
    );
  }
}

class _TrackDragCard extends StatelessWidget {
  const new({required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colors.background.withValues(alpha: 0.95),
        borderRadius: theme.style.borderRadius.md,
        border: Border.all(color: theme.colors.border),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 200),
          child: Text(
            title,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.typography.body.sm.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
    );
  }
}

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
