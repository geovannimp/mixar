import 'dart:async';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/library/collection_actions.dart';
import 'package:gui_flutter/library/create_collection_dialog.dart';
import 'package:gui_flutter/library/history_providers.dart';
import 'package:gui_flutter/library/library_list_chrome.dart';
import 'package:gui_flutter/library/providers.dart';
import 'package:gui_flutter/library/track_list.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/key_format.dart';
import 'package:gui_flutter/mixer/track_drag.dart';
import 'package:gui_flutter/settings/settings_defaults.dart';
import 'package:gui_flutter/settings/settings_providers.dart';
import 'package:gui_flutter/shell/app_button.dart';
import 'package:gui_flutter/shell/mixar_dialog.dart';
import 'package:gui_flutter/shell/mixar_input.dart';
import 'package:gui_flutter/shell/mixar_menu.dart';
import 'package:gui_flutter/shell/mixar_overlay_controller.dart';
import 'package:gui_flutter/shell/mixar_popover.dart';
import 'package:gui_flutter/shell/mixar_theme.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Width of the leading `#` column.
const _kHistoryIndexWidth = 26.0;

/// Width of the deck badge column.
const _kHistoryDeckWidth = 28.0;

/// Leading width of both densities: `#`, deck badge, and the gutters around
/// them. Shared by the row layout and its trailing-meta budget.
const double _kHistoryLeadingWidth =
    _kHistoryIndexWidth + kRowGutter + _kHistoryDeckWidth + kRowGutter;

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
        Padding(
          padding: const EdgeInsets.fromLTRB(6, 8, 6, 8),
          child: Row(
            children: [
              Expanded(
                child: MixarInput(
                  hint: 'Filter entries…',
                  onChanged: (value) =>
                      ref.read(historyEntryFilterProvider.notifier).set(value),
                ),
              ),
              const SizedBox(width: 6),
              LibraryRowDensityButton(density: density),
              const SizedBox(width: 4),
              _HistorySessionActionsMenu(
                sessionId: sessionId,
                session: session,
              ),
            ],
          ),
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
                : _ComfortableEntryRow(
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

  static Future<void> _renameSession(
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
      ref.read(libraryMessageProvider.notifier).setError('$e');
    }
  }

  static Future<void> _exportSession(
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
      ref.read(libraryMessageProvider.notifier).setError('$e');
    }
  }
}

/// Leading `#` position, right-aligned so the numbers line up down the list.
class _IndexBadge extends StatelessWidget {
  const new({required this.index});

  final int index;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return SizedBox(
      width: _kHistoryIndexWidth,
      child: Text(
        '${index + 1}',
        textAlign: TextAlign.right,
        maxLines: 1,
        style: theme.typography.body.xs.copyWith(
          color: theme.colors.mutedForeground,
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
      ),
    );
  }
}

/// Deck A/B, tinted with the fader accent so it matches the deck panels.
class _DeckBadge extends StatelessWidget {
  const new({required this.deck});

  final int deck;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final accent = faderAccentForDeck(deck);
    final color = accent == null
        ? theme.colors.mutedForeground
        : FaderColors.forAccent(accent).grip;
    return Semantics(
      label: deckDisplayLabel(deck),
      container: true,
      // Announce "Deck B", not the bare letter plus the label again.
      excludeSemantics: true,
      child: SizedBox(
        width: _kHistoryDeckWidth,
        child: Text(
          _deckLetter(deck),
          textAlign: TextAlign.center,
          maxLines: 1,
          style: theme.typography.body.sm.copyWith(
            color: color,
            fontWeight: FontWeight.w700,
          ),
        ),
      ),
    );
  }
}

String _deckLetter(int deck) => switch (deck) {
  0 => 'A',
  1 => 'B',
  _ => '${deck + 1}',
};

/// Title + metadata pills on the left, BPM/key on the right.
class _ComfortableEntryRow extends StatelessWidget {
  const new({
    required this.index,
    required this.entry,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.actionsSlot,
  });

