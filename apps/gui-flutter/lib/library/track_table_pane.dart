import 'dart:typed_data';

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:trina_grid/trina_grid.dart';

/// Filter + [trina_grid](https://github.com/doonfrs/trina_grid) track table.
class TrackTablePane extends ConsumerStatefulWidget {
  const TrackTablePane({super.key});

  @override
  ConsumerState<TrackTablePane> createState() => _TrackTablePaneState();
}

class _TrackTablePaneState extends ConsumerState<TrackTablePane> {
  TrinaGridStateManager? _manager;
  List<LibraryTrackSummary> _tracks = const [];
  VoidCallback? _scrollListener;

  @override
  void dispose() {
    _detachScrollListener();
    super.dispose();
  }

  void _detachScrollListener() {
    final listener = _scrollListener;
    final scroll = _manager?.scroll.bodyRowsVertical;
    if (listener != null && scroll != null) {
      scroll.removeListener(listener);
    }
    _scrollListener = null;
  }

  void _attachScrollListener(TrinaGridStateManager manager) {
    _detachScrollListener();
    final scroll = manager.scroll.bodyRowsVertical;
    if (scroll == null) {
      return;
    }
    _scrollListener = () => _requestVisibleArtwork(manager);
    scroll.addListener(_scrollListener!);
  }

  void _requestVisibleArtwork(TrinaGridStateManager manager) {
    if (_tracks.isEmpty) {
      return;
    }
    final scroll = manager.scroll.bodyRowsVertical;
    if (scroll == null || !scroll.hasClients) {
      ref
          .read(artworkCacheProvider.notifier)
          .ensureLoaded(ref, _tracks.take(30).map((t) => t.id).toList());
      return;
    }
    final rowH = manager.rowTotalHeight;
    if (rowH <= 0) {
      return;
    }
    final first = (scroll.offset / rowH).floor().clamp(0, _tracks.length - 1);
    final count = (scroll.position.viewportDimension / rowH).ceil() + 2;
    final last = (first + count).clamp(0, _tracks.length);
    final ids = [for (var i = first; i < last; i++) _tracks[i].id];
    ref.read(artworkCacheProvider.notifier).ensureLoaded(ref, ids);
  }

