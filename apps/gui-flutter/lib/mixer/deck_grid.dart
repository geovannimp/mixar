import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/deck_panel.dart';
import 'package:gui_flutter/mixer/mixer_strip.dart';

/// Deck A | Mixer | Deck B.
class DeckGrid extends StatelessWidget {
  const DeckGrid({super.key});

  @override
  Widget build(BuildContext context) {
    return const Padding(
      padding: EdgeInsets.all(8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(child: DeckPanel(label: 'Deck A')),
          SizedBox(width: 8),
          MixerStrip(),
          SizedBox(width: 8),
          Expanded(child: DeckPanel(label: 'Deck B')),
        ],
      ),
    );
  }
}
