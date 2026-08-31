import 'dart:async';
import 'dart:ui' show FontFeature;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/library/collection_actions.dart';
import 'package:gui_flutter/library/create_collection_dialog.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:trina_grid/trina_grid.dart';

/// Session detail: entry table + session actions.
class HistoryDetailPane extends ConsumerWidget {
  const HistoryDetailPane({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final sessionId = ref.watch(activeHistorySessionIdProvider);
    final sessions =
        ref.watch(historySessionsProvider).asData?.value ?? const [];
    HistorySessionSummary? session;
    for (final row in sessions) {
      if (row.id == sessionId) {
        session = row;
        break;
      }
    }
    final entries = ref.watch(filteredHistoryEntriesProvider);
    final allEntries = ref.watch(historyEntriesProvider).asData?.value;

    if (sessionId == null) {
      return Center(
        child: Text(
          'Select a history session',
          style: theme.typography.body.sm.copyWith(
            color: theme.colors.mutedForeground,
          ),
        ),
      );
    }

    return Padding(
      padding: const EdgeInsets.fromLTRB(6, 0, 0, 0),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.only(right: 6, bottom: 8),
            child: Row(
              spacing: 4,
              children: [
                Expanded(
                  child: FTextField(
                    hint: 'Filter entries…',
                    control: FTextFieldManagedControl(
                      onChange: (value) => ref
                          .read(historyEntryFilterProvider.notifier)
                          .set(value.text),
                    ),
                  ),
                ),
                _HistorySessionActionsMenu(
                  sessionId: sessionId,
                  session: session,
                ),
              ],
            ),
          ),
          Expanded(
            child: entries.when(
              skipLoadingOnReload: true,
              loading: () => const Center(child: FCircularProgress()),
              error: (e, _) => Center(child: Text('$e')),
              data: (rows) {
                if (allEntries != null && allEntries.isEmpty) {
                  return Center(
                    child: Text(
                      'No plays logged in this session',
                      style: theme.typography.body.sm.copyWith(
                        color: theme.colors.mutedForeground,
                      ),
                    ),
                  );
                }
                if (rows.isEmpty) {
                  return Center(
                    child: Text(
                      'No matching entries',
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
                      key: ValueKey(sessionId),
                      columns: _historyColumns(theme),
                      rows: _historyRows(rows),
                      mode: TrinaGridMode.readOnly,
                      configuration: _historyGridConfig(theme),
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

  static Future<void> _renameSession(
    BuildContext context,
    WidgetRef ref,
    HistorySessionSummary session,
  ) async {
    var title = session.title;
    final next = await showFDialog<String?>(
      context: context,
      builder: (context, _, animation) {
        return FDialog(
          animation: animation,
          builder: (context, _) {
            final theme = context.theme;
            return Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text(
                    'Rename session',
                    style: theme.typography.body.md.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 12),
                  FTextField(
                    control: .managed(
                      initial: TextEditingValue(text: title),
                      onChange: (v) => title = v.text,
                    ),
                  ),
                  const SizedBox(height: 16),
                  Row(
                    spacing: 8,
                    children: [
                      FButton(
                        variant: .outline,
                        onPress: () => Navigator.of(context).pop(),
                        child: const Text('Cancel'),
                      ),
                      FButton(
                        onPress: () => Navigator.of(context).pop(title.trim()),
                        child: const Text('Save'),
                      ),
                    ],
                  ),
                ],
              ),
            );
          },
        );
      },
    );
    if (next == null || next.isEmpty || !context.mounted) {
      return;
    }
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await transport.renameHistorySession(sessionId: session.id, title: next);
      invalidateHistory(ref);
    } catch (e) {
      ref.read(libraryMessageProvider.notifier).setError('$e');
    }
  }

  static Future<void> _exportSession(
    BuildContext context,
    WidgetRef ref,
    String sessionId, {
    String? sessionTitle,
  }) async {
    final format = await showFDialog<HistoryExportFormatSetting?>(
      context: context,
      builder: (context, _, animation) {
        return FDialog(
          animation: animation,
          builder: (context, _) {
            return Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text('Export format'),
                  const SizedBox(height: 12),
                  for (final (label, value) in [
                    ('CSV', HistoryExportFormatSetting.csv),
                    ('M3U8', HistoryExportFormatSetting.m3U8),
                    ('Plain text', HistoryExportFormatSetting.txt),
                  ])
                    Padding(
                      padding: const EdgeInsets.only(bottom: 6),
                      child: FButton(
                        variant: .outline,
                        onPress: () => Navigator.of(context).pop(value),
                        child: Text(label),
                      ),
                    ),
                ],
              ),
            );
          },
        );
      },
    );
    if (format == null || !context.mounted) {
      return;
    }
    final ext = switch (format) {
      HistoryExportFormatSetting.csv => 'csv',
      HistoryExportFormatSetting.m3U8 => 'm3u8',
      HistoryExportFormatSetting.txt => 'txt',
    };
    final dest = await FilePicker.platform.saveFile(
      dialogTitle: 'Export history session',
      fileName: _historyExportFileName(sessionTitle, ext),
    );
    if (dest == null) {
      return;
    }
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await transport.exportHistorySession(
        sessionId: sessionId,
        format: format,
        destPath: dest,
      );
    } catch (e) {
      ref.read(libraryMessageProvider.notifier).setError('$e');
    }
  }

