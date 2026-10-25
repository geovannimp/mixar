import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/core/utils/library_row_fields.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/library/track_artwork_thumb.dart';
import 'package:gui_flutter/widgets/library/track_status_pills.dart';
import 'package:gui_flutter/widgets/library/track_title_line.dart';
import 'package:gui_flutter/widgets/library/trailing_meta.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// One metadata chip a comfortable row can paint under its title.
class _RowPill {
  const _RowPill({required this.text, this.glyph});

  final String text;

  /// Decoration only: the clock goes before the duration text loses a
  /// character to it.
  final IconData? glyph;
}

class TrackListComfortableRow extends StatelessWidget {
  const new({
    required this.track,
    required this.title,
    required this.keyDisplayMode,
    required this.keyColorMode,
    required this.artSize,
    required this.actionsSlot,
    required this.fields,
  });

  final LibraryTrackSummary track;
  final String title;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;
  final double artSize;
  final Widget actionsSlot;

  /// Visible metadata fields in saved order (Settings → UI → Library list).
  final List<LibraryRowFieldSetting> fields;

  /// The chip [field] paints, or null when the track has no value for it
  /// (unknown album, no length, …) or when the field lives in the trailing
  /// cluster instead of the pill group.
  _RowPill? _pillFor(LibraryRowFieldSetting field) {
    final (text, glyph) = switch (field) {
      LibraryRowFieldSetting.album => (track.album ?? '', null),
      LibraryRowFieldSetting.genre => (track.genre ?? '', null),
      LibraryRowFieldSetting.year => (
        track.year?.toString() ?? '',
        LucideIcons.calendar,
      ),
      LibraryRowFieldSetting.trackNumber => (
        track.trackNumber?.toString() ?? '',
        LucideIcons.hash,
      ),
      LibraryRowFieldSetting.discNumber => (
        track.discNumber?.toString() ?? '',
        LucideIcons.disc3,
      ),
      LibraryRowFieldSetting.comment => (
        track.comment ?? '',
        LucideIcons.messageSquareText,
      ),
      LibraryRowFieldSetting.isrc => (track.isrc ?? '', LucideIcons.barcode),
      LibraryRowFieldSetting.duration => (
        formatTrackDuration(track.durationMs),
        LucideIcons.clock,
      ),
      LibraryRowFieldSetting.rating ||
      LibraryRowFieldSetting.color ||
      LibraryRowFieldSetting.bpm ||
      LibraryRowFieldSetting.key => ('', null),
    };
    final label = text.trim();
    return label.isEmpty ? null : _RowPill(text: label, glyph: glyph);
  }

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final titleStyle = theme.typography.body.sm.copyWith(
      color: theme.colors.foreground,
      fontWeight: FontWeight.w600,
    );
    // Every selected field that has a value and belongs under the title. BPM,
    // key, rating and colour are aligned in the trailing meta instead.
    final pills = [for (final field in fields) ?_pillFor(field)];
    final artist = track.artist ?? '';
    final bpm = libraryRowFieldShown(fields, LibraryRowFieldSetting.bpm)
        ? track.bpm
        : null;
    final rawKey = libraryRowFieldShown(fields, LibraryRowFieldSetting.key)
        ? track.key ?? ''
        : '';
    final rating = libraryRowFieldShown(fields, LibraryRowFieldSetting.rating)
        ? track.rating
        : null;
    final colorLabel =
        libraryRowFieldShown(fields, LibraryRowFieldSetting.color)
        ? track.colorLabel
        : null;

    return LayoutBuilder(
      builder: (context, constraints) {
        // A pill is bounded to one run and the group to two (the fit test in
        // the details column), so the metadata can never paint a third run
        // past the fixed `itemExtent`, where it would be silently clipped.
        //
        // Trailing slots are reserved only for the fields the user shows, and
        // only when this track has a value for them, so hiding one gives its
        // width back to the title instead of leaving a gap before the actions.
        final metaWidth = trailingMetaWidth(
          constraints.maxWidth,
          artSize + kArtworkGap,
          natural: trailingMetaNaturalWidth(
            rating: trackShowsRating(rating),
            color: trackShowsColorLabel(colorLabel),
            bpm: bpm != null,
            key: rawKey.isNotEmpty,
          ),
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
                  // A pill never paints wider than one run.
                  final maxPillWidth = available;
                  final pillStyle = theme.typography.body.xs.copyWith(
                    fontWeight: FontWeight.w500,
                  );
                  double naturalWidth(_RowPill pill, {required bool glyph}) =>
                      measureTextWidth(context, pill.text, pillStyle) +
                      kMetaPillPadding.horizontal +
                      (glyph ? kMetaPillLeadingExtent : 0);
                  // Saved order is render order. A chip that cannot pack into
                  // the two runs the row can show is left out on its own, and
                  // the fields after it still get their turn — one long album
                  // title must not cost the row its length. Every kept chip
                  // paints whole; nothing is ellipsized.
                  final kept = <(_RowPill, bool)>[];
                  var widths = <double>[];
                  for (final pill in pills) {
                    final glyph =
                        pill.glyph != null &&
                        naturalWidth(pill, glyph: true) <= available;
                    final width = naturalWidth(pill, glyph: glyph);
                    if (width > available) {
                      continue;
                    }
                    final candidate = [...widths, width];
                    if (!fitsInRuns(candidate, available: available, runs: 2)) {
                      continue;
                    }
                    widths = candidate;
                    kept.add((pill, glyph));
                  }
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
                      if (kept.isNotEmpty && maxPillWidth > 0)
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
                                  for (final (pill, glyph) in kept)
                                    ConstrainedBox(
                                      constraints: BoxConstraints(
                                        maxWidth: maxPillWidth,
                                      ),
                                      child: MetaPill(
                                        text: pill.text,
                                        leading: glyph
                                            ? Icon(
                                                pill.glyph,
                                                size: 11,
                                                color: theme
                                                    .colors
                                                    .mutedForeground,
                                              )
                                            : null,
                                      ),
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
                bpm: bpm,
                rawKey: rawKey,
                keyDisplayMode: keyDisplayMode,
                keyColorMode: keyColorMode,
                rating: rating,
                colorLabel: colorLabel,
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
