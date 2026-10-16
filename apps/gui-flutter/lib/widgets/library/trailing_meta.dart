import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/utils/key_format.dart';
import 'package:gui_flutter/core/utils/library_list_layout.dart';
import 'package:gui_flutter/widgets/library/key_pill.dart';
import 'package:gui_flutter/widgets/library/meta_pill.dart';
import 'package:gui_flutter/widgets/library/meta_pill_glyph.dart';
import 'package:lucide_icons_flutter/lucide_icons.dart';

/// BPM + key (and optional duration) for the right side of a list row.
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
