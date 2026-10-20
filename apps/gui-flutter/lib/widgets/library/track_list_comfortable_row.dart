import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/library/track_artwork_thumb.dart';
import 'package:gui_flutter/widgets/library/track_status_pills.dart';
import 'package:gui_flutter/widgets/library/track_title_line.dart';
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
    final album = track.album ?? '';
    final genre = track.genre ?? '';
    final duration = formatTrackDuration(track.durationMs);
    final artist = track.artist ?? '';
    Widget? albumPill() => album.isNotEmpty ? MetaPill(text: album) : null;
    Widget? genrePill() => genre.isNotEmpty ? MetaPill(text: genre) : null;
    Widget? durationPill({required bool glyph}) => duration.isNotEmpty
        ? MetaPill(
            text: duration,
            leading: glyph
                ? Icon(
                    LucideIcons.clock,
                    size: 11,
                    color: theme.colors.mutedForeground,
                  )
                : null,
          )
        : null;

    return LayoutBuilder(
      builder: (context, constraints) {
        // A pill is bounded to one run and the group to two (the fit test in
        // the details column), so the metadata can never paint a third run
        // past the fixed `itemExtent`, where it would be silently clipped.
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
                  final available = columnConstraints.maxWidth;
                  // A pill never paints wider than one run. The fit test below
                  // is what keeps the group to two runs, so there is no
                  // half-column cap that would ellipsize a pill with room.
                  final maxPillWidth = available;
                  // Album and genre are extras: they stay only when the whole
                  // group paints unclipped inside the two runs the row can
                  // show, and go together when it does not. Width decides, not
                  // the layout mode — a narrow drawer with room keeps them.
                  final pillStyle = theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w500,
                  );
                  double naturalWidth(String text, {bool glyph = false}) =>
                      measureTextWidth(context, text, pillStyle) +
                      kMetaPillPadding.horizontal +
                      (glyph ? kMetaPillLeadingExtent : 0);
                  // The clock is decoration: it goes before the duration text
                  // loses a character to it.
                  final durationGlyph =
                      duration.isNotEmpty &&
                      naturalWidth(duration, glyph: true) <= available;
                  final widths = [
                    if (album.isNotEmpty) naturalWidth(album),
                    if (genre.isNotEmpty) naturalWidth(genre),
                    if (duration.isNotEmpty)
                      naturalWidth(duration, glyph: durationGlyph),
                  ];
                  final extrasFit =
                      widths.every((width) => width <= available) &&
                      fitsInRuns(widths, available: available, runs: 2);
                  final pills = <Widget?>[
                    if (extrasFit) albumPill(),
                    if (extrasFit) genrePill(),
                    durationPill(glyph: durationGlyph),
                  ].whereType<Widget>().toList();
                  return Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      // Title and artist share one baseline so the smaller
                      // artist does not float off the title's baseline.
                      TrackTitleLine(
                        title: title,
                        artist: artist,
                        titleStyle: titleStyle,
                      ),
                      // Wrap, not one ellipsized line, so two mid-length
                      // extras share the width instead of both truncating.
                      // Only what the fit test above kept is built, and that
                      // test already packed it into two runs; the per-pill cap
                      // keeps one pill from spanning both, and the clip is
                      // belt-and-braces so no label paints past the row edge.
                      // Skipped when there is no room left: a zero-width cap
                      // renders the pills as an empty gap.
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
