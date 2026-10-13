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

/// Progress bar thickness, and the gap that keeps stacked per-job bars apart.
/// The vertical stride is their sum, so changing one stays consistent.
const kStatusBarHeight = 2.0;
const kStatusBarGap = 1.0;

/// Partition analysis / stem lanes and synthesise queued jobs.
///
/// Shared by in-flow [_TrackStatusPills] and the bars-only overlay so lane
/// order cannot drift between the two surfaces.
List<TrackProgressInfo> trackStatusInfos({
  required List<TrackProgressInfo> jobs,
  required bool analyzing,
  required bool stemsGenerating,
}) {
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
  return [...analysisLane, ...stemLane];
}

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
  /// density, so the window is plain offset arithmetic. Visible-range emits
  /// are already coalesced to once per frame in [TrackListView].
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
    unawaited(ref.read(artworkCacheProvider.notifier).ensureLoaded(visible));
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
///
/// One click selects a field (ascending); another click on the active field
/// flips direction, like a table column header.
class _SortMenu extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final (field, ascending) = ref.watch(librarySortProvider);
    return MixarMenuAnchor(
      dialogTitle: 'Sort tracks',
      // Field labels are short, but keep headroom so an arrow + accent row
      // never wraps under body.sm.
      minWidth: 160,
      menuBuilder: (context, handle) => MixarMenuBody(
        groups: [
          MixarMenuGroup(
            children: [
              for (final option in LibrarySortField.values)
                MixarMenuItem(
                  // `title`, not `child`: `child` skips the row padding.
                  title: Row(
                    children: [
                      Expanded(
                        // One line per row: a wrapped label doubles the row
                        // height and breaks the menu's rhythm.
                        child: Text(
                          option.label,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            color: option == field
                                ? theme.colors.primary
                                : theme.colors.foreground,
                            fontWeight: option == field
                                ? FontWeight.w600
                                : FontWeight.w400,
                          ),
                        ),
                      ),
                      if (option == field)
                        Icon(
                          // Points the way the list reads: A→Z / low→high is ↓.
                          ascending
                              ? LucideIcons.arrowDown
                              : LucideIcons.arrowUp,
                          size: 14,
                          color: theme.colors.primary,
                        ),
                    ],
                  ),
                  onPress: () {
                    handle.hide();
                    ref.read(librarySortProvider.notifier).select(option);
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
        // `md` grows minHeight to the header (same as the 3-dots menu) so the
        // hover wash fills it. Label stays compact via explicit `body.xs` /
        // 12px arrow below, and tighter horizontal padding than `md` default —
        // `padding` only overrides contentPadding, not contentConstraints.
        child: AppButton(
          semanticsLabel: 'Sort tracks',
          onPress: handle.toggle,
          variant: .ghost,
          size: .md,
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 11),
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
                  style: theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(width: 4),
              Icon(
                ascending ? LucideIcons.arrowDown : LucideIcons.arrowUp,
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
      builder: (context, constraints) {
        final metaWidth = trailingMetaWidth(
          constraints.maxWidth,
          artSize + kArtworkGap,
          // Compact has no metadata pills, so the trailing group carries
          // the length too and must reserve room for it.
          natural: kDurationSlotWidth + kMetaPillGap + kTrailingMetaBaseWidth,
        );
        // Status sits immediately left of TrailingMeta (not inside the title
        // Expanded), capped so long labels ellipsize without eating the title.
        final statusMax =
            (constraints.maxWidth -
                    artSize -
                    kArtworkGap -
                    metaWidth -
                    kActionsColumnWidth -
                    kMinTitleWidth)
                .clamp(0.0, double.infinity);
        return Row(
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
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: statusMax),
              child: _TrackStatusPills(trackId: track.id),
            ),
            // maxWidth budget only — force-width left a gap before actions.
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: metaWidth),
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
        );
      },
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
        // Title column width excludes status + trailing meta; status sits
        // immediately left of TrailingMeta so it does not float mid-row.
        final detailsWidth =
            constraints.maxWidth -
            artSize -
            kArtworkGap -
            metaWidth -
            kRowGutter -
            kActionsColumnWidth;
        final statusMax = (detailsWidth - kMinTitleWidth).clamp(
          0.0,
          double.infinity,
        );
        return Row(
          children: [
            _ArtworkThumb(trackId: track.id, size: artSize),
            const SizedBox(width: kArtworkGap),
            Expanded(
              // Measure the real title column: in-flow status pills shrink this
              // Expanded, so a status-free detailsWidth cap would leave
              // maxPillWidth too wide and force a third Wrap run past the
              // kMetaPillAreaHeight clip.
              child: LayoutBuilder(
                builder: (context, columnConstraints) {
                  final maxPillWidth =
                      (columnConstraints.maxWidth - kMetaPillGap) / 2;
                  return Column(
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
                      // hiding them. Two runs is the cap (see `maxPillWidth`);
                      // the clip is belt-and-braces so no label can paint
                      // outside the row. Skipped entirely when there is no room
                      // left: a zero-width cap would render the pills as an
                      // empty gap.
                      if (pills.isNotEmpty && maxPillWidth > 0)
                        Padding(
                          padding: const EdgeInsets.only(top: 3),
                          // maxHeight, not a fixed height. Reserving two runs
                          // when a row only needs one leaves dead space under
                          // the pills, which pushes the title + pills group off
                          // centre. The cap still bounds the area at the two
                          // runs the width cap above guarantees.
                          //
                          // Hung left by the pill's own inset so the metadata
                          // text lines up with the title above.
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
                  );
                },
              ),
            ),
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: statusMax),
              child: _TrackStatusPills(trackId: track.id),
            ),
            // maxWidth budget only — force-width left a gap before actions.
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: metaWidth),
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

/// In-flow status chips for a track's analysis / stem jobs.
///
/// Watches progress independently so a tick rebuilds the pills without
/// regenerating the row. [IgnorePointer] keeps selection and drag-to-deck on
/// the row; menus stay on the actions slot.
class _TrackStatusPills extends ConsumerWidget {
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
    final infos = trackStatusInfos(
      jobs: jobs,
      analyzing: analyzing,
      stemsGenerating: stemsGenerating,
    );
    if (infos.isEmpty) {
      return const SizedBox.shrink();
    }
    // LayoutBuilder + per-pill max: ellipsize under pressure without a
    // Flexible parent that would expand the group to the details width and
    // leave the label stranded left of TrailingMeta.
    return IgnorePointer(
      child: LayoutBuilder(
        builder: (context, constraints) {
          // Budget the inner Row after the trailing gap padding; otherwise two
          // pills sized to the outer max overflow by exactly [kMetaPillGap].
          final innerMax = constraints.maxWidth.isFinite
              ? (constraints.maxWidth - kMetaPillGap).clamp(
                  0.0,
                  double.infinity,
                )
              : double.infinity;
          final gapTotal = kMetaPillGap * (infos.length - 1);
          final perPillMax = infos.length == 1
              ? innerMax
              : ((innerMax - gapTotal) / infos.length).clamp(
                  0.0,
                  double.infinity,
                );
          return Padding(
            padding: const EdgeInsets.only(right: kMetaPillGap),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                for (var i = 0; i < infos.length; i++) ...[
                  if (i > 0) const SizedBox(width: kMetaPillGap),
                  ConstrainedBox(
                    constraints: BoxConstraints(maxWidth: perPillMax),
                    child: _TrackStatusPill(info: infos[i]),
                  ),
                ],
              ],
            ),
          );
        },
      ),
    );
  }
}

/// Progress bars only — pills live in the row via [_TrackStatusPills].
///
/// Lives as a row overlay so a progress tick rebuilds the bars without the
/// whole list, and so bars can paint flush on the row bottom edge.
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
    final infos = trackStatusInfos(
      jobs: jobs,
      analyzing: analyzing,
      stemsGenerating: stemsGenerating,
    );
    if (infos.isEmpty) {
      return const SizedBox.shrink();
    }
    final fractions = [for (final info in infos) ?info.fraction];
    if (fractions.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = context.theme;
    // Decorative only. Without this the bar's ColoredBox is the topmost hit
    // target, so Stack hit testing stops there and the row's pointer-down
    // selection and drag-to-deck never fire.
    return IgnorePointer(
      child: Stack(
        children: [
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
                  decoration: BoxDecoration(color: theme.colors.primaryTint),
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
    return Semantics(
      label: info.label,
      container: true,
      // Announce the phase once instead of also reading the spinner + text.
      excludeSemantics: true,
      child: MetaPill(
        text: info.label,
        fillWidth: false,
        leading: info.fraction == null
            ? MLoader(size: MLoaderSize.xs, color: theme.colors.mutedForeground)
            : null,
      ),
    );
  }
}
