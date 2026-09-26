import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/artwork_cache.dart';
import 'package:gui_flutter/library/focused_load.dart';
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
import 'package:gui_flutter/shell/m_loader.dart';
import 'package:gui_flutter/shell/mixar_context_menu.dart';
import 'package:gui_flutter/shell/mixar_input.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:trina_grid/trina_grid.dart';

/// Row selection fill. Forui neutral dark uses the same hex for `muted` and
/// `secondary`, so `theme.colors.muted` is invisible on the table surface.
Color libraryTableSelectedRowColor(MixarThemeData theme) => Color.alphaBlend(
  theme.colors.primary.withValues(alpha: 0.14),
  theme.colors.secondary,
);

/// Opacity applied to rows already committed in the open history session.
const kSessionPlayedRowOpacity = 0.3;

/// Trailing actions column width. The row status overlay keeps its pill clear
/// of it, so the two must not drift apart.
const kActionsColumnWidth = 44.0;

/// Gap between the status pill group and the actions column, and between
/// adjacent pills.
const kStatusPillGap = 8.0;

/// Progress bar thickness, and the gap that keeps stacked per-job bars apart.
/// The vertical stride is their sum, so changing one stays consistent.
const kStatusBarHeight = 2.0;
const kStatusBarGap = 1.0;

/// Identity for [TrinaGrid] remounts. Keep play/harmonic state out — those
/// change often and remounting the grid is what blinks the library table.
Object libraryTableRemountKey({
  required Object? sourceId,
  required List<String> tableColumns,
  required KeyColorMode keyColorMode,
  required KeyDisplayMode keyDisplayMode,
}) => (sourceId, tableColumns.join(','), keyColorMode, keyDisplayMode);

LibraryTrackSummary? _trackData(TrinaRow<dynamic> row) {
  final data = row.data;
  return data is LibraryTrackSummary ? data : null;
}

