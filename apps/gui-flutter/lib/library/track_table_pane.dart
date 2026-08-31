import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/library/focused_load.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/track_detail_dialog.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:trina_grid/trina_grid.dart';

/// Opacity applied to rows already committed in the open history session.
const kSessionPlayedRowOpacity = 0.45;

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
    final tab = ref.read(librarySourceTabProvider);
    final resolved =
        ref.read(driveResolvedByPathProvider).asData?.value ?? const {};
    final ids = [
      for (final t in _tracks)
        if (trackIsInLibrary(t, tab: tab, driveResolvedByPath: resolved)) t.id,
    ];
    if (ids.isEmpty) {
      return;
    }
    final scroll = manager.scroll.bodyRowsVertical;
    if (scroll == null || !scroll.hasClients) {
      ref
          .read(artworkCacheProvider.notifier)
          .ensureLoaded(ids.take(30).toList());
      return;
    }
    final rowH = manager.rowTotalHeight;
    if (rowH <= 0) {
      return;
    }
    final first = (scroll.offset / rowH).floor().clamp(0, _tracks.length - 1);
    final count = (scroll.position.viewportDimension / rowH).ceil() + 2;
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
    final analyzingId = ref.watch(analyzingTrackIdProvider);
    final engineRunning = ref.watch(engineRunningProvider);
    final tableColumns = ref.watch(libraryTableColumnsProvider);
    ref.watch(sessionPlayedKeysProvider);
    final config = _gridConfig(theme);

    ref.listen(analyzingTrackIdProvider, (_, next) {
      final manager = _manager;
      if (manager == null || _tracks.isEmpty) {
        return;
      }
      manager.removeAllRows();
      manager.appendRows(_rowsFor(_tracks, next));
      _applyMidiFocus(manager, ref.read(focusedTrackRowIndexProvider));
    });
    ref.listen(sessionPlayedKeysProvider, (_, _) {
      final manager = _manager;
      if (manager == null || _tracks.isEmpty) {
        return;
      }
      manager.removeAllRows();
      manager.appendRows(
        _rowsFor(_tracks, ref.read(analyzingTrackIdProvider)),
      );
      _applyMidiFocus(manager, ref.read(focusedTrackRowIndexProvider));
    });
    ref.listen(artworkCacheProvider, (_, _) {
      _manager?.notifyListeners();
    });
    ref.listen(focusedTrackRowIndexProvider, (_, index) {
      final manager = _manager;
      if (manager != null) {
        _applyMidiFocus(manager, index);
      }
    });

    ref.listen(libraryTableTracksProvider, (_, next) {
      ref
          .read(focusedTrackRowIndexProvider.notifier)
          .setCount(next.asData?.value.length ?? 0);
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
        _applyMidiFocus(manager, ref.read(focusedTrackRowIndexProvider));
        _requestVisibleArtwork(manager);
      });
    });

    return Padding(
      padding: const EdgeInsets.fromLTRB(6, 0, 0, 0),
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
            child: !drive && selectedId == null
                ? Center(
                    child: Text(
                      'Select a collection',
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.mutedForeground,
                      ),
                    ),
                  )
                : drive && drivePath == null
                ? Center(
                    child: Text(
                      'Select a drive or folder to browse audio files',
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
                            drive
                                ? 'No audio files in this folder'
                                : 'No tracks',
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
                            // TrinaGrid only reads rowWrapper at construct.
                            key: ValueKey((
                              drive ? drivePath : selectedId,
                              engineRunning,
                              tableColumns.join(','),
                            )),
                            columns: _columns(theme, tableColumns),
                            rows: _rowsFor(tracks, analyzingId),
                            mode: TrinaGridMode.readOnly,
                            rowWrapper: _rowWrapper,
                            onLoaded: (e) {
                              _manager = e.stateManager;
                              e.stateManager.setShowColumnFilter(false);
                              _attachScrollListener(e.stateManager);
                              _requestVisibleArtwork(e.stateManager);
                              ref
                                  .read(focusedTrackRowIndexProvider.notifier)
                                  .setCount(_tracks.length);
                              _applyMidiFocus(
                                e.stateManager,
                                ref.read(focusedTrackRowIndexProvider),
                              );
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

  List<TrinaColumn> _columns(FThemeData theme, List<String> activeColumns) {
    final visible = {
      for (final col in kLibraryColumnDefs)
        if (col.required || activeColumns.contains(col.id)) col.id,
    };
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
          final bytes = ref.read(artworkCacheProvider)[trackId];
          if (bytes != null && bytes.isNotEmpty) {
            return Center(
              child: Image.memory(
                bytes,
                width: 28,
                height: 28,
                fit: BoxFit.cover,
                cacheWidth: 56,
                cacheHeight: 56,
                errorBuilder: (_, _, _) =>
                    Container(width: 28, height: 28, color: theme.colors.muted),
              ),
            );
          }
          return Center(
            child: Container(width: 28, height: 28, color: theme.colors.muted),
          );
        },
      ),
      if (visible.contains('title'))
        TrinaColumn(
          title: 'Title',
          field: 'title',
          type: TrinaColumnType.text(),
          width: 280,
          minWidth: 120,
          enableContextMenu: false,
          enableDropToResize: true,
        ),
      if (visible.contains('artist'))
        TrinaColumn(
          title: 'Artist',
          field: 'artist',
          type: TrinaColumnType.text(),
          width: 180,
          minWidth: 96,
          enableContextMenu: false,
          enableDropToResize: true,
        ),
      if (visible.contains('album'))
        TrinaColumn(
          title: 'Album',
          field: 'album',
          type: TrinaColumnType.text(),
          width: 180,
          minWidth: 96,
          enableContextMenu: false,
          enableDropToResize: true,
        ),
      if (visible.contains('genre'))
        TrinaColumn(
          title: 'Genre',
          field: 'genre',
          type: TrinaColumnType.text(),
          width: 120,
          minWidth: 80,
          enableContextMenu: false,
          enableDropToResize: true,
        ),
      if (visible.contains('bpm'))
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
      if (visible.contains('key'))
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
      if (visible.contains('duration'))
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
      if (visible.contains('path'))
        TrinaColumn(
          title: 'Path',
          field: 'pathDisplay',
          type: TrinaColumnType.text(),
          width: 240,
          minWidth: 120,
          enableContextMenu: false,
          enableDropToResize: true,
        ),
      TrinaColumn(
        title: '',
        field: 'actions',
        type: TrinaColumnType.text(),
        width: 44,
        minWidth: 44,
        cellPadding: EdgeInsets.zero,
        suppressedAutoSize: true,
        enableContextMenu: false,
        enableDropToResize: false,
        enableSorting: false,
        renderer: (ctx) {
          final trackId = ctx.row.cells['trackId']?.value as String?;
          final path = ctx.row.cells['path']?.value as String?;
          if (trackId == null || path == null) {
            return const SizedBox.shrink();
          }
          final analyzing = ref.read(analyzingTrackIdProvider) == trackId;
          final inLibrary = ctx.row.cells['inLibrary']?.value == true;
          final title =
              ctx.row.cells['dragTitle']?.value as String? ??
              ctx.row.cells['title']?.value as String? ??
              '';
          return Center(
            child: TrackActionsMenu(
              trackId: trackId,
              path: path,
              title: title,
              inLibrary: inLibrary,
              analyzing: analyzing,
              enableSecondaryPress: false,
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
      rowWrapperIsConstantHeight: true,
      selectingMode: TrinaGridSelectingMode.none,
      scrollbar: const TrinaGridScrollbarConfig(
        // Mouse-drag scrolls steal super_dnd's 4px ImmediateMultiDrag when
        // dragging a row up onto the decks. Wheel + thumb still scroll.
        dragDevices: {
          PointerDeviceKind.touch,
          PointerDeviceKind.stylus,
          PointerDeviceKind.invertedStylus,
          PointerDeviceKind.trackpad,
        },
      ),
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
    final tab = ref.read(librarySourceTabProvider);
    final resolved =
        ref.read(driveResolvedByPathProvider).asData?.value ?? const {};
    return [
      for (final t in tracks)
        TrinaRow(
          cells: {
            'trackId': TrinaCell(value: t.id),
            'inLibrary': TrinaCell(
              value: trackIsInLibrary(
                t,
                tab: tab,
                driveResolvedByPath: resolved,
              ),
            ),
            'path': TrinaCell(value: t.path),
            'dragTitle': TrinaCell(value: trackTitleLabel(t)),
            'artwork': TrinaCell(value: t.id),
            'title': TrinaCell(
              value: analyzingId == t.id
                  ? '${trackTitleLabel(t)} …'
                  : trackTitleLabel(t),
            ),
            'artist': TrinaCell(value: t.artist ?? ''),
            'album': TrinaCell(value: t.album ?? ''),
            'genre': TrinaCell(value: t.genre ?? ''),
            'bpm': TrinaCell(
              value: t.bpm == null ? '' : t.bpm!.toStringAsFixed(1),
            ),
            'key': TrinaCell(value: t.key ?? ''),
            'length': TrinaCell(value: _formatDuration(t.durationMs)),
            'pathDisplay': TrinaCell(value: t.path),
            'actions': TrinaCell(value: t.id),
          },
        ),
    ];
  }

  Widget _rowWrapper(
    BuildContext context,
    Widget rowWidget,
    TrinaRow rowData,
    TrinaGridStateManager stateManager,
  ) {
    final engineRunning = ref.read(engineRunningProvider);
    final inner = engineRunning
        ? _dragRowWrapper(context, rowWidget, rowData, stateManager)
        : rowWidget;
    final path = rowData.cells['path']?.value as String?;
    final trackId = rowData.cells['trackId']?.value as String?;
    if (path == null || trackId == null) {
      return inner;
    }
    final played =
        ref.read(sessionPlayedKeysProvider).asData?.value ??
        SessionPlayedKeys.empty;
    final dimmed = played.matches(trackId: trackId, path: path);
    final row = dimmed
        ? Opacity(opacity: kSessionPlayedRowOpacity, child: inner)
        : inner;
    final inLibrary = rowData.cells['inLibrary']?.value == true;
    final title =
        rowData.cells['dragTitle']?.value as String? ??
        rowData.cells['title']?.value as String? ??
        '';
    final analyzing = ref.read(analyzingTrackIdProvider) == trackId;
    return _TrackActionsContextMenu(
      trackId: trackId,
      path: path,
      title: title,
      inLibrary: inLibrary,
      analyzing: analyzing,
      child: row,
    );
  }

  Widget _dragRowWrapper(
    BuildContext context,
    Widget rowWidget,
    TrinaRow rowData,
    TrinaGridStateManager stateManager,
  ) {
    final path = rowData.cells['path']?.value as String?;
    final trackId = rowData.cells['trackId']?.value as String?;
    final inLibrary = rowData.cells['inLibrary']?.value == true;
    final title =
        rowData.cells['dragTitle']?.value as String? ??
        rowData.cells['title']?.value as String? ??
        '';
    if (path == null) {
      return rowWidget;
    }
    final payload = TrackDragPayload(
      source: inLibrary ? TrackDragSource.library : TrackDragSource.filesystem,
      trackId: inLibrary ? trackId : null,
      path: path,
      title: trackDisplayTitle(title: title, path: path),
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
        child: rowWidget,
      ),
    );
  }

  void _applyMidiFocus(TrinaGridStateManager manager, int index) {
    final visualIndex = visualRowIndexForFocusedTrack(
      [
        for (final row in manager.refRows)
          row.cells['trackId']?.value as String?,
      ],
      [for (final track in _tracks) track.id],
      index,
    );
    if (visualIndex == null || visualIndex >= manager.refRows.length) {
      return;
    }
    final row = manager.refRows[visualIndex];
    final cell = row.cells['title'] ?? row.cells.values.first;
    manager.setCurrentCell(cell, visualIndex);
    final scroll = manager.scroll.bodyRowsVertical;
    if (scroll == null || !scroll.hasClients) {
      return;
    }
    final rowH = manager.rowTotalHeight;
    if (rowH <= 0) {
      return;
    }
    final target = visualIndex * rowH;
    final view = scroll.position.viewportDimension;
    final offset = scroll.offset;
    if (target < offset) {
      scroll.jumpTo(target.clamp(0, scroll.position.maxScrollExtent));
    } else if (target + rowH > offset + view) {
      scroll.jumpTo(
        (target + rowH - view).clamp(0, scroll.position.maxScrollExtent),
      );
    }
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

class TrackActionsMenu extends ConsumerWidget {
  const TrackActionsMenu({
    required this.trackId,
    required this.path,
    required this.title,
    required this.inLibrary,
    required this.analyzing,
    this.enableSecondaryPress = true,
    super.key,
  });

  final String trackId;
  final String path;
  final String title;
  final bool inLibrary;
  final bool analyzing;
  final bool enableSecondaryPress;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final engineRunning = ref.watch(engineRunningProvider);
    return FPopoverMenu(
      faded: null,
      overlayLocation: OverlayChildLocation.rootOverlay,
      menuBuilder: (context, controller, _) => _trackActionGroups(
        context: context,
        ref: ref,
        controller: controller,
        trackId: trackId,
        path: path,
        title: title,
        inLibrary: inLibrary,
        analyzing: analyzing,
        engineRunning: engineRunning,
      ),
      builder: (context, controller, child) => FButton.icon(
        variant: .ghost,
        size: .xs,
        semanticsLabel: 'Track actions',
        onPress: analyzing ? null : controller.toggle,
        onSecondaryPress: analyzing || !enableSecondaryPress
            ? null
            : controller.toggle,
        child: child!,
      ),
      child: analyzing
          ? const FCircularProgress()
          : const Icon(FLucideIcons.ellipsisVertical),
    );
  }
}

class _TrackActionsContextMenu extends ConsumerWidget {
  const _TrackActionsContextMenu({
    required this.trackId,
    required this.path,
    required this.title,
    required this.inLibrary,
    required this.analyzing,
    required this.child,
  });

  final String trackId;
  final String path;
  final String title;
  final bool inLibrary;
  final bool analyzing;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final engineRunning = ref.watch(engineRunningProvider);
    return FContextMenu(
      faded: null,
      overlayLocation: OverlayChildLocation.rootOverlay,
      secondaryPress: !analyzing,
      menuBuilder: (context, controller, _) => _trackActionGroups(
        context: context,
        ref: ref,
        controller: controller,
        trackId: trackId,
        path: path,
        title: title,
        inLibrary: inLibrary,
        analyzing: analyzing,
        engineRunning: engineRunning,
      ),
      child: child,
    );
  }
}

List<FItemGroupMixin> _trackActionGroups({
  required BuildContext context,
  required WidgetRef ref,
  required FPopoverController controller,
  required String trackId,
  required String path,
  required String title,
  required bool inLibrary,
  required bool analyzing,
  required bool engineRunning,
}) {
  Future<void> load(int deckId) {
    return loadPayloadToDeck(
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
    );
  }

  return [
    .group(
      children: [
        .raw(
          style: .delta(
            rawContentStyle: .delta(padding: .value(.fromLTRB(8, 8, 8, 2))),
          ),
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
                      unawaited(controller.hide());
                      unawaited(load(0));
                    },
                  ),
                  _LoadDeckChip(
                    letter: 'B',
                    color: FaderColors.b.grip,
                    enabled: engineRunning,
                    onPress: () {
                      unawaited(controller.hide());
                      unawaited(load(1));
                    },
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    ),
    .group(
      children: [
        .item(
          title: Text(analyzing ? 'Analyzing…' : 'Analyze'),
          enabled: inLibrary && !analyzing,
          onPress: !inLibrary || analyzing
              ? null
              : () {
                  unawaited(controller.hide());
                  unawaited(analyzeTrackAction(ref, trackId));
                },
        ),
        .item(
          title: const Text('Refresh'),
          enabled: inLibrary,
          onPress: inLibrary
              ? () {
                  unawaited(controller.hide());
                  unawaited(refreshTrackAction(ref, trackId));
                }
              : null,
        ),
        .item(
          title: const Text('Track details…'),
          enabled: inLibrary,
          onPress: inLibrary
              ? () {
                  unawaited(controller.hide());
                  unawaited(
                    showTrackDetailDialog(
                      context,
                      ref,
                      trackId: trackId,
                    ),
                  );
                }
              : null,
        ),
      ],
    ),
  ];
}

class _LoadDeckChip extends StatelessWidget {
  const _LoadDeckChip({
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
      child: FButton(
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
  const _TrackDragCard({required this.title});

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
