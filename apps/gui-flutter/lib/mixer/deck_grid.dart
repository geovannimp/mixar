import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_panel.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/mixer_strip.dart';

/// Deck A | Mixer | Deck B in one card.
class DeckGrid extends StatelessWidget {
  const DeckGrid({super.key});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8),
      child: FCard(
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: const [
            Expanded(
              child: DeckPanel(
                deckId: 0,
                label: 'Deck A',
                accent: FaderAccent.a,
              ),
            ),
            SizedBox(width: 8),
            MixerStrip(),
            SizedBox(width: 8),
            Expanded(
              child: DeckPanel(
                deckId: 1,
                label: 'Deck B',
                accent: FaderAccent.b,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
