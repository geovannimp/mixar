import 'dart:async';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/mixar_button_style.dart';
import 'package:gui_flutter/core/utils/collection_actions.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/settings_defaults.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/providers/library/providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/providers/shell/app_notice.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/create_collection_dialog.dart';
import 'package:gui_flutter/widgets/library/history_comfortable_entry_row.dart';
import 'package:gui_flutter/widgets/library/history_session_actions_menu.dart';
import 'package:gui_flutter/widgets/library/library_list_message.dart';
import 'package:gui_flutter/widgets/library/library_pane_toolbar.dart';
import 'package:gui_flutter/widgets/library/library_row_density_button.dart';
import 'package:gui_flutter/widgets/library/track_list_view.dart';
import 'package:gui_flutter/widgets/library/trailing_meta.dart';
import 'package:gui_flutter/widgets/shell/app_button.dart';
import 'package:gui_flutter/widgets/shell/mixar_dialog.dart';
import 'package:gui_flutter/widgets/shell/mixar_input.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:gui_flutter/widgets/library/history_deck_badge.dart';
import 'package:gui_flutter/widgets/library/history_index_badge.dart';
import 'package:gui_flutter/widgets/library/history_row_layout.dart';

/// Width of the leading `#` column.
/// Session detail: entry list + session actions.
class HistoryDetailPane extends ConsumerWidget {
  const new({super.key});

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
    final density = ref.watch(libraryRowDensityProvider);
    final settings = ref
        .watch(appSettingsProvider)
        .maybeWhen(data: (s) => s, orElse: defaultAppSettings);
    final keyDisplayMode = keyModeFromSettings(settings.keyDisplayMode);
    final keyColorMode = keyColorModeFromSettings(settings.keyColorMode);

