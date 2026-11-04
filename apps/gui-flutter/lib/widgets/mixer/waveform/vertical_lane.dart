import 'package:flutter/widgets.dart';
import 'package:gui_flutter/core/theme/app_typography.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/widgets/mixer/waveform/scrolling_lane.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// One deck's scrolling waveform turned on its side.
///
/// [RotatedBox] swaps the lane's constraints, so the lane lays the track out
/// along the column's height and paints one lane-width thick. Quarter turns run
/// the strip bottom-to-top: upcoming audio sits above the playhead, like
/// rekordbox and Serato's vertical view. Pointer scrubbing rotates with the
/// content, so dragging up still moves time forward.
///
/// The deck letter is painted outside the rotation, at the lane's foot — the
/// strip's time start — so a column of lanes still says which deck is which.
class VerticalWaveformLane extends StatelessWidget {
  const new({required this.deckId, super.key});

  final int deckId;

  static const _labels = ['A', 'B', 'C', 'D'];

  @override
  Widget build(BuildContext context) {
    final theme = context.theme;
    return Stack(
      fit: StackFit.expand,
      children: [
        RotatedBox(quarterTurns: 3, child: ScrollingLane(deckId: deckId)),
        Align(
          alignment: Alignment.bottomLeft,
          child: Padding(
            padding: const EdgeInsets.only(left: 8, bottom: 8),
            child: Text(
              _labels[deckId],
              key: ValueKey('wave-lane-label-$deckId'),
              style: theme.typography.display.xs.copyWith(
                fontFamily: MixarFonts.spaceGrotesk,
                color: deckInk(
                  deckAccentFor(deckId) ?? DeckAccent.neutral,
                  theme.colors.brightness,
                ).withValues(alpha: 0.55),
                fontWeight: FontWeight.w700,
                letterSpacing: 0.6,
                height: 1,
              ),
            ),
          ),
        ),
      ],
    );
  }
}
