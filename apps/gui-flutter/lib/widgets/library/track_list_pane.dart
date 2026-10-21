import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/artwork_cache.dart';
import 'package:gui_flutter/providers/library/focused_load.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/collection_actions_menu.dart';
import 'package:gui_flutter/widgets/library/library_layout_scope.dart';
import 'package:gui_flutter/widgets/library/library_list_message.dart';
import 'package:gui_flutter/widgets/library/library_pane_back_button.dart';
import 'package:gui_flutter/widgets/library/library_pane_toolbar.dart';
import 'package:gui_flutter/widgets/library/library_row_density_button.dart';
import 'package:gui_flutter/widgets/library/track_artwork_thumb.dart';
import 'package:gui_flutter/widgets/library/track_edit_drawer.dart';
import 'package:gui_flutter/widgets/library/track_list_comfortable_row.dart';
import 'package:gui_flutter/widgets/library/track_list_sort_menu.dart';
import 'package:gui_flutter/widgets/library/track_list_view.dart';
import 'package:gui_flutter/widgets/library/track_status_pills.dart';
import 'package:gui_flutter/widgets/library/track_title_line.dart';
import 'package:gui_flutter/widgets/library/trailing_meta.dart';
import 'package:gui_flutter/widgets/shell/mixar_menu_item.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Progress bar thickness, and the gap that keeps stacked per-job bars apart.
/// The vertical stride is their sum, so changing one stays consistent.
const kStatusBarHeight = 2.0;
const kStatusBarGap = 1.0;

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
          leading: (LibraryLayoutScope.maybeOf(context)?.narrow ?? false)
              ? const LibraryPaneBackButton()
              : null,
          hint: 'Filter tracks…',
          onChanged: (value) =>
              ref.read(trackFilterProvider.notifier).set(value),
          trailing: [
            TrackListSortMenu(),
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
                      : TrackListComfortableRow(
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

/// Track-row menu items (Analyze, Generate stems, Refresh, Edit), shared by
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
      title: const Text('Edit…'),
      enabled: inLibrary,
      onPress: inLibrary
          ? () {
              // The menu's context/ref are disposed by `dismiss()`, so start
              // the async open first and hand the drawer the navigator's
              // stable context plus the app-scoped container.
              final navigator = Navigator.of(context);
              final container = ProviderScope.containerOf(
                context,
                listen: false,
              );
              final opening = showTrackEditDrawer(
                navigator.context,
                container,
                trackId: track.id,
              );
              dismiss();
              unawaited(opening);
            }
          : null,
    ),
  ];
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
        final extraMetaWidth =
            (trackShowsRating(track.rating)
                ? kMetaPillGap + kRatingSlotWidth
                : 0.0) +
            (trackShowsColorLabel(track.colorLabel)
                ? kMetaPillGap + kColorSlotWidth
                : 0.0);
        final metaWidth = trailingMetaWidth(
          constraints.maxWidth,
          artSize + kArtworkGap,
          // Compact has no metadata pills, so the trailing group carries
          // the length too and must reserve room for it.
          natural:
              kDurationSlotWidth +
              kMetaPillGap +
              kTrailingMetaBaseWidth +
              extraMetaWidth,
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
            TrackArtworkThumb(trackId: track.id, size: artSize),
            const SizedBox(width: kArtworkGap),
            Expanded(
              // Same identity line as the comfortable row: title, bullet,
              // artist — so both densities name the track the same way.
              child: TrackTitleLine(
                title: title,
                artist: track.artist ?? '',
                titleStyle: textStyle,
              ),
            ),
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: statusMax),
              child: TrackStatusPills(trackId: track.id),
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
                rating: track.rating,
                colorLabel: track.colorLabel,
              ),
            ),
            actionsSlot,
          ],
        );
      },
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
