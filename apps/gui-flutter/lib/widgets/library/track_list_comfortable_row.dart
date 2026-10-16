import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/library/track_artwork_thumb.dart';
import 'package:gui_flutter/widgets/library/track_status_pills.dart';
import 'package:gui_flutter/widgets/library/trailing_meta.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

class TrackListComfortableRow extends StatelessWidget {
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
    final titleStyle = theme.typography.body.sm.copyWith(
      color: theme.colors.foreground,
      fontWeight: FontWeight.w600,
    );
    // The metadata the old column table showed, less BPM and key, which are
    // aligned in the trailing meta instead. Each renders as its own pill.
    final duration = formatTrackDuration(track.durationMs);
    Widget? artistPill() =>
        (track.artist ?? '').isNotEmpty ? MetaPill(text: track.artist!) : null;
    Widget? albumPill() =>
        (track.album ?? '').isNotEmpty ? MetaPill(text: track.album!) : null;
    Widget? genrePill() =>
        (track.genre ?? '').isNotEmpty ? MetaPill(text: track.genre!) : null;
    Widget? durationPill() => duration.isNotEmpty
        ? MetaPill(
            text: duration,
            leading: Icon(
              LucideIcons.clock,
              size: 11,
              color: theme.colors.mutedForeground,
            ),
          )
        : null;
    final pills = <Widget?>[
      artistPill(),
      albumPill(),
      genrePill(),
      durationPill(),
    ].whereType<Widget>().toList();

    return LayoutBuilder(
      builder: (context, constraints) {
        // Bound each pill to half the details column, so at most two share a
        // run and four pills can never need a third one. Without this a long
        // artist name consumes a whole run and the row grows past the fixed
        // `itemExtent`, where the extra runs are silently clipped.
        final metaWidth = trailingMetaWidth(
          constraints.maxWidth,
          artSize + kArtworkGap,
        );
        // Title column width excludes status + trailing meta; status sits
        // immediately left of TrailingMeta so it does not float mid-row.
        final detailsWidth =
            constraints.maxWidth -
            artSize -
            kArtworkGap -
            metaWidth -
            kRowGutter -
            kActionsColumnWidth;
        final statusMax = (detailsWidth - kMinTitleWidth).clamp(
          0.0,
          double.infinity,
        );
        return Row(
          children: [
            TrackArtworkThumb(trackId: track.id, size: artSize),
            const SizedBox(width: kArtworkGap),
            Expanded(
              // Measure the real title column: in-flow status pills shrink this
              // Expanded, so a status-free detailsWidth cap would leave
              // maxPillWidth too wide and force a third Wrap run past the
              // kMetaPillAreaHeight clip.
              child: LayoutBuilder(
                builder: (context, columnConstraints) {
                  final maxPillWidth =
                      (columnConstraints.maxWidth - kMetaPillGap) / 2;
                  return Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: titleStyle,
                      ),
                      // Wrap, not a single ellipsized line: a long artist name
                      // pushes the remaining pills onto a second line instead of
                      // hiding them. Two runs is the cap (see `maxPillWidth`);
                      // the clip is belt-and-braces so no label can paint
                      // outside the row. Skipped entirely when there is no room
                      // left: a zero-width cap would render the pills as an
                      // empty gap.
                      if (pills.isNotEmpty && maxPillWidth > 0)
                        Padding(
                          padding: const EdgeInsets.only(top: 3),
                          // maxHeight, not a fixed height. Reserving two runs
                          // when a row only needs one leaves dead space under
                          // the pills, which pushes the title + pills group off
                          // centre. The cap still bounds the area at the two
                          // runs the width cap above guarantees.
                          //
                          // Hung left by the pill's own inset so the metadata
                          // text lines up with the title above.
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
                  );
                },
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
              ),
            ),
            const SizedBox(width: kRowGutter),
            actionsSlot,
          ],
        );
      },
    );
  }
}
