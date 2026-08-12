import 'package:flutter/widgets.dart';
import 'package:gui_flutter/mixer/deck_panel.dart';
import 'package:gui_flutter/mixer/fader_slider.dart';
import 'package:gui_flutter/mixer/mixer_strip.dart';

/// Deck A | Mixer | Deck B.
class DeckGrid extends StatefulWidget {
  const DeckGrid({super.key});

  @override
  State<DeckGrid> createState() => _DeckGridState();
}

class _DeckGridState extends State<DeckGrid> {
  /// At most one tempo master (null = none).
  FaderAccent? _master;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: DeckPanel(
              label: 'Deck A',
              accent: FaderAccent.a,
              isMaster: _master == FaderAccent.a,
              onMasterChanged: (next) => setState(() {
                _master = next ? FaderAccent.a : null;
              }),
            ),
          ),
          const SizedBox(width: 8),
          const MixerStrip(),
          const SizedBox(width: 8),
          Expanded(
            child: DeckPanel(
              label: 'Deck B',
              accent: FaderAccent.b,
              isMaster: _master == FaderAccent.b,
              onMasterChanged: (next) => setState(() {
                _master = next ? FaderAccent.b : null;
              }),
            ),
          ),
        ],
      ),
    );
  }
}