  final int index;
  final HistoryEntryInfo entry;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final Widget actionsSlot;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final title = historyEntryDisplayTitle(entry);
    final pills = <Widget>[
      ?_textPill(entry.artist),
      ?_textPill(entry.album),
      ?_textPill(fileNameFromPath(entry.location)),
      ?_textPill(entry.isrc),
      MetaPill(
        text: formatHistoryPlaySpan(entry.startedAt, entry.endedAt),
        leading: const MetaPillGlyph(LucideIcons.calendarClock),
      ),
      ?_lengthPill(entry.playedDurationMs),
    ].toList();

    return LayoutBuilder(
      builder: (context, constraints) {
        final metaWidth = trailingMetaWidth(
          constraints.maxWidth,
          _kHistoryLeadingWidth,
        );
        final detailsWidth =
            constraints.maxWidth -
            _kHistoryLeadingWidth -
            kRowGutter -
            metaWidth -
            kActionsColumnWidth;
        // At most three pills per run, so six pills never need a third run and
        // stay inside the fixed row height. A long label ellipsizes instead of
        // pushing the next pill past the clip.
        final maxPillWidth = (detailsWidth - 3 * kMetaPillGap) / 3;
        return Row(
          children: [
            _IndexBadge(index: index),
            const SizedBox(width: kRowGutter),
            _DeckBadge(deck: entry.deck),
            const SizedBox(width: kRowGutter),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.typography.body.sm.copyWith(
                      color: theme.colors.foreground,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  if (pills.isNotEmpty && maxPillWidth > 0)
                    Padding(
                      padding: const EdgeInsets.only(top: 3),
                      // Hung left by the pill's own inset so the metadata text
                      // (e.g. the file name) lines up with the title above it.
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
                bpm: entry.bpm,
                rawKey: entry.key ?? '',
                keyDisplayMode: keyDisplayMode,
                keyColorMode: keyColorMode,
              ),
            ),
            actionsSlot,
          ],
        );
      },
    );
  }

  Widget? _textPill(String? value) {
    final text = value?.trim() ?? '';
    return text.isEmpty ? null : MetaPill(text: text);
  }

  /// Played length. Unknown (`—`) is omitted rather than shown as a dash pill,
  /// matching the track list, which drops an unknown duration entirely. Carries
  /// the same clock glyph as the library's duration pill so the chips match.
  Widget? _lengthPill(int? playedDurationMs) {
    final text = formatPlayedDurationMs(playedDurationMs);
    return text == '—'
        ? null
        : MetaPill(text: text, leading: const MetaPillGlyph(LucideIcons.clock));
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
  });

  final int index;
  final HistoryEntryInfo entry;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final Widget actionsSlot;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return LayoutBuilder(
      builder: (context, constraints) => Row(
        children: [
          _IndexBadge(index: index),
          const SizedBox(width: kRowGutter),
          _DeckBadge(deck: entry.deck),
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
          SizedBox(
            width: trailingMetaWidth(
              constraints.maxWidth,
              _kHistoryLeadingWidth,
              // Compact hides the pills, so the trailing group carries the
              // played length too and must reserve room for it.
              natural:
                  kDurationSlotWidth + kMetaPillGap + kTrailingMetaBaseWidth,
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

class _HistorySessionActionsMenu extends ConsumerWidget {
  const new({required this.sessionId, required this.session});

  final String sessionId;
  final HistorySessionSummary? session;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MixarMenuAnchor(
      menuBuilder: (context, controller) => MixarMenuBody(
        groups: [
          MixarMenuGroup(
            children: [
              MixarMenuItem(
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
              MixarMenuItem(
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
              MixarMenuItem(
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
          MixarMenuGroup(
            children: [
              MixarMenuItem(
                title: const Text('Delete'),
                destructive: true,
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
      ),
      childBuilder: (context, controller) => AppButton.icon(
        variant: .ghost,
        semanticsLabel: 'Session actions',
        onPress: controller.toggle,
        child: const Icon(LucideIcons.ellipsisVertical),
      ),
    );
  }
}

Future<void> _afterHistoryMenu(
  BuildContext context,
  MixarOverlayController controller,
  Future<void> Function() action,
) async {
  controller.hide();
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