    if (sessionId == null) {
      return LibraryListMessage(
        'Select a history session',
        color: theme.colors.mutedForeground,
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        LibraryPaneToolbar(
          hint: 'Filter entries…',
          onChanged: (value) =>
              ref.read(historyEntryFilterProvider.notifier).set(value),
          trailing: [
            LibraryRowDensityButton(density: density),
            const SizedBox(width: 4),
            HistorySessionActionsMenu(sessionId: sessionId, session: session),
          ],
        ),
        Expanded(
          child: TrackListView<HistoryEntryInfo>(
            items: entries,
            // Keep the previous entries on screen while the session reloads,
            // instead of flashing the loader (the history list's behaviour).
            skipLoadingOnReload: true,
            idOf: (entry) => entry.id,
            payloadOf: (ref, entry) => payloadFromHistoryEntry(entry),
            rowBuilder: (context, ref, index, entry, density, slot) =>
                density.isCompact
                ? _CompactEntryRow(
                    index: index,
                    entry: entry,
                    keyDisplayMode: keyDisplayMode,
                    keyColorMode: keyColorMode,
                    actionsSlot: slot,
                  )
                : HistoryComfortableEntryRow(
                    index: index,
                    entry: entry,
                    keyDisplayMode: keyDisplayMode,
                    keyColorMode: keyColorMode,
                    actionsSlot: slot,
                  ),
            emptyBuilder: (context) => LibraryListMessage(
              (allEntries != null && allEntries.isEmpty)
                  ? 'No plays logged in this session'
                  : 'No matching entries',
              color: theme.colors.mutedForeground,
            ),
            errorBuilder: (context, e) =>
                LibraryListMessage('$e', color: theme.colors.destructive),
          ),
        ),
      ],
    );
  }

  static Future<void> renameSession(
    BuildContext context,
    WidgetRef ref,
    HistorySessionSummary session,
  ) async {
    var title = session.title;
    final next = await showMixarDialog<String?>(
      context: context,
      builder: (context) {
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
              MixarInput(initialValue: title, onChanged: (v) => title = v),
              const SizedBox(height: 16),
              Row(
                spacing: 8,
                children: [
                  AppButton(
                    variant: .outline,
                    onPress: () => Navigator.of(context).pop(),
                    child: const Text('Cancel'),
                  ),
                  AppButton(
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
    if (next == null || next.isEmpty || !context.mounted) {
      return;
    }
    try {
      final transport = await ref.read(libraryTransportProvider.future);
      await transport.renameHistorySession(sessionId: session.id, title: next);
      invalidateHistory(ref);
    } catch (e) {
      ref.read(appNoticeProvider.notifier).setError('$e');
    }
  }

  static Future<void> exportSession(
    BuildContext context,
    WidgetRef ref,
    String sessionId, {
    String? sessionTitle,
  }) async {
    final format = await showMixarDialog<HistoryExportFormatSetting?>(
      context: context,
      builder: (context) {
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
                  child: AppButton(
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
    if (format == null || !context.mounted) {
      return;
    }
    final ext = switch (format) {
      HistoryExportFormatSetting.csv => 'csv',
      HistoryExportFormatSetting.m3U8 => 'm3u8',
      HistoryExportFormatSetting.txt => 'txt',
    };
    final dest = await FilePicker.saveFile(
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
      ref.read(appNoticeProvider.notifier).setError('$e');
    }
  }

  static Future<void> createCollectionFromHistory(
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

  static Future<void> deleteSession(
    BuildContext context,
    WidgetRef ref,
    String sessionId,
  ) async {
    final confirmed = await showMixarConfirm<bool>(
      context: context,
      title: 'Delete this session?',
      body: 'Removes the XSPF file and index row.',
      actions: const [
        MixarDialogAction(
          label: 'Cancel',
          value: false,
          variant: MixarButtonVariant.outline,
        ),
        MixarDialogAction(
          label: 'Delete',
          value: true,
          variant: MixarButtonVariant.destructive,
        ),
      ],
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
      ref.read(appNoticeProvider.notifier).setError('$e');
    }
  }
}

/// Single dense line: `#` deck title … length BPM key.
class _CompactEntryRow extends StatelessWidget {
  const new({
    required this.index,
    required this.entry,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.actionsSlot,
    super.key,
  });

  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final Widget actionsSlot;
  final int index;
  final HistoryEntryInfo entry;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) => Row(
        children: [
          IndexBadge(index: index),
          const SizedBox(width: kRowGutter),
          DeckBadge(deck: entry.deck),
          const SizedBox(width: kRowGutter),
          Expanded(
            child: Text(
              historyEntryDisplayTitle(entry),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.typography.body.sm.copyWith(
                color: theme.colors.foreground,
              ),
            ),
          ),
          const SizedBox(width: kRowGutter),
          ConstrainedBox(
            constraints: BoxConstraints(
              maxWidth: trailingMetaWidth(
                constraints.maxWidth,
                kHistoryLeadingWidth,
                // Compact hides the pills, so the trailing group carries the
                // played length too and must reserve room for it.
                natural:
                    kDurationSlotWidth + kMetaPillGap + kTrailingMetaBaseWidth,
              ),
            ),
            child: TrailingMeta(
              bpm: entry.bpm,
              rawKey: entry.key ?? '',
              keyDisplayMode: keyDisplayMode,
              keyColorMode: keyColorMode,
              durationMs: entry.playedDurationMs,
            ),
          ),
          actionsSlot,
        ],
      ),
    );
  }
}

String _historyExportFileName(String? sessionTitle, String ext) {
  final raw = sessionTitle?.trim();
  final base = (raw != null && raw.isNotEmpty) ? raw : 'history-session';
  final safe = base.replaceAll(RegExp(r'[<>:"/\\|?*]'), '-');
  return '$safe.$ext';
}

/// `start → end` for a play, dropping the duplicated date when the play stayed
/// on one local day. Entries crossing midnight keep both full timestamps, and
/// an open entry ends at `live`.
String formatHistoryPlaySpan(String startedAt, String? endedAt) {
  final start = DateTime.tryParse(startedAt)?.toLocal();
  final end = endedAt == null ? null : DateTime.tryParse(endedAt)?.toLocal();
  final sameDay =
      start != null &&
      end != null &&
      start.year == end.year &&
      start.month == end.month &&
      start.day == end.day;
  final startText = sameDay
      ? _clockLabel(start)
      : formatHistoryTimestamp(startedAt);
  final endText = endedAt == null
      ? 'live'
      : sameDay
      ? _clockLabel(end)
      : formatHistoryTimestamp(endedAt);
  return '$startText → $endText';
}

String _clockLabel(DateTime time) {
  final h = time.hour.toString().padLeft(2, '0');
  final m = time.minute.toString().padLeft(2, '0');
  return '$h:$m';
}
