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
/// Content-sized: parents pass a `maxWidth` budget via [trailingMetaWidth], but
/// this widget only takes the width of its pills so the cluster sits flush
/// against the row actions. Loose flex still lets labels ellipsize when the
/// budget is tight.
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
    // `mainAxisSize: min` so a parent maxWidth budget does not leave empty
    // space after the key (that read as "not flush right" before the menu).
    // Loose flex + `fillWidth: false` keeps pills packed with only
    // `kMetaPillGap` between them and still ellipsizes under pressure.
    final duration = formatTrackDuration(durationMs);
    final children = <Widget>[
      if (duration.isNotEmpty)
        Flexible(
          fit: FlexFit.loose,
          flex: kDurationSlotFlex,
          child: MetaPill(
            text: duration,
            fillWidth: false,
            leading: const MetaPillGlyph(LucideIcons.clock),
          ),
        ),
      if (trackShowsColorLabel(colorLabel))
        if (colorLabelColor(colorLabel) case final color?)
          Flexible(
            fit: FlexFit.loose,
            child: Semantics(
              label: 'Colour ${colorLabelDisplayName(colorLabel)}',
              child: Container(
                key: kTrackColorPillKey,
                width: 16,
                height: 16,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: color,
                  border: Border.all(
                    color: context.theme.colors.border,
                    width: context.theme.style.borderWidth,
                  ),
                ),
              ),
            ),
          ),
      if (trackShowsRating(rating))
        Flexible(
          fit: FlexFit.loose,
          child: MetaPill(
            key: kTrackRatingPillKey,
            text: '$rating',
            fillWidth: false,
            keepLeadingGlyph: true,
            leading: const MetaPillGlyph(LucideIcons.star),
          ),
        ),
      if (bpm != null)
        Flexible(
          fit: FlexFit.loose,
          flex: kBpmSlotFlex,
          child: MetaPill(
            text: '${bpm!.toStringAsFixed(1)} BPM',
            fillWidth: false,
            leading: const MetaPillGlyph(LucideIcons.metronome),
          ),
        ),
      if (rawKey.isNotEmpty)
        Flexible(
          fit: FlexFit.loose,
          flex: kKeySlotFlex,
          child: KeyPill(
            rawKey: rawKey,
            keyDisplayMode: keyDisplayMode,
            keyColorMode: keyColorMode,
          ),
        ),
    ];
    return Row(
      key: kTrailingMetaKey,
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var i = 0; i < children.length; i++) ...[
          if (i > 0) const SizedBox(width: kMetaPillGap),
          children[i],
        ],
      ],
    );
  }
}
