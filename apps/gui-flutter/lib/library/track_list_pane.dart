import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/library/collection_actions_menu.dart';
import 'package:gui_flutter/library/focused_load.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/library_list_chrome.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_detail_dialog.dart';
import 'package:gui_flutter/library/track_list.dart';
import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

// Re-exported so existing callers/tests that reach for the row chrome through
// this library keep working after it moved to `library_list_chrome.dart`.
export 'package:gui_flutter/library/library_list_chrome.dart'
    show formatTrackDuration, kTrailingMetaKey, libraryListSelectedRowColor;
export 'package:gui_flutter/library/track_list.dart' show TrackListRow;

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
// the pills in comfortable) take whatever is left. The trailing slot widths and
// the shared budget live in `library_list_chrome.dart`.
const kArtworkGap = 10.0;

/// Filter + sortable track list.
class TrackListPane extends ConsumerStatefulWidget {
  const new({super.key});

  @override
  ConsumerState<TrackListPane> createState() => _TrackListPaneState();
}

class _TrackListPaneState extends ConsumerState<TrackListPane> {
  @override
  Widget build(BuildContext context) {
    ref.watch(libraryEventsBootstrapProvider);
    final theme = context.theme;
    final selectedId = ref.watch(activeCollectionIdProvider);
    final drive = ref.watch(librarySourceTabProvider) == LibrarySourceTab.drive;
    final drivePath = ref.watch(driveCurrentPathProvider);
    final tracksAsync = ref.watch(libraryTableTracksProvider);
    final density = ref.watch(libraryRowDensityProvider);
    // Watched (not read): `trackRowInLibrary` reads this to derive each row's
    // payload source and menu enablement, and a drive resolution completing
    // must rebuild the rows — the dependency the old hoisted watch provided.
    ref.watch(driveResolvedByPathProvider);
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s, orElse: defaultAppSettings);
    final keyDisplayMode = keyModeFromSettings(settings.keyDisplayMode);
    final keyColorMode = keyColorModeFromSettings(settings.keyColorMode);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        LibraryPaneToolbar(
          hint: 'Filter tracks…',
          onChanged: (value) =>
              ref.read(trackFilterProvider.notifier).set(value),
          trailing: [
            _SortMenu(),
            const SizedBox(width: 4),
            LibraryRowDensityButton(density: density),
            // Collection actions in the topbar, mirroring the history session
            // actions menu. Hidden outside the collections tab.
            if (!drive) ...[
              const SizedBox(width: 4),
              const _ActiveCollectionActions(),
            ],
          ],
        ),
        Expanded(
          child: !drive && selectedId == null
              ? LibraryListMessage(
                  'Select a collection',
                  color: theme.colors.mutedForeground,
                )
              : drive && drivePath == null
              ? LibraryListMessage(
                  'Select a drive or folder to browse audio files',
                  color: theme.colors.mutedForeground,
                )
              : TrackListView<LibraryTrackSummary>(
                  items: tracksAsync,
                  idOf: (track) => track.id,
                  payloadOf: _payloadOf,
                  rowBuilder: (context, ref, index, track, density, slot) =>
                      density.isCompact
                      ? _CompactRow(
                          track: track,
                          title: trackTitleLabel(track),
                          keyDisplayMode: keyDisplayMode,
                          keyColorMode: keyColorMode,
                          artSize: density.artSize,
                          actionsSlot: slot,
                        )
                      : _ComfortableRow(
                          track: track,
                          title: trackTitleLabel(track),
                          keyDisplayMode: keyDisplayMode,
                          keyColorMode: keyColorMode,
                          artSize: density.artSize,
                          actionsSlot: slot,
                        ),
                  emptyBuilder: (context) => LibraryListMessage(
                    drive ? 'No audio files in this folder' : 'No tracks',
                    color: theme.colors.mutedForeground,
                  ),
                  errorBuilder: (context, e) => LibraryListMessage(
                    'Tracks error: $e',
                    color: theme.colors.destructive,
                  ),
                  extraMenuItems: trackRowExtraMenuItems,
                  overlayBuilder: (track) =>
                      _TrackStatusOverlay(trackId: track.id),
                  dimmedOf: (ref, track) => ref.watch(
                    sessionTrackDimmedProvider((track.id, track.path)),
                  ),
                  onVisibleRange: _prefetchArtwork,
                ),
        ),
      ],
    );
  }

  /// Drag payload for a track, resolving whether it is a library row or a
  /// filesystem file (drive browse).
  TrackDragPayload _payloadOf(WidgetRef ref, LibraryTrackSummary track) {
    return payloadFromListTrack(
      track,
      inLibrary: trackRowInLibrary(ref, track),
    );
  }

  /// Artwork prefetch for the visible window; row height is constant per
  /// density, so the window is plain offset arithmetic.
  void _prefetchArtwork(int first, int last) {
    final tracks = ref.read(libraryTableTracksProvider).asData?.value;
    if (tracks == null) {
      return;
    }
    final visible = [
      for (var i = first; i < last && i < tracks.length; i++)
        if (trackRowInLibrary(ref, tracks[i])) tracks[i].id,
    ];
    if (visible.isEmpty) {
      return;
    }
    ref.read(artworkCacheProvider.notifier).ensureLoaded(visible);
  }
}