  @override
  Widget build(BuildContext context) {
    ref.watch(libraryEventsBootstrapProvider);
    final theme = context.theme;
    final selectedId = ref.watch(activeCollectionIdProvider);
    final tracksAsync = ref.watch(filteredTracksProvider);
    final analyzingId = ref.watch(analyzingTrackIdProvider);
    final artwork = ref.watch(artworkCacheProvider);
    final config = _gridConfig(theme);

    ref.listen(analyzingTrackIdProvider, (_, next) {
      final manager = _manager;
      if (manager == null || _tracks.isEmpty) {
        return;
      }
      manager.removeAllRows();
      manager.appendRows(_rowsFor(_tracks, next));
    });

    ref.listen(filteredTracksProvider, (_, next) {
      final manager = _manager;
      if (manager == null) {
        return;
      }
      next.whenData((tracks) {
        _tracks = tracks;
        manager.removeAllRows();
        if (tracks.isNotEmpty) {
          manager.appendRows(_rowsFor(tracks, analyzingId));
        }
        _requestVisibleArtwork(manager);
      });
    });

    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          FTextField(
            hint: 'Filter tracks…',
            control: FTextFieldManagedControl(
              onChange: (value) =>
                  ref.read(trackFilterProvider.notifier).set(value.text),
            ),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: selectedId == null
                ? Center(
                    child: Text(
                      'Select a collection',
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.mutedForeground,
                      ),
                    ),
                  )
                : tracksAsync.when(
                    loading: () => const Center(child: FCircularProgress()),
                    error: (e, _) => Text(
                      'Tracks error: $e',
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.destructive,
                      ),
                    ),
                    data: (tracks) {
                      _tracks = tracks;
                      if (tracks.isEmpty) {
                        return Center(
                          child: Text(
                            'No tracks',
                            style: theme.typography.body.sm.copyWith(
                              color: theme.colors.mutedForeground,
                            ),
                          ),
                        );
                      }
                      return DecoratedBox(
                        decoration: BoxDecoration(
                          color: theme.colors.secondary,
                          borderRadius: theme.style.borderRadius.md,
                          border: Border.all(color: theme.colors.border),
                        ),
                        child: ClipRRect(
                          borderRadius: theme.style.borderRadius.md,
                          child: TrinaGrid(
                            key: ValueKey(selectedId),
                            columns: _columns(theme, artwork, analyzingId),
                            rows: _rowsFor(tracks, analyzingId),
                            mode: TrinaGridMode.readOnly,
                            onLoaded: (e) {
                              _manager = e.stateManager;
                              e.stateManager.setShowColumnFilter(false);
                              _attachScrollListener(e.stateManager);
                              _requestVisibleArtwork(e.stateManager);
                            },
                            configuration: config,
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

  List<TrinaColumn> _columns(
    FThemeData theme,
    Map<String, Uint8List?> artwork,
    String? analyzingId,
  ) {
    return [
      TrinaColumn(
        title: '',
        field: 'artwork',
        type: TrinaColumnType.text(),
        width: 44,
        minWidth: 44,
        enableContextMenu: false,
        enableDropToResize: false,
        enableSorting: false,
        renderer: (ctx) {
          final trackId = ctx.row.cells['trackId']?.value as String?;
          if (trackId == null) {
            return const SizedBox.shrink();
          }
          final bytes = artwork[trackId];
          if (bytes != null && bytes.isNotEmpty) {
            return Center(
              child: Image.memory(bytes, width: 28, height: 28, fit: BoxFit.cover),
            );
          }
          return Center(
            child: Container(
              width: 28,
              height: 28,
              color: theme.colors.muted,
            ),
          );
        },
      ),
      TrinaColumn(
        title: 'Title',
        field: 'title',
        type: TrinaColumnType.text(),
        width: 280,
        minWidth: 120,
        enableContextMenu: false,
        enableDropToResize: true,
      ),
      TrinaColumn(
        title: 'Artist',
        field: 'artist',
        type: TrinaColumnType.text(),
        width: 180,
        minWidth: 96,
        enableContextMenu: false,
        enableDropToResize: true,
      ),
      TrinaColumn(
        title: 'BPM',
        field: 'bpm',
        type: TrinaColumnType.text(),
        width: 72,
        minWidth: 56,
        textAlign: TrinaColumnTextAlign.right,
        titleTextAlign: TrinaColumnTextAlign.right,
        enableContextMenu: false,
        enableDropToResize: true,
      ),
      TrinaColumn(
        title: 'Key',
        field: 'key',
        type: TrinaColumnType.text(),
        width: 64,
        minWidth: 48,
        textAlign: TrinaColumnTextAlign.center,
        titleTextAlign: TrinaColumnTextAlign.center,
        enableContextMenu: false,
        enableDropToResize: true,
      ),
      TrinaColumn(
        title: 'Length',
        field: 'length',
        type: TrinaColumnType.text(),
        width: 80,
        minWidth: 64,
        textAlign: TrinaColumnTextAlign.right,
        titleTextAlign: TrinaColumnTextAlign.right,
        enableContextMenu: false,
        enableDropToResize: true,
      ),
      TrinaColumn(
        title: '',
        field: 'actions',
        type: TrinaColumnType.text(),
        width: 40,
        minWidth: 40,
        enableContextMenu: false,
        enableDropToResize: false,
        enableSorting: false,
        renderer: (ctx) {
          final trackId = ctx.row.cells['trackId']?.value as String?;
          if (trackId == null) {
            return const SizedBox.shrink();
          }
          final analyzing = analyzingId == trackId;
          return Center(
            child: FPopoverMenu(
              menu: [
                FItemGroup(
                  children: [
                    FItem(
                      title: Text(analyzing ? 'Analyzing…' : 'Analyze'),
                      enabled: !analyzing,
                      onPress: analyzing
                          ? null
                          : () => analyzeTrackAction(ref, trackId),
                    ),
                    FItem(
                      title: const Text('Refresh'),
                      onPress: () => refreshTrackAction(ref, trackId),
                    ),
                  ],
                ),
              ],
              child: analyzing
                  ? const FCircularProgress(size: .sm)
                  : const Text('⋯'),
            ),
          );
        },
      ),
    ];
  }

  TrinaGridConfiguration _gridConfig(FThemeData theme) {
    final surface = theme.colors.secondary;
    final stripe = Color.alphaBlend(
      theme.colors.foreground.withValues(alpha: 0.04),
      surface,
    );
    final text = theme.typography.body.sm.copyWith(
      color: theme.colors.foreground,
    );
    final header = theme.typography.body.sm.copyWith(
      color: theme.colors.mutedForeground,
      fontWeight: FontWeight.w600,
    );

    return TrinaGridConfiguration(
      columnSize: const TrinaGridColumnSizeConfig(
        autoSizeMode: TrinaAutoSizeMode.scale,
        resizeMode: TrinaResizeMode.pushAndPull,
      ),
      style: TrinaGridStyleConfig(
        enableGridBorderShadow: false,
        enableColumnBorderVertical: false,
        enableCellBorderVertical: false,
        enableCellBorderHorizontal: true,
        gridBackgroundColor: surface,
        rowColor: surface,
        oddRowColor: stripe,
        evenRowColor: surface,
        activatedColor: theme.colors.muted,
        activatedBorderColor: theme.colors.primary,
        borderColor: theme.colors.border,
        gridBorderColor: theme.colors.border,
        inactivatedBorderColor: theme.colors.border,
        cellColorInEditState: surface,
        cellColorInReadOnlyState: surface,
        cellTextStyle: text,
        columnTextStyle: header,
        iconColor: theme.colors.mutedForeground,
        menuBackgroundColor: theme.colors.background,
        rowHeight: 36,
        columnHeight: 40,
        gridBorderWidth: 0,
        gridPadding: 0,
        gridBorderRadius: theme.style.borderRadius.md,
      ),
    );
  }

  List<TrinaRow> _rowsFor(
    List<LibraryTrackSummary> tracks,
    String? analyzingId,
  ) {
    return [
      for (final t in tracks)
        TrinaRow(
          cells: {
            'trackId': TrinaCell(value: t.id),
            'artwork': TrinaCell(value: t.id),
            'title': TrinaCell(
              value: analyzingId == t.id
                  ? '${trackTitleLabel(t)} …'
                  : trackTitleLabel(t),
            ),
            'artist': TrinaCell(value: t.artist ?? ''),
            'bpm': TrinaCell(
              value: t.bpm == null ? '' : t.bpm!.toStringAsFixed(1),
            ),
            'key': TrinaCell(value: t.key ?? ''),
            'length': TrinaCell(value: _formatDuration(t.durationMs)),
            'actions': TrinaCell(value: t.id),
          },
        ),
    ];
  }

  String _formatDuration(int? ms) {
    if (ms == null || ms <= 0) {
      return '';
    }
    final totalSec = ms ~/ 1000;
    final m = totalSec ~/ 60;
    final s = totalSec % 60;
    return '$m:${s.toString().padLeft(2, '0')}';
  }
}
