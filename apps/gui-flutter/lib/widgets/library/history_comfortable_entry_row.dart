import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/track_drag.dart';
import 'package:gui_flutter/providers/library/history_providers.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/history_deck_badge.dart';
import 'package:gui_flutter/widgets/library/history_detail_pane.dart';
import 'package:gui_flutter/widgets/library/history_index_badge.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/library/meta_pill_glyph.dart';
import 'package:gui_flutter/widgets/library/trailing_meta.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';
import 'package:gui_flutter/widgets/library/history_row_layout.dart';

/// Title + metadata pills on the left, BPM/key on the right.
class HistoryComfortableEntryRow extends StatelessWidget {
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
          kHistoryLeadingWidth,
        );
        final detailsWidth =
            constraints.maxWidth -
            kHistoryLeadingWidth -
            kRowGutter -
            metaWidth -
            kActionsColumnWidth;
        // At most three pills per run, so six pills never need a third run and
        // stay inside the fixed row height. A long label ellipsizes instead of
        // pushing the next pill past the clip.
        final maxPillWidth = (detailsWidth - 3 * kMetaPillGap) / 3;
        return Row(
          children: [
            IndexBadge(index: index),
            const SizedBox(width: kRowGutter),
            DeckBadge(deck: entry.deck),
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
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: metaWidth),
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