/// Filter + [trina_grid](https://github.com/doonfrs/trina_grid) track table.
class TrackTablePane extends ConsumerStatefulWidget {
  const new({super.key});

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
    final tableColumns = ref.watch(libraryTableColumnsProvider);
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s, orElse: defaultAppSettings);
    final keyDisplayMode = keyModeFromSettings(settings.keyDisplayMode);
    final keyColorMode = keyColorModeFromSettings(settings.keyColorMode);
    final config = _gridConfig(theme);

    // No listener on analyzing / track progress: both render in the per-row
    // status overlay. Regenerating rows here blinked the whole table on every
    // fraction tick (stems report many) and dropped cell state.

    // Session dim: each row Consumer watches sessionPlayedKeysProvider — do not
    // notifyListeners the grid when history commits (min play seconds).
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
          manager.appendRows(_rowsFor(tracks));
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
          MixarInput(
            hint: 'Filter tracks…',
            onChanged: (value) =>
                ref.read(trackFilterProvider.notifier).set(value),
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
                    loading: () => const Center(child: MLoader()),
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
                          // Isolate table paint from overlay tooltips / meters.
                          child: RepaintBoundary(
                            child: SizedBox.expand(
                              child: TrinaGrid(
                                // Remount only when columns/source identity change.
                                // Engine/drag/dim live in row Consumers — not this key.
                                key: ValueKey(
                                  libraryTableRemountKey(
                                    sourceId: drive ? drivePath : selectedId,
                                    tableColumns: tableColumns,
                                    keyColorMode: keyColorMode,
                                    keyDisplayMode: keyDisplayMode,
                                  ),
                                ),
                                columns: _columns(
                                  theme,
                                  tableColumns,
                                  keyDisplayMode: keyDisplayMode,
                                  keyColorMode: keyColorMode,
                                ),
                                rows: _rowsFor(tracks),
                                mode: TrinaGridMode.readOnly,
                                rowWrapper: _rowWrapper,
                                onLoaded: (e) {
                                  _manager = e.stateManager;
                                  e.stateManager.setShowColumnFilter(false);
                                  _attachScrollListener(e.stateManager);
                                  _requestVisibleArtwork(e.stateManager);
                                  ref
                                      .read(
                                        focusedTrackRowIndexProvider.notifier,
                                      )
                                      .setCount(_tracks.length);
                                  _applyMidiFocus(
                                    e.stateManager,
                                    ref.read(focusedTrackRowIndexProvider),
                                  );
                                },
                                onActiveCellChanged: (event) {
                                  if (event.idx < 0) {
                                    return;
                                  }
                                  _syncFocusedIndexFromVisual(event.idx);
                                },
                                rowColorCallback: (ctx) {
                                  final current =
                                      ctx.stateManager.currentRowIdx;
                                  if (current != null &&
                                      current == ctx.rowIdx) {
                                    return libraryTableSelectedRowColor(theme);
                                  }
                                  return theme.colors.secondary;
                                },
                                configuration: config,
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

  List<TrinaColumn> _columns(
    MixarThemeData theme,
    List<String> activeColumns, {
    required KeyDisplayMode keyDisplayMode,
    required KeyColorMode keyColorMode,
  }) {
    final visible = {
      for (final col in kLibraryColumnDefs)
        if (col.required || activeColumns.contains(col.id)) col.id,
    };
    final columns = [
      TrinaColumn(
        title: '',
        field: 'artwork',
        type: TrinaColumnType.text(),
        width: 36,
        minWidth: 36,
        // Default cell padding is horizontal:10; that leaves 16px in a 36px
        // column and squashes a square thumb into a tall rectangle.
        cellPadding: EdgeInsets.zero,
        suppressedAutoSize: true,
        enableContextMenu: false,
        enableDropToResize: false,
        enableSorting: false,
        renderer: (ctx) {
          final trackId = _trackData(ctx.row)?.id;
          if (trackId == null) {
            return const SizedBox.shrink();
          }
          const size = 28.0;
          Widget placeholder() => SizedBox.square(
            dimension: size,
            child: ColoredBox(
              color: theme.colors.muted,
              child: Center(
                child: Icon(
                  LucideIcons.disc2,
                  size: 16,
                  color: theme.colors.mutedForeground,
                ),
              ),
            ),
          );
          final bytes = ref.read(artworkCacheProvider)[trackId];
          if (bytes != null && bytes.isNotEmpty) {
            return Center(
              child: SizedBox.square(
                dimension: size,
                child: Image.memory(
                  bytes,
                  width: size,
                  height: size,
                  fit: BoxFit.cover,
                  cacheWidth: 56,
                  cacheHeight: 56,
                  errorBuilder: (_, _, _) => placeholder(),
                ),
              ),
            );
          }
          return Center(child: placeholder());
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
        ),
      if (visible.contains('artist'))
        TrinaColumn(
          title: 'Artist',
          field: 'artist',
          type: TrinaColumnType.text(),
          width: 180,
          minWidth: 96,
          enableContextMenu: false,
        ),
      if (visible.contains('album'))
        TrinaColumn(
          title: 'Album',
          field: 'album',
          type: TrinaColumnType.text(),
          width: 180,
          minWidth: 96,
          enableContextMenu: false,
        ),
      if (visible.contains('genre'))
        TrinaColumn(
          title: 'Genre',
          field: 'genre',
          type: TrinaColumnType.text(),
          width: 120,
          enableContextMenu: false,
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
          renderer: (ctx) {
            final raw = ctx.row.cells['key']?.value as String? ?? '';
            final label = raw.isEmpty ? '' : formatDeckKey(raw, keyDisplayMode);
            final baseStyle =
                ctx.stateManager.configuration.style.cellTextStyle;
            // Consumer watches harmonic ref so play/pause does not notifyListeners
            // the whole grid (that rebuilt Opacity and blinked played rows).
            return Consumer(
              builder: (context, ref, _) {
                final color = colorForKey(
                  raw,
                  keyColorMode,
                  harmonicReferenceKey: ref.watch(harmonicReferenceKeyProvider),
                );
                return Center(
                  child: Text(
                    label,
                    style: baseStyle.copyWith(
                      color: color ?? theme.colors.foreground,
                      fontWeight: color != null ? FontWeight.w600 : null,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                );
              },
            );
          },
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
        ),
      if (visible.contains('path'))
        TrinaColumn(
          title: 'Path',
          field: 'pathDisplay',
          type: TrinaColumnType.text(),
          width: 240,
          minWidth: 120,
          enableContextMenu: false,
        ),
      TrinaColumn(
        title: '',
        field: 'actions',
        type: TrinaColumnType.text(),
        width: kActionsColumnWidth,
        minWidth: kActionsColumnWidth,
        cellPadding: EdgeInsets.zero,
        suppressedAutoSize: true,
        enableContextMenu: false,
        enableDropToResize: false,
        enableSorting: false,
        renderer: (ctx) {
          final track = _trackData(ctx.row);
          if (track == null) {
            return const SizedBox.shrink();
          }
          final inLibrary = ctx.row.cells['inLibrary']?.value == true;
          final title = trackTitleLabel(track);
          // Watch job state so Analyze / Generate stems re-enable the moment
          // the job ends. The grid no longer regenerates rows for it.
          return Center(
            child: Consumer(
              builder: (context, ref, _) {
                final analyzing = ref.watch(
                  analyzingTrackIdsProvider.select(
                    (ids) => ids.contains(track.id),
                  ),
                );
                final stemsGenerating = ref.watch(
                  stemGeneratingTrackIdsProvider.select(
                    (ids) => ids.contains(track.id),
                  ),
                );
                return TrackActionsMenu(
                  trackId: track.id,
                  path: track.path,
                  title: title,
                  inLibrary: inLibrary,
                  analyzing: analyzing,
                  stemsGenerating: stemsGenerating,
                  enableSecondaryPress: false,
                );
              },
            ),
          );
        },
      ),
    ];
    final headerBg = Color.alphaBlend(
      theme.colors.foreground.withValues(alpha: 0.08),
      theme.colors.secondary,
    );
    for (final column in columns) {
      column.backgroundColor = headerBg;
    }
    return columns;
  }

  TrinaGridConfiguration _gridConfig(MixarThemeData theme) {
    final surface = theme.colors.secondary;
    final selected = libraryTableSelectedRowColor(theme);
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
        // Trina subtracts scrollbar thickness from column auto-size when this
        // is true, leaving an empty gutter even if the thumb is hidden.
        columnShowScrollWidth: false,
        showHorizontal: false,
      ),
      columnSize: const TrinaGridColumnSizeConfig(
        autoSizeMode: TrinaAutoSizeMode.scale,
        resizeMode: TrinaResizeMode.pushAndPull,
      ),
      style: TrinaGridStyleConfig(
        enableColumnBorderVertical: false,
        enableCellBorderVertical: false,
        gridBackgroundColor: surface,
        rowColor: surface,
        oddRowColor: surface,
        evenRowColor: surface,
        activatedColor: selected,
        // Keep selectingMode.none for deck drag; transparent current-cell
        // border so focus reads as a full-row fill (rowColorCallback + this).
        activatedBorderColor: const Color(0x00000000),
        unfocusedSelectionColor: selected,
        borderColor: theme.colors.border,
        gridBorderColor: theme.colors.border,
        inactivatedBorderColor: const Color(0x00000000),
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

  List<TrinaRow<dynamic>> _rowsFor(List<LibraryTrackSummary> tracks) {
    final tab = ref.read(librarySourceTabProvider);
    final resolved =
        ref.read(driveResolvedByPathProvider).asData?.value ?? const {};
    final rows = <TrinaRow<LibraryTrackSummary>>[
      for (final t in tracks)
        TrinaRow<LibraryTrackSummary>(
          data: t,
          cells: {
            'inLibrary': TrinaCell(
              value: trackIsInLibrary(
                t,
                tab: tab,
                driveResolvedByPath: resolved,
              ),
            ),
            'artwork': TrinaCell(value: t.id),
            'title': TrinaCell(value: trackTitleLabel(t)),
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
    return List<TrinaRow<dynamic>>.from(rows);
  }

  Widget _rowWrapper(
    BuildContext context,
    Widget rowWidget,
    TrinaRow<dynamic> rowData,
    TrinaGridStateManager stateManager,
  ) {
    final track = _trackData(rowData);
    if (track == null) {
      return rowWidget;
    }
    final inLibrary = rowData.cells['inLibrary']?.value == true;
    final title = trackTitleLabel(track);
    // Pointer-down (not tap): super_dnd's drag recognizer often wins the
    // gesture arena, so Trina's onTapUp never selects the row.
    return Listener(
      behavior: HitTestBehavior.translucent,
      onPointerDown: (_) => _selectVisualRow(stateManager, rowData),
      child: _TrackActionsContextMenu(
        trackId: track.id,
        path: track.path,
        title: title,
        inLibrary: inLibrary,
        // Watch engine + dim here so drag attaches after start without
        // remounting TrinaGrid (ValueKey no longer includes engineRunning).
        child: Consumer(
          builder: (context, ref, child) {
            final dimmed = ref.watch(
              sessionTrackDimmedProvider((track.id, track.path)),
            );
            // Overlay sits inside the drag/dim wrapper so it tracks the row
            // when it is dragged onto a deck. Positioned children only, so the
            // row height still measures from the Trina row alone.
            final row = Stack(
              children: [
                child ?? const SizedBox.shrink(),
                _TrackStatusOverlay(
                  trackId: track.id,
                  // The row slot is rowHeight + cellHorizontalBorderWidth.
                  // Anchor the bar to the painted content so the 1px
                  // separator stays below it instead of reading as a gap.
                  bottomInset: stateManager
                      .configuration
                      .style
                      .cellHorizontalBorderWidth,
                ),
              ],
            );
            final content = ref.watch(engineRunningProvider)
                ? _dragRowWrapper(context, row, rowData, stateManager)
                : row;
            return AnimatedOpacity(
              opacity: dimmed ? kSessionPlayedRowOpacity : 1,
              duration: const Duration(milliseconds: 120),
              child: content,
            );
          },
          child: rowWidget,
        ),
      ),
    );
  }

  void _selectVisualRow(
    TrinaGridStateManager manager,
    TrinaRow<dynamic> rowData,
  ) {
    final visualIndex = manager.refRows.indexOf(rowData);
    if (visualIndex < 0) {
      return;
    }
    final cell = rowData.cells['title'] ?? rowData.cells.values.first;
    manager.setCurrentCell(cell, visualIndex);
    manager.setKeepFocus(true);
    _syncFocusedIndexFromVisual(visualIndex);
  }

  void _syncFocusedIndexFromVisual(int visualIndex) {
    final manager = _manager;
    if (manager == null ||
        visualIndex < 0 ||
        visualIndex >= manager.refRows.length) {
      return;
    }
    final trackId = _trackData(manager.refRows[visualIndex])?.id;
    if (trackId == null) {
      return;
    }
    final tableIndex = _tracks.indexWhere((t) => t.id == trackId);
    if (tableIndex < 0) {
      return;
    }
    ref.read(focusedTrackRowIndexProvider.notifier).set(tableIndex);
  }

  Widget _dragRowWrapper(
    BuildContext context,
    Widget rowWidget,
    TrinaRow<dynamic> rowData,
    TrinaGridStateManager stateManager,
  ) {
    final track = _trackData(rowData);
    if (track == null) {
      return rowWidget;
    }
    final inLibrary = rowData.cells['inLibrary']?.value == true;
    final title = trackTitleLabel(track);
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
        child: rowWidget,
      ),
    );
  }

  void _applyMidiFocus(TrinaGridStateManager manager, int index) {
    final visualIndex = visualRowIndexForFocusedTrack(
      [for (final row in manager.refRows) _trackData(row)?.id],
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

/// In-flight analysis / stem job chrome drawn over a track row.
///
/// Lives in the row wrapper rather than the title cell so a progress tick
/// rebuilds one row instead of regenerating every row in the grid.
class _TrackStatusOverlay extends ConsumerWidget {
  const new({required this.trackId, required this.bottomInset});

  final String trackId;

  /// Trina reserves a horizontal cell border below each row's painted content.
  /// Progress bars are offset by it so they sit flush on the content edge.
  final double bottomInset;

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
    // At most one job per lane, so at most two pills.
    final infos =
        <TrackProgressInfo>[
          ...jobs,
          if (analyzing && !jobs.any((i) => !isStemProgressPhase(i.phase)))
            const TrackProgressInfo(phase: 'analyze'),
          if (stemsGenerating && !jobs.any((i) => isStemProgressPhase(i.phase)))
            const TrackProgressInfo(phase: 'stems_queued'),
        ]..sort(
          (a, b) => (isStemProgressPhase(a.phase) ? 1 : 0).compareTo(
            isStemProgressPhase(b.phase) ? 1 : 0,
          ),
        );
    if (infos.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = context.theme;
    return Stack(
      children: [
        // Span the row so the pill group stays flush right as it grows —
        // a right-only Positioned hands its child loose constraints, which
        // Center then centres inside the inset box and drifts left.
        Positioned(
          left: 0,
          right: 0,
          top: 0,
          bottom: 0,
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
        // One bar per job, stacked in the same order as the pills above.
        for (var i = 0; i < infos.length; i++)
          if (infos[i].fraction case final fraction?)
            Positioned(
              left: 0,
              right: 0,
              bottom: bottomInset + i * (kStatusBarHeight + kStatusBarGap),
              child: SizedBox(
                height: kStatusBarHeight,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: theme.colors.primary.withValues(alpha: 0.18),
                  ),
                  child: Align(
                    alignment: Alignment.centerLeft,
                    child: FractionallySizedBox(
                      widthFactor: fraction.clamp(0.0, 1.0),
                      child: SizedBox.expand(
                        child: ColoredBox(color: theme.colors.primary),
                      ),
                    ),
                  ),
                ),
              ),
            ),
      ],
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
    required this.child,
  });

  final String trackId;
  final String path;
  final String title;
  final bool inLibrary;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final engineRunning = ref.watch(engineRunningProvider);
    // Watched here rather than read in the row wrapper: the grid no longer
    // regenerates rows when a job starts or ends, so a read would go stale.
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
                          unawaited(load(0));
                        },
                      ),
                      _LoadDeckChip(
                        letter: 'B',
                        color: FaderColors.b.grip,
                        enabled: engineRunning,
                        onPress: () {
                          dismiss();
                          unawaited(load(1));
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
