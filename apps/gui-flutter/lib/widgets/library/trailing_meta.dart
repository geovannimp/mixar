import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/color_labels.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/src/rust/api/library.dart';
import 'package:gui_flutter/widgets/library/key_pill.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/library/meta_pill_glyph.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// Stable keys for the colour circle and the rating pill, so tests can scope
/// to them without guessing at generic [Container]/[MetaPill] widgets.
const kTrackColorPillKey = ValueKey<String>('trackColourPill');
const kTrackRatingPillKey = ValueKey<String>('trackRatingPill');

/// Rating, colour, BPM + key (and optional duration) for a list row's right
/// side.
///
/// Content-sized: parents pass a `maxWidth` budget via [trailingMetaWidth], and
/// the cluster only takes the width of the pills it keeps so it sits flush
/// against the row actions. Pills are laid out in priority order and hidden
/// once the budget cannot seat one, so a narrow pane drops the widest value
/// instead of shrinking every pill into an unreadable stub.
class TrailingMeta extends StatelessWidget {
  const new({
    required this.bpm,
    required this.rawKey,
    required this.keyDisplayMode,
    required this.keyColorMode,
    this.durationMs,
    this.rating,
    this.colorLabel,
    super.key,
  });

  final double? bpm;
  final String rawKey;
  final KeyDisplayMode keyDisplayMode;
  final KeyColorMode keyColorMode;

  /// Rendered as a leading pill when set. Compact rows have no metadata pills
  /// row, so the trailing group is the only place their length can appear;
  /// comfortable rows already carry the duration on the left and pass null to
  /// avoid showing it twice.
  final int? durationMs;

  /// Star rating 1–5; omitted from the group when unset.
  final int? rating;

  /// Canonical colour-label name; rendered as a filled circle when set.
  final LibraryColorLabel? colorLabel;

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    final duration = formatTrackDuration(durationMs);
    // Render order doubles as priority: the cheap, information-dense pills come
    // first, so a squeezed row keeps the length and rating and gives up the
    // BPM/key labels.
    // (slot width, minimum width, pill): the slot is the pill's widest useful
    // size, the minimum the narrowest it can paint without overflowing. The
    // colour circle is fixed, so its minimum is its own width.
    final candidates = <(double, double, Widget)>[
      if (duration.isNotEmpty)
        (
          kDurationSlotWidth,
          kTrailingPillMinWidth,
          MetaPill(
            text: duration,
            fillWidth: false,
            leading: const MetaPillGlyph(LucideIcons.clock),
          ),
        ),
      if (trackShowsColorLabel(colorLabel))
        if (colorLabelColor(colorLabel) case final color?)
          (
            kColorSlotWidth,
            kColorSlotWidth,
            Semantics(
              label: 'Colour ${colorLabelDisplayName(colorLabel)}',
              child: Container(
                key: kTrackColorPillKey,
                width: kColorSlotWidth,
                height: kColorSlotWidth,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: color,
                  border: Border.all(
                    color: theme.colors.border,
                    width: theme.style.borderWidth,
                  ),
                ),
              ),
            ),
          ),
      if (trackShowsRating(rating))
        (
          kRatingSlotWidth,
          kTrailingPillMinWidth,
          MetaPill(
            key: kTrackRatingPillKey,
            text: '$rating',
            fillWidth: false,
            keepLeadingGlyph: true,
            leading: const MetaPillGlyph(LucideIcons.star),
          ),
        ),
      if (bpm != null)
        (
          kBpmSlotWidth,
          kTrailingPillMinWidth,
          MetaPill(
            text: '${bpm!.toStringAsFixed(1)} BPM',
            fillWidth: false,
            leading: const MetaPillGlyph(LucideIcons.metronome),
          ),
        ),
      if (rawKey.isNotEmpty)
        (
          kKeySlotWidth,
          kTrailingPillMinWidth,
          KeyPill(
            rawKey: rawKey,
            keyDisplayMode: keyDisplayMode,
            keyColorMode: keyColorMode,
          ),
        ),
    ];
    return LayoutBuilder(
      builder: (context, constraints) {
        final budget = constraints.maxWidth;
        final pills = <Widget>[];
        var used = 0.0;
        for (final (slot, minWidth, pill) in candidates) {
          final gap = pills.isEmpty ? 0.0 : kMetaPillGap;
          // A full run leaves no room, and subtracting the gap would go
          // negative; clamp so the arithmetic below stays a width.
          final room = (budget - used - gap).clamp(0.0, double.infinity);
          // The slot is a maximum: a pill narrower than it hugs its content.
          // `room` only bites when the budget cannot seat the whole slot, and
          // under the pill minimum the row hides it rather than clipping it.
          final maxWidth = room < slot ? room : slot;
          if (maxWidth < minWidth) {
            continue;
          }
          used += gap + maxWidth;
          if (pills.isNotEmpty) {
            pills.add(const SizedBox(width: kMetaPillGap));
          }
          pills.add(
            ConstrainedBox(
              constraints: BoxConstraints(maxWidth: maxWidth),
              child: pill,
            ),
          );
        }
        return Row(
          key: kTrailingMetaKey,
          mainAxisSize: MainAxisSize.min,
          children: pills,
        );
      },
    );
  }
}