  static Future<void> _createCollectionFromHistory(
    BuildContext context,
    WidgetRef ref,
    String sessionId,
    String? sessionTitle,
  ) async {
    final result = await showCreateCollectionDialog(
      context,
      input: CreateCollectionInput(
        initialName: sessionTitle ?? 'History session',
        initialType: CreateCollectionType.playlist,
        initialSortable: true,
        historySessionId: sessionId,
      ),
    );
    if (result == null || !context.mounted) {
      return;
    }
    final collection = await createCollection(
      ref,
      result,
      historySessionId: sessionId,
    );
    if (collection != null) {
      selectCreatedCollection(ref, collection);
    }
  }

  static Future<void> _deleteSession(
    BuildContext context,
    WidgetRef ref,
    String sessionId,
  ) async {
    final confirmed = await showFDialog<bool>(
      context: context,
      builder: (context, _, animation) {
        return FDialog(
          animation: animation,
          builder: (context, _) {
            return Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text('Delete this session?'),
                  const SizedBox(height: 8),
                  const Text('Removes the XSPF file and index row.'),
                  const SizedBox(height: 16),
                  Row(
                    spacing: 8,
                    children: [
                      FButton(
                        variant: .outline,
                        onPress: () => Navigator.of(context).pop(false),
                        child: const Text('Cancel'),
                      ),
                      FButton(
                        variant: .destructive,
                        onPress: () => Navigator.of(context).pop(true),
                        child: const Text('Delete'),
                      ),
                    ],
                  ),
                ],
              ),
            );
          },
        );
      },
    );
    if (confirmed != true || !context.mounted) {
      return;
    }
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await transport.deleteHistorySession(sessionId: sessionId);
      ref.read(selectedHistorySessionIdProvider.notifier).set(null);
      invalidateHistory(ref);
    } catch (e) {
      ref.read(libraryMessageProvider.notifier).setError('$e');
    }
  }
}

class _HistorySessionActionsMenu extends ConsumerWidget {
  const _HistorySessionActionsMenu({
    required this.sessionId,
    required this.session,
  });

  final String sessionId;
  final HistorySessionSummary? session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    return FPopoverMenu(
      faded: null,
      overlayLocation: OverlayChildLocation.rootOverlay,
      menuBuilder: (_, controller, __) => [
        .group(
          children: [
            .item(
              title: const Text('Rename'),
              enabled: session != null,
              onPress: session == null
                  ? null
                  : () {
                      unawaited(
                        _afterHistoryMenu(context, controller, () {
                          return HistoryDetailPane._renameSession(
                            context,
                            ref,
                            session!,
                          );
                        }),
                      );
                    },
            ),
            .item(
              title: const Text('Export'),
              onPress: () {
                unawaited(
                  _afterHistoryMenu(context, controller, () {
                    return HistoryDetailPane._exportSession(
                      context,
                      ref,
                      sessionId,
                      sessionTitle: session?.title,
                    );
                  }),
                );
              },
            ),
            .item(
              title: const Text('Create collection'),
              onPress: () {
                unawaited(
                  _afterHistoryMenu(context, controller, () {
                    return HistoryDetailPane._createCollectionFromHistory(
                      context,
                      ref,
                      sessionId,
                      session?.title,
                    );
                  }),
                );
              },
            ),
          ],
        ),
        .group(
          children: [
            .item(
              title: Text(
                'Delete',
                style: theme.typography.body.sm.copyWith(
                  color: theme.colors.destructive,
                ),
              ),
              onPress: () {
                unawaited(
                  _afterHistoryMenu(context, controller, () {
                    return HistoryDetailPane._deleteSession(
                      context,
                      ref,
                      sessionId,
                    );
                  }),
                );
              },
            ),
          ],
        ),
      ],
      builder: (context, controller, child) => FButton.icon(
        variant: .ghost,
        semanticsLabel: 'Session actions',
        onPress: controller.toggle,
        child: child!,
      ),
      child: const Icon(FLucideIcons.ellipsisVertical),
    );
  }
}

Future<void> _afterHistoryMenu(
  BuildContext context,
  FPopoverController controller,
  Future<void> Function() action,
) async {
  await controller.hide();
  if (!context.mounted) {
    return;
  }
  await action();
}

String _historyExportFileName(String? sessionTitle, String ext) {
  final raw = sessionTitle?.trim();
  final base = (raw != null && raw.isNotEmpty) ? raw : 'history-session';
  final safe = base.replaceAll(RegExp(r'[<>:"/\\|?*]'), '-');
  return '$safe.$ext';
}

