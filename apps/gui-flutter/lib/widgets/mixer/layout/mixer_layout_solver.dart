import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_config.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';

/// The split the panes are asked to hold: fractions of the space left once the
/// FX bar has taken its pixels, plus the decisions that came with them.
@immutable
class MixerLayoutSolution {
  const new({
    required this.topFraction,
    required this.contentFraction,
    required this.bottomFraction,
    required this.libraryDocked,
    required this.barVisible,
  });

  /// Waveform band; null hides the slot.
  final double? topFraction;

  /// Deck row. Always positive: the band and the library yield to it.
  final double contentFraction;

  /// Library, and the bottom waveform band when one is placed there; null
  /// hides the slot.
  final double? bottomFraction;

  /// Whether the library is docked in the bottom slot rather than the drawer.
  final bool libraryDocked;

  /// Whether the FX bar rides at the foot.
  final bool barVisible;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MixerLayoutSolution &&
          other.topFraction == topFraction &&
          other.contentFraction == contentFraction &&
          other.bottomFraction == bottomFraction &&
          other.libraryDocked == libraryDocked &&
          other.barVisible == barVisible;

  @override
  int get hashCode => Object.hash(
    topFraction,
    contentFraction,
    bottomFraction,
    libraryDocked,
    barVisible,
  );
}

/// Splits the body height into the band, the content row, and the bottom slot.
///
/// The body never scrolls, so whatever the window, the three slots must add up
/// to the space left by the FX bar. The content row is the part that must
/// survive: the band yields down to its floor and then hides, the bottom band
/// gives up its place entirely, and the library only docks when at least
/// [MixerPaneMetrics.libraryMin] would be left for it — otherwise it moves to
/// the drawer and the content row takes the remainder.
MixerLayoutSolution solveMixerLayout({
  required double height,
  required MixerLayoutConfig config,
  required double bandFraction,
}) {
  final barPx = config.showFxBar ? FxBar.height : 0.0;
  final avail = height > barPx ? height - barPx : 0.0;
  if (avail == 0) {
    return const MixerLayoutSolution(
      topFraction: null,
      contentFraction: 1,
      bottomFraction: null,
      libraryDocked: false,
      barVisible: false,
    );
  }

  final bandShown =
      config.kind == MixerLayoutKind.waveformTop && config.showWaveform;
  var bandPx = bandShown
      ? math.max(MixerPaneMetrics.bandMin, bandFraction * avail)
      : 0.0;
  var bottomFixed =
      config.kind == MixerLayoutKind.waveformBottom && config.showWaveform
      ? MixerPaneMetrics.bandDefault
      : 0.0;

  final pref = config.fourDecks
      ? MixerPaneMetrics.contentPreferredFour
      : MixerPaneMetrics.contentPreferredTwo;
  final floor = config.fourDecks
      ? MixerPaneMetrics.contentFloorFour
      : MixerPaneMetrics.contentFloorTwo;

  final space = avail - bandPx - bottomFixed;
  final docked =
      config.preferDockedLibrary && space - pref >= MixerPaneMetrics.libraryMin;
  var contentPx = docked ? pref : space;
  final libraryPx = docked ? space - pref : 0.0;

  if (!docked && contentPx < floor) {
    // The band yields down to its floor, then hides.
    if (bandPx > 0) {
      bandPx = math.max(0, bandPx - (floor - contentPx));
      if (bandPx < MixerPaneMetrics.bandMin) {
        bandPx = 0;
      }
    }
    contentPx = math.max(0, avail - bandPx - bottomFixed);
  }
  if (!docked && contentPx < floor && bottomFixed > 0) {
    // The bottom band yields entirely before the content row would clip.
    bottomFixed = 0;
    contentPx = math.max(0, avail - bandPx);
  }

  final bottomPx = bottomFixed + libraryPx;
  return MixerLayoutSolution(
    topFraction: bandPx > 0 ? bandPx / avail : null,
    contentFraction: contentPx / avail,
    bottomFraction: bottomPx > 0 ? bottomPx / avail : null,
    libraryDocked: docked,
    barVisible: config.showFxBar,
  );
}
