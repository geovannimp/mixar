import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
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

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final selectedId = ref.watch(activeCollectionIdProvider);
    final tracksAsync = ref.watch(filteredTracksProvider);
    final config = _gridConfig(theme);

    ref.listen(filteredTracksProvider, (_, next) {
      final manager = _manager;
      if (manager == null) {
        return;
      }
      next.whenData((tracks) {
        manager.removeAllRows();
        if (tracks.isNotEmpty) {
          manager.appendRows(_rowsFor(tracks));
        }
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
                  ref.read(trackFilterProvider.notifier).state = value.text,
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
                            columns: _columns(),
                            rows: _rowsFor(tracks),
                            mode: TrinaGridMode.readOnly,
                            onLoaded: (e) {
                              _manager = e.stateManager;
                              e.stateManager.setShowColumnFilter(false);
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

  List<TrinaColumn> _columns() {
    return [
      TrinaColumn(
        title: 'Title',
        field: 'title',
        type: TrinaColumnType.text(),
        width: 320,
        minWidth: 120,
        enableContextMenu: false,
        enableDropToResize: true,
      ),
      TrinaColumn(
        title: 'Artist',
        field: 'artist',
        type: TrinaColumnType.text(),
        width: 200,
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
        // Fill the viewport on load, and keep total width when the user drags a
        // column edge (shrink one → siblings absorb the leftover).
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

  List<TrinaRow> _rowsFor(List<LibraryTrackSummary> tracks) {
    return [
      for (final t in tracks)
        TrinaRow(
          cells: {
            'title': TrinaCell(value: trackTitleLabel(t)),
            'artist': TrinaCell(value: t.artist ?? ''),
            'bpm': TrinaCell(
              value: t.bpm == null ? '' : t.bpm!.toStringAsFixed(1),
            ),
            'key': TrinaCell(value: t.key ?? ''),
            'length': TrinaCell(value: _formatDuration(t.durationMs)),
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