List<TrinaColumn> _historyColumns(FThemeData theme) {
  return [
    TrinaColumn(
      title: '#',
      field: 'position',
      type: TrinaColumnType.text(),
      width: 44,
      minWidth: 44,
      enableContextMenu: false,
      enableDropToResize: false,
      enableSorting: false,
      textAlign: TrinaColumnTextAlign.center,
      renderer: (ctx) {
        final value = ctx.cell.value as String? ?? '';
        return Center(
          child: Text(
            value,
            style: theme.typography.body.sm.copyWith(
              color: theme.colors.mutedForeground,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        );
      },
    ),
    TrinaColumn(
      title: 'Deck',
      field: 'deck',
      type: TrinaColumnType.text(),
      width: 72,
      minWidth: 56,
      enableContextMenu: false,
      enableDropToResize: true,
      textAlign: TrinaColumnTextAlign.center,
      renderer: (ctx) {
        final deckId = ctx.cell.value as int? ?? 0;
        final accent = faderAccentForDeck(deckId);
        final color = accent == null
            ? theme.colors.mutedForeground
            : FaderColors.forAccent(accent).grip;
        return Center(
          child: Text(
            deckDisplayLabel(deckId),
            style: theme.typography.body.sm.copyWith(
              color: color,
              fontWeight: FontWeight.w600,
            ),
          ),
        );
      },
    ),
    TrinaColumn(
      title: 'Title',
      field: 'title',
      type: TrinaColumnType.text(),
      width: 240,
      minWidth: 120,
      enableContextMenu: false,
      enableDropToResize: true,
    ),
    TrinaColumn(
      title: 'Artist',
      field: 'artist',
      type: TrinaColumnType.text(),
      width: 160,
      minWidth: 96,
      enableContextMenu: false,
      enableDropToResize: true,
    ),
    TrinaColumn(
      title: 'File',
      field: 'file',
      type: TrinaColumnType.text(),
      width: 200,
      minWidth: 120,
      enableContextMenu: false,
      enableDropToResize: true,
    ),
    TrinaColumn(
      title: 'Start',
      field: 'started',
      type: TrinaColumnType.text(),
      width: 140,
      minWidth: 112,
      enableContextMenu: false,
      enableDropToResize: true,
    ),
    TrinaColumn(
      title: 'End',
      field: 'ended',
      type: TrinaColumnType.text(),
      width: 140,
      minWidth: 112,
      enableContextMenu: false,
      enableDropToResize: true,
    ),
    TrinaColumn(
      title: 'Length',
      field: 'length',
      type: TrinaColumnType.text(),
      width: 72,
      minWidth: 56,
      enableContextMenu: false,
      enableDropToResize: true,
      textAlign: TrinaColumnTextAlign.right,
    ),
    TrinaColumn(
      title: 'BPM',
      field: 'bpm',
      type: TrinaColumnType.text(),
      width: 56,
      minWidth: 48,
      enableContextMenu: false,
      enableDropToResize: true,
      textAlign: TrinaColumnTextAlign.right,
    ),
    TrinaColumn(
      title: 'Key',
      field: 'key',
      type: TrinaColumnType.text(),
      width: 48,
      minWidth: 40,
      enableContextMenu: false,
      enableDropToResize: true,
      textAlign: TrinaColumnTextAlign.right,
    ),
    TrinaColumn(
      title: 'ISRC',
      field: 'isrc',
      type: TrinaColumnType.text(),
      width: 112,
      minWidth: 80,
      enableContextMenu: false,
      enableDropToResize: true,
      textAlign: TrinaColumnTextAlign.right,
    ),
  ];
}

List<TrinaRow> _historyRows(List<HistoryEntryInfo> entries) {
  return [
    for (var i = 0; i < entries.length; i++)
      TrinaRow(
        cells: {
          'position': TrinaCell(value: '${i + 1}'),
          'deck': TrinaCell(value: entries[i].deck),
          'title': TrinaCell(value: historyEntryDisplayTitle(entries[i])),
          'artist': TrinaCell(value: entries[i].artist ?? ''),
          'file': TrinaCell(value: fileNameFromPath(entries[i].location)),
          'started': TrinaCell(
            value: formatHistoryTimestamp(entries[i].startedAt),
          ),
          'ended': TrinaCell(
            value: entries[i].endedAt == null
                ? '…'
                : formatHistoryTimestamp(entries[i].endedAt!),
          ),
          'length': TrinaCell(
            value: formatPlayedDurationMs(entries[i].playedDurationMs?.toInt()),
          ),
          'bpm': TrinaCell(value: entries[i].bpm?.toStringAsFixed(0) ?? '—'),
          'key': TrinaCell(value: entries[i].key ?? '—'),
          'isrc': TrinaCell(value: entries[i].isrc ?? '—'),
        },
      ),
  ];
}

TrinaGridConfiguration _historyGridConfig(FThemeData theme) {
  final surface = theme.colors.secondary;
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
    scrollbar: const TrinaGridScrollbarConfig(isAlwaysShown: false),
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
      oddRowColor: surface,
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
