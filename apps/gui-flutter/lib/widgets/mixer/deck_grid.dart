import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/src/rust/api/settings.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';
import 'package:gui_flutter/widgets/mixer/waveform/vertical_lane.dart';
import 'package:gui_flutter/widgets/mixer/waveform_section.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Deck panels around the shared mixer; four-deck mode stacks A/C and B/D.
///
/// The waveform's two middle positions replace the mixer column with the lanes
/// themselves: horizontal lanes stacked, or one vertical lane per deck side by
/// side, the way rekordbox centers its waveforms.
class DeckGrid extends ConsumerWidget {
  const new({super.key});

  static const _labels = ['Deck A', 'Deck B', 'Deck C', 'Deck D'];

  /// Deck body inset; the stacked split stops at the panel edges, not the
  /// column edges, so it must use the same value.
  static const inset = 10.0;

  /// Gap between the mixer column and each deck column.
  static const rowSpacing = 8.0;

  /// Mixer column; the strip bleeds these gutters so its rule reaches the deck
  /// card borders.
  static const _mixer = RepaintBoundary(
    child: MixerStrip(bleed: rowSpacing + inset),
  );

  /// Center column thickness when the waveform takes the mixer's place. Four
  /// decks keep the mixer-wide column so the stacked lanes and the deck pads
  /// both get room.
  static const _centerWidth = 320.0;
  static const _centerWidthFour = 388.0;

  // Horizontal inset only: the tempo rail must reach the waveform pane above
  // and the library divider below, so the vertical inset moves into DeckPanel.
  //
  // The edge facing the waveform drops its inset when the waveform takes the
  // mixer's column: those gutters belong to the mixer, so without it the lanes
  // run edge to edge against the deck cards with full hit testing.
  Widget _panel(int deckId, {bool leftInset = true, bool rightInset = true}) =>
      Padding(
        padding: EdgeInsets.only(
          left: leftInset ? inset : 0,
          right: rightInset ? inset : 0,
        ),
        child: RepaintBoundary(
          key: ValueKey('deck-$deckId'),
          child: DeckPanel(
            deckId: deckId,
            label: _labels[deckId],
            accent: deckAccentFor(deckId)!,
          ),
        ),
      );

  Widget _lane(int deckId) => RepaintBoundary(
    key: ValueKey('wave-lane-$deckId'),
    child: VerticalWaveformLane(deckId: deckId),
  );

  /// The center column: the mixer normally, the lanes for the middle positions.
  ///
  /// The middle layouts hide the mixer but keep its foot: the horizontal lanes
  /// stack (or one vertical lane per deck runs side by side) above the same
  /// crossfader panel the mixer column carries under its channels, unless the
  /// crossfader is switched off in Settings → Layout.
  Widget _center(
    BuildContext context,
    WaveformPositionSetting position,
    double width,
    int numDecks, {
    required bool showCrossfader,
  }) {
    final vertical = position == WaveformPositionSetting.middleVertical;
    final content = vertical
        ? Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              for (var deckId = 0; deckId < numDecks; deckId++) ...[
                if (deckId > 0) _seam(context),
                Expanded(child: _lane(deckId)),
              ],
            ],
          )
        : const WaveformSection(showFootDivider: false);
    return SizedBox(
      width: width,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(child: content),
          // The mixer keeps its strip off the deck cards; the center column is
          // flush, so the crossfader re-adds that gutter and the same foot
          // spacing the mixer cell has.
          if (showCrossfader)
            const CrossfaderPanel(
              contentPadding: EdgeInsets.only(
                left: rowSpacing + inset,
                right: rowSpacing + inset,
                bottom: MixerStrip.verticalInset,
              ),
            ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(appSettingsProvider).value;
    final numDecks = settings?.numDecks ?? 2;
    final position = settings?.waveformPosition ?? WaveformPositionSetting.top;
    final showWaveform = settings?.showWaveform ?? true;
    // The strip is the only part of the deck row the Mixer switch removes: the
    // decks always stay, and a middle waveform already owns the center column.
    final showMixer = settings?.showMixer ?? true;
    final showCrossfader = settings?.showCrossfader ?? true;
    final middle =
        showWaveform &&
        (position == WaveformPositionSetting.middleHorizontal ||
            position == WaveformPositionSetting.middleVertical);
    final centerWidth = numDecks == 4 ? _centerWidthFour : _centerWidth;
    final center = middle
        ? _center(
            context,
            position,
            centerWidth,
            numDecks,
            showCrossfader: showCrossfader,
          )
        : (showMixer ? _mixer : null);

    // Middle layouts hand the mixer's gutters to the waveform: no row gap and
    // no deck inset on the edge that faces the lanes. An empty centre column
    // gets the same treatment, so the decks meet on a single hairline instead
    // of leaving the mixer's gutter behind as dead space.
    final closed = center == null;
    final spacing = middle || closed ? 0.0 : rowSpacing;
    final innerInset = !middle && !closed;
    final seam = closed ? _seam(context) : null;

    if (numDecks == 2) {
      return Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: spacing,
        children: [
          Expanded(child: _panel(0, rightInset: innerInset)),
          ?seam,
          ?center,
          Expanded(child: _panel(1, leftInset: innerInset)),
        ],
      );
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: spacing,
      children: [
        Expanded(
          child: _stacked(
            context,
            0,
            2,
            leftInset: true,
            rightInset: innerInset,
            // The window edge side has no inset: the hairline reaches the
            // border instead of stopping where the deck card does.
            dividerPadding: EdgeInsets.only(right: innerInset ? inset : 0),
          ),
        ),
        ?seam,
        ?center,
        Expanded(
          child: _stacked(
            context,
            1,
            3,
            leftInset: innerInset,
            rightInset: true,
            dividerPadding: EdgeInsets.only(left: innerInset ? inset : 0),
          ),
        ),
      ],
    );
  }

  /// Hairline between the deck columns when no centre column sits between them.
  Widget _seam(BuildContext context) => SizedBox(
    width: context.theme.style.borderWidth,
    child: ColoredBox(color: context.theme.colors.border),
  );

  Widget _stacked(
    BuildContext context,
    int topId,
    int bottomId, {
    required bool leftInset,
    required bool rightInset,
    required EdgeInsets dividerPadding,
  }) => _split(
    context,
    topId,
    bottomId,
    keyPrefix: 'deck-split',
    dividerPadding: dividerPadding,
    build: (deckId) =>
        _panel(deckId, leftInset: leftInset, rightInset: rightInset),
  );

  /// Two panes in one column, split by a hairline that spans the panes edge to
  /// edge. The top pane gives up the odd pixel so the split starts on a whole
  /// device pixel instead of antialiasing across two rows, including at
  /// fractional display scaling.
  ///
  /// The divider follows the panes' insets except at the window edge, where the
  /// caller passes no padding so the rule runs to the border.
  Widget _split(
    BuildContext context,
    int topId,
    int bottomId, {
    required String keyPrefix,
    required EdgeInsets dividerPadding,
    required Widget Function(int) build,
  }) {
    final theme = context.theme;
    final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;
    return LayoutBuilder(
      builder: (_, constraints) {
        final half = (constraints.maxHeight - theme.style.borderWidth) / 2;
        final height = math.max(0.0, (half * dpr).floorToDouble() / dpr);
        return Column(
          children: [
            SizedBox(height: height, child: build(topId)),
            Padding(
              padding: dividerPadding,
              child: MDivider(
                key: ValueKey('$keyPrefix-$topId'),
                padding: .zero,
                color: theme.colors.border,
              ),
            ),
            Expanded(child: build(bottomId)),
          ],
        );
      },
    );
  }
}
