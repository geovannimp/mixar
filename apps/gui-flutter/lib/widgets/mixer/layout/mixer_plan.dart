import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart' show Axis, EdgeInsets;
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_content.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_config.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_layout_solver.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';

/// One slot's content, with the floor the solver handed it.
@immutable
class MixerSlotSpec {
  const new({required this.content, required this.minPx});

  final MixerContent content;

  /// Fewest pixels the slot is worth; the page puts it on the pane's minimum.
  final double minPx;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MixerSlotSpec &&
          other.content == content &&
          other.minPx == minPx;

  @override
  int get hashCode => Object.hash(content, minPx);
}

/// The content row: the decks either side of a centre column, or the decks
/// meeting on a seam where no centre column is shown.
@immutable
class MixerContentRowPlan {
  const new({
    required this.left,
    required this.center,
    required this.right,
    required this.centerWidth,
    required this.seam,
  });

  final MixerSlotSpec? left;
  final MixerSlotSpec? center;
  final MixerSlotSpec? right;

  /// Centre column thickness; the pane is pinned to it.
  final double centerWidth;

  /// Centre hidden: draw a rule between left and right.
  final bool seam;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MixerContentRowPlan &&
          other.left == left &&
          other.center == center &&
          other.right == right &&
          other.centerWidth == centerWidth &&
          other.seam == seam;

  @override
  int get hashCode => Object.hash(left, center, right, centerWidth, seam);
}

/// What each slot holds, for one solved layout.
@immutable
class MixerPlan {
  const new({
    required this.top,
    required this.content,
    required this.bottom,
    required this.bar,
  });

  final MixerSlotSpec? top;
  final MixerContentRowPlan? content;
  final MixerSlotSpec? bottom;
  final MixerSlotSpec? bar;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MixerPlan &&
          other.top == top &&
          other.content == content &&
          other.bottom == bottom &&
          other.bar == bar;

  @override
  int get hashCode => Object.hash(top, content, bottom, bar);
}

/// Turns settings plus a solved split into the two stacks' contents.
MixerPlan mixerPlanFor(MixerLayoutConfig config, MixerLayoutSolution solution) {
  final docked = config.preferDockedLibrary && solution.libraryDocked;
  final lanes =
      config.showWaveform &&
      (config.kind == MixerLayoutKind.centreHorizontal ||
          config.kind == MixerLayoutKind.centreVertical);
  final mixerCentre = !lanes && config.showMixer;
  // The centre column is the mixer, so the decks keep their gutter against it;
  // a waveform centre hands that gutter to the lanes.
  final innerInset = mixerCentre;
  const inset = MixerPaneMetrics.deckInset;

  final top = config.kind == MixerLayoutKind.waveformTop && config.showWaveform
      ? const MixerSlotSpec(
          content: WaveformContent(),
          minPx: MixerPaneMetrics.bandMin,
        )
      : null;

  final leftPadding = EdgeInsets.only(
    left: inset,
    right: innerInset ? inset : 0,
  );
  final rightPadding = EdgeInsets.only(
    left: innerInset ? inset : 0,
    right: inset,
  );
  final left = config.fourDecks
      ? MixerSlotSpec(
          content: DeckPairContent(
            0,
            2,
            padding: leftPadding,
            dividerPadding: EdgeInsets.only(right: innerInset ? inset : 0),
          ),
          minPx: 0,
        )
      : MixerSlotSpec(content: DeckContent(0, padding: leftPadding), minPx: 0);
  final right = config.fourDecks
      ? MixerSlotSpec(
          content: DeckPairContent(
            1,
            3,
            padding: rightPadding,
            dividerPadding: EdgeInsets.only(left: innerInset ? inset : 0),
          ),
          minPx: 0,
        )
      : MixerSlotSpec(content: DeckContent(1, padding: rightPadding), minPx: 0);

  final MixerSlotSpec? center;
  final double centerWidth;
  if (mixerCentre) {
    centerWidth = config.mixerColumnWidth;
    center = MixerSlotSpec(content: MixerStripContent(centerWidth), minPx: 0);
  } else if (lanes) {
    center = MixerSlotSpec(
      content: GroupContent(Axis.vertical, [
        MixerGroupChild(
          config.kind == MixerLayoutKind.centreVertical
              ? const VerticalLanesContent()
              : const WaveformContent(),
        ),
        if (config.showCrossfader)
          const MixerGroupChild(CrossfaderContent(), fill: false),
      ]),
      minPx: 0,
    );
    centerWidth = config.laneColumnWidth;
  } else {
    center = null;
    centerWidth = 0;
  }

  final MixerSlotSpec? bottom;
  if (config.kind == MixerLayoutKind.waveformBottom && config.showWaveform) {
    bottom = MixerSlotSpec(
      content: GroupContent(Axis.vertical, [
        const MixerGroupChild(
          WaveformContent(),
          fill: false,
          extent: MixerPaneMetrics.bandDefault,
        ),
        if (docked)
          const MixerGroupChild(
            LibraryContent(),
            min: MixerPaneMetrics.libraryMin,
            dividerBefore: true,
          ),
      ]),
      minPx: MixerPaneMetrics.bandDefault,
    );
  } else if (docked) {
    bottom = const MixerSlotSpec(
      content: LibraryContent(),
      minPx: MixerPaneMetrics.libraryMin,
    );
  } else {
    bottom = null;
  }

  return MixerPlan(
    top: top,
    content: MixerContentRowPlan(
      left: left,
      center: center,
      right: right,
      centerWidth: centerWidth,
      seam: center == null,
    ),
    bottom: bottom,
    bar: config.showFxBar
        ? const MixerSlotSpec(content: FxBarContent(), minPx: FxBar.height)
        : null,
  );
}
