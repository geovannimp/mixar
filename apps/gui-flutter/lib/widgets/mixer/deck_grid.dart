import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/core/theme/deck_accent.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Deck panels around the shared mixer; four-deck mode stacks A/C and B/D.
class DeckGrid extends ConsumerWidget {
  const new({super.key});

  static const _labels = ['Deck A', 'Deck B', 'Deck C', 'Deck D'];

  /// Deck body inset; the stacked split stops at the panel edges, not the
  /// column edges, so it must use the same value.
  static const _inset = 10.0;

  // Horizontal inset only: the tempo rail must reach the waveform pane above
  // and the library divider below, so the vertical inset moves into DeckPanel.
  Widget _panel(int deckId) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: _inset),
    child: RepaintBoundary(
      key: ValueKey('deck-$deckId'),
      child: DeckPanel(
        deckId: deckId,
        label: _labels[deckId],
        accent: deckAccentFor(deckId)!,
      ),
    ),
  );

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final numDecks = ref.watch(appSettingsProvider).value?.numDecks ?? 2;
    if (numDecks == 2) {
      return Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 8,
        children: [
          Expanded(child: _panel(0)),
          const RepaintBoundary(child: MixerStrip()),
          Expanded(child: _panel(1)),
        ],
      );
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 8,
      children: [
        Expanded(child: _stacked(context, 0, 2)),
        const RepaintBoundary(child: MixerStrip()),
        Expanded(child: _stacked(context, 1, 3)),
      ],
    );
  }

  /// Two decks stacked in one column, split by a hairline inset to the deck
  /// edges. The top deck gives up the odd pixel so the split starts on a whole
  /// device pixel instead of antialiasing across two rows, including at
  /// fractional display scaling.
  Widget _stacked(BuildContext context, int topId, int bottomId) {
    final theme = context.theme;
    final dpr = MediaQuery.maybeOf(context)?.devicePixelRatio ?? 1;
    return LayoutBuilder(
      builder: (_, constraints) {
        final half = (constraints.maxHeight - theme.style.borderWidth) / 2;
        final height = math.max(0.0, (half * dpr).floorToDouble() / dpr);
        return Column(
          children: [
            SizedBox(height: height, child: _panel(topId)),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: _inset),
              child: MDivider(
                key: ValueKey('deck-split-$topId'),
                padding: .zero,
                color: theme.colors.border,
              ),
            ),
            Expanded(child: _panel(bottomId)),
          ],
        );
      },
    );
  }
}
