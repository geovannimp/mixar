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
    final analyzingIds = ref.watch(analyzingTrackIdsProvider);
    final tableColumns = ref.watch(libraryTableColumnsProvider);
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s, orElse: defaultAppSettings);
    final keyDisplayMode = keyModeFromSettings(settings.keyDisplayMode);
    final keyColorMode = keyColorModeFromSettings(settings.keyColorMode);
    final config = _gridConfig(theme);

    ref.listen(analyzingTrackIdsProvider, (_, next) {
      final manager = _manager;
      if (manager == null || _tracks.isEmpty) {
        return;
      }
      manager.removeAllRows();
      manager.appendRows(_rowsFor(_tracks, next));
      _applyMidiFocus(manager, ref.read(focusedTrackRowIndexProvider));
    });
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
          manager.appendRows(_rowsFor(tracks, analyzingIds));
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
                                rows: _rowsFor(tracks, analyzingIds),
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
        width: 44,
        minWidth: 44,
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
          final analyzing = ref
              .read(analyzingTrackIdsProvider)
              .contains(track.id);
          final inLibrary = ctx.row.cells['inLibrary']?.value == true;
          final title = trackTitleLabel(track);
          return Center(
            child: TrackActionsMenu(
              trackId: track.id,
              path: track.path,
              title: title,
              inLibrary: inLibrary,
              analyzing: analyzing,
              enableSecondaryPress: false,
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

  List<TrinaRow<dynamic>> _rowsFor(
    List<LibraryTrackSummary> tracks,
    Set<String> analyzingIds,
  ) {
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
            'title': TrinaCell(
              value: analyzingIds.contains(t.id)
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
    final analyzing = ref.read(analyzingTrackIdsProvider).contains(track.id);
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
        analyzing: analyzing,
        // Watch engine + dim here so drag attaches after start without
        // remounting TrinaGrid (ValueKey no longer includes engineRunning).
        child: Consumer(
          builder: (context, ref, child) {
            final dimmed = ref.watch(
              sessionTrackDimmedProvider((track.id, track.path)),
            );
            final row = child ?? const SizedBox.shrink();
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

class TrackActionsMenu extends ConsumerWidget {
  const new({
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
    return MixarMenuAnchor(
      enabled: !analyzing,
      menuBuilder: (context, controller) => _trackActionsMenuBody(
        context: context,
        ref: ref,
        dismiss: controller.hide,
        trackId: trackId,
        path: path,
        title: title,
        inLibrary: inLibrary,
        analyzing: analyzing,
        engineRunning: engineRunning,
      ),
      childBuilder: (context, controller) => AppButton.icon(
        variant: .ghost,
        size: .xs,
        semanticsLabel: 'Track actions',
        onPress: analyzing ? null : controller.toggle,
        onSecondaryPress: analyzing || !enableSecondaryPress
            ? null
            : controller.toggle,
        child: analyzing
            ? const MLoader()
            : const Icon(LucideIcons.ellipsisVertical),
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
    return MixarContextMenu(
      enabled: !analyzing,
      menuBuilder: (context, handle) => _trackActionsMenuBody(
        context: context,
        ref: ref,
        dismiss: handle.hide,
        trackId: trackId,
        path: path,
        title: title,
        inLibrary: inLibrary,
        analyzing: analyzing,
        engineRunning: engineRunning,
      ),
      childBuilder: (context, handle) => GestureDetector(
        onSecondaryTapDown: analyzing
            ? null
            : (details) => handle.showAt(details.globalPosition),
        onLongPressStart: analyzing
            ? null
            : (details) => handle.showAt(details.globalPosition),
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