/// Topbar collection actions for the active collection (hidden until the
/// collections list resolves, mirroring the history toolbar behaviour of
/// showing session actions only with a session selected).
class _ActiveCollectionActions extends ConsumerWidget {
  const new();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final id = ref.watch(activeCollectionIdProvider);
    final collections = ref.watch(collectionsProvider).asData?.value;
    LibraryCollectionSummary? active;
    if (id != null && collections != null) {
      for (final collection in collections) {
        if (collection.id == id) {
          active = collection;
          break;
        }
      }
    }
    if (active == null) {
      return const SizedBox.shrink();
    }
    return CollectionActionsMenu(collection: active);
  }
}

/// Whether a library-list row is a real library track (vs a filesystem file in
/// the drive tab), which gates its payload source and menu actions.
bool trackRowInLibrary(WidgetRef ref, LibraryTrackSummary track) {
  final tab = ref.read(librarySourceTabProvider);
  final resolved =
      ref.read(driveResolvedByPathProvider).asData?.value ?? const {};
  return trackIsInLibrary(track, tab: tab, driveResolvedByPath: resolved);
}

/// Track-row menu items (Analyze, Generate stems, Refresh, Details), shared by
/// the library list and tests so both build the same menu.
List<Widget> trackRowExtraMenuItems(
  BuildContext context,
  WidgetRef ref,
  LibraryTrackSummary track,
  VoidCallback dismiss,
) {
  final inLibrary = trackRowInLibrary(ref, track);
  final analyzing = ref.watch(
    analyzingTrackIdsProvider.select((ids) => ids.contains(track.id)),
  );
  final stemsGenerating = ref.watch(
    stemGeneratingTrackIdsProvider.select((ids) => ids.contains(track.id)),
  );
  return [
    MixarMenuItem(
      title: Text(analyzing ? 'Analyzing…' : 'Analyze'),
      enabled: inLibrary && !analyzing,
      onPress: !inLibrary || analyzing
          ? null
          : () {
              dismiss();
              unawaited(analyzeTrackAction(ref, track.id));
            },
    ),
    MixarMenuItem(
      title: Text(stemsGenerating ? 'Generating stems…' : 'Generate stems'),
      // Stems are a separate, explicit action: analysis never triggers them.
      // Safe to fire against a track that already has a cache — the worker
      // no-ops when the cache is valid.
      enabled: inLibrary && !stemsGenerating,
      onPress: !inLibrary || stemsGenerating
          ? null
          : () {
              dismiss();
              unawaited(generateStemsAction(ref, track.id));
            },
    ),
    MixarMenuItem(
      title: const Text('Refresh'),
      enabled: inLibrary,
      onPress: inLibrary
          ? () {
              dismiss();
              unawaited(refreshTrackAction(ref, track.id));
            }
          : null,
    ),
    MixarMenuItem(
      title: const Text('Track details…'),
      enabled: inLibrary,
      onPress: inLibrary
          ? () {
              dismiss();
              unawaited(showTrackDetailDialog(context, ref, trackId: track.id));
            }
          : null,
    ),
  ];
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

class _CompactRow extends StatelessWidget {
  const new({
    required this.track,
    required this.title,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.artSize,
    required this.actionsSlot,
  });

  final LibraryTrackSummary track;
  final String title;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final double artSize;
  final Widget actionsSlot;

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
              artSize + kArtworkGap,
              // Compact has no metadata pills, so the trailing group carries
              // the length too and must reserve room for it.
              natural:
                  kDurationSlotWidth + kMetaPillGap + kTrailingMetaBaseWidth,
            ),
            child: TrailingMeta(
              bpm: track.bpm,
              rawKey: track.key ?? '',
              keyDisplayMode: keyDisplayMode,
              keyColorMode: keyColorMode,
              durationMs: track.durationMs,
            ),
          ),
          actionsSlot,
        ],
      ),
    );
  }
}

class _ComfortableRow extends StatelessWidget {
  const new({
    required this.track,
    required this.title,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.artSize,
    required this.actionsSlot,
  });

  final LibraryTrackSummary track;
  final String title;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final double artSize;
  final Widget actionsSlot;

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
        (track.artist ?? '').isNotEmpty ? MetaPill(text: track.artist!) : null;
    Widget? albumPill() =>
        (track.album ?? '').isNotEmpty ? MetaPill(text: track.album!) : null;
    Widget? genrePill() =>
        (track.genre ?? '').isNotEmpty ? MetaPill(text: track.genre!) : null;
    Widget? durationPill() => duration.isNotEmpty
        ? MetaPill(
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
        final metaWidth = trailingMetaWidth(
          constraints.maxWidth,
          artSize + kArtworkGap,
        );
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
                      //
                      // Hung left by the pill's own inset so the metadata text
                      // lines up with the title above it.
                      child: Transform.translate(
                        offset: const Offset(-kMetaPillInset, 0),
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
                    ),
                ],
              ),
            ),
            const SizedBox(width: kRowGutter),
            SizedBox(
              width: metaWidth,
              child: TrailingMeta(
                bpm: track.bpm,
                rawKey: track.key ?? '',
                keyDisplayMode: keyDisplayMode,
                keyColorMode: keyColorMode,
              ),
            ),
            const SizedBox(width: kRowGutter),
            actionsSlot,
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
