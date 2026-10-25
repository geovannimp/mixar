import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/deck_panel.dart';
import 'package:gui_flutter/widgets/mixer/fader_slider.dart';
import 'package:gui_flutter/widgets/mixer/mixer_strip.dart';

/// Deck panels around the shared mixer; four-deck mode stacks A/C and B/D.
class DeckGrid extends ConsumerWidget {
  const new({super.key});

  static const _labels = ['Deck A', 'Deck B', 'Deck C', 'Deck D'];

  Widget _panel(int deckId) => Padding(
    padding: const EdgeInsets.all(10),
    child: RepaintBoundary(
      key: ValueKey('deck-$deckId'),
      child: DeckPanel(
        deckId: deckId,
        label: _labels[deckId],
        accent: faderAccentForDeck(deckId)!,
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
        Expanded(
          child: Column(
            spacing: 4,
            children: [
              Expanded(child: _panel(0)),
              Expanded(child: _panel(2)),
            ],
          ),
        ),
        const RepaintBoundary(child: MixerStrip()),
        Expanded(
          child: Column(
            spacing: 4,
            children: [
              Expanded(child: _panel(1)),
              Expanded(child: _panel(3)),
            ],
          ),
        ),
      ],
    );
  }
}
