import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/fx_bar.dart';
import 'package:gui_flutter/widgets/mixer/layout/mixer_slot.dart';
import 'package:gui_flutter/widgets/mixer/library_panel.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';
import 'package:gui_flutter/widgets/mixer/waveform/vertical_lane.dart';
import 'package:gui_flutter/widgets/mixer/waveform_section.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// What a slot holds. A slot is a rectangle; the content is whatever the plan
/// puts in it, so one pane stack serves every waveform placement.
sealed class MixerContent {
  const new();
}

/// The scrolling waveform band: one lane per deck, stacked.
final class WaveformContent extends MixerContent {
  const new();
}

/// The middle-vertical placement: one lane per deck, side by side.
final class VerticalLanesContent extends MixerContent {
  const new();
}

/// One deck card, inset where it faces a neighbour rather than the window.
final class DeckContent extends MixerContent {
  const new(this.deckId, {this.padding = EdgeInsets.zero});

  final int deckId;
  final EdgeInsets padding;
}

/// Two deck cards stacked in one column, split by a hairline that spans the
/// panes edge to edge.
final class DeckPairContent extends MixerContent {
  const new(this.topId, this.bottomId, {required this.padding});

  final int topId;
  final int bottomId;
  final EdgeInsets padding;
}

/// The channel strip between the decks.
///
/// Carries its own width: the strip's columns are fixed-width, so the slot has
/// to be at least that wide or the row would overflow. The pane is pinned to
/// the same width; if the two ever disagree for a frame — the settings the
/// width comes from resolve after the first build, and a deck-count switch
/// moves the pane a frame later — the strip keeps its own width, centred in the
/// pane, and the pane's viewport clips it instead of the layout failing.
final class MixerStripContent extends MixerContent {
  const new(this.width);

  final double width;
}

/// The crossfader, under a waveform centre column in place of the strip.
final class CrossfaderContent extends MixerContent {
  const new();
}

/// The docked library.
final class LibraryContent extends MixerContent {
  const new();
}

/// The FX bar at the foot of the page.
final class FxBarContent extends MixerContent {
  const new();
}

/// Several contents sharing one slot along [axis].
final class GroupContent extends MixerContent {
  const new(this.axis, this.children);

  final Axis axis;
  final List<MixerGroupChild> children;
}

/// One member of a [GroupContent]: filling the leftover space, or holding a
/// budgeted extent of its own.
final class MixerGroupChild {
  const new(
    this.content, {
    this.fill = true,
    this.extent,
    this.min,
    this.dividerBefore = false,
  });

  final MixerContent content;

  /// Takes the group's leftover space.
  final bool fill;

  /// Budgeted px for a non-fill child (the waveform band's 160).
  final double? extent;

  /// Solver-only floor for a fill child; the widget ignores it.
  final double? min;

  /// Draw a rule before this child.
  final bool dividerBefore;
}

/// Builds the widget for [content]; `null` content is an empty slot.
Widget buildMixerContent(BuildContext context, MixerContent? content) =>
    switch (content) {
      null => const SizedBox.shrink(),
      WaveformContent() => const WaveformSection(),
      VerticalLanesContent() => _lanes(context),
      DeckContent(:final deckId, :final padding) => Padding(
        padding: padding,
        child: _deck(deckId),
      ),
      DeckPairContent(:final topId, :final bottomId, :final padding) => _split(
        context,
        topId,
        bottomId,
        padding: padding,
      ),
      MixerStripContent(:final width) => OverflowBox(
        alignment: Alignment.center,
        minWidth: width,
        maxWidth: double.infinity,
        child: const RepaintBoundary(child: MixerStrip()),
      ),
      CrossfaderContent() => const CrossfaderPanel(
        edgeFill: MixerPaneMetrics.mixerBleed,
      ),
      LibraryContent() => const LibraryPanel(),
      FxBarContent() => const FxBar(),
      GroupContent(:final axis, :final children) => _group(
        context,
        axis,
        children,
      ),
    };

Widget _deck(int deckId) => RepaintBoundary(
  key: ValueKey('deck-$deckId'),
  child: DeckPanel(
    deckId: deckId,
    label: deckDisplayLabel(deckId),
    accent: deckAccentFor(deckId)!,
  ),
);

/// One vertical lane per deck, side by side, split by hairlines.
Widget _lanes(BuildContext context) {
  final theme = context.theme;
  final settings = ProviderScope.containerOf(context)
      .read(appSettingsProvider)
      .value;
  final numDecks = settings?.numDecks ?? 2;
  return Row(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      for (var deckId = 0; deckId < numDecks; deckId++) ...[
        if (deckId > 0)
          MDivider(
            axis: Axis.vertical,
            padding: EdgeInsets.zero,
            color: theme.colors.border,
          ),
        Expanded(
          child: RepaintBoundary(
            key: ValueKey('wave-lane-$deckId'),
            child: VerticalWaveformLane(deckId: deckId),
          ),
        ),
      ],
    ],
  );
}

/// Two panes in one column, split by a hairline that spans the panes edge to
/// edge. The top pane gives up the odd pixel so the split starts on a whole
/// device pixel instead of antialiasing across two rows, including at
/// fractional display scaling.
Widget _split(
  BuildContext context,
  int topId,
  int bottomId, {
  required EdgeInsets padding,
}) {
  final theme = context.theme;
  final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;
  Widget pane(int deckId) => Padding(padding: padding, child: _deck(deckId));
  return LayoutBuilder(
    builder: (_, constraints) {
      final half = (constraints.maxHeight - theme.style.borderWidth) / 2;
      final floored = (half * dpr).floorToDouble() / dpr;
      return Column(
        children: [
          SizedBox(
            height: floored.isNegative ? 0 : floored,
            child: pane(topId),
          ),
          MDivider(
            key: ValueKey('deck-split-$topId'),
            padding: EdgeInsets.zero,
            color: theme.colors.border,
          ),
          Expanded(child: pane(bottomId)),
        ],
      );
    },
  );
}

Widget _group(BuildContext context, Axis axis, List<MixerGroupChild> children) {
  final horizontal = axis == Axis.horizontal;
  return Flex(
    direction: axis,
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      for (final child in children) ...[
        if (child.dividerBefore)
          MDivider(
            axis: horizontal ? Axis.vertical : Axis.horizontal,
            padding: EdgeInsets.zero,
            color: context.theme.colors.border,
          ),
        if (child.fill)
          Expanded(child: buildMixerContent(context, child.content))
        else if (child.extent case final extent?)
          SizedBox(
            height: horizontal ? null : extent,
            width: horizontal ? extent : null,
            child: buildMixerContent(context, child.content),
          )
        else
          buildMixerContent(context, child.content),
      ],
    ],
  );
}
