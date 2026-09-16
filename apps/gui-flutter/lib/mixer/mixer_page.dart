import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_grid.dart';
import 'package:gui_flutter/mixer/library_panel.dart';
import 'package:gui_flutter/mixer/waveform_section.dart';
import 'package:panes/panes.dart';

/// Mixer page with resizable regions ([MultiPane]).
///
/// Vertical: waveforms | (fixed decks + library). Decks are not a resizable
/// region — only the waveform/library split moves. Sizes are session-local
/// (no [PaneController.save] / [PaneController.load]).
class MixerPage extends StatefulWidget {
  const MixerPage({super.key});

  static const _waveformDefault = 160.0;
  static const _waveformMin = 110.0;
  static const _deckRowHeight = 410.0;

  @override
  State<MixerPage> createState() => _MixerPageState();
}

class _MixerPageState extends State<MixerPage> {
  late final PaneController _controller;

  @override
  void initState() {
    super.initState();
    _controller = PaneController(
      entries: [
        PaneEntry(
          id: 'waveforms',
          initialSize: PaneSize.pixel(MixerPage._waveformDefault),
          minSize: PaneSize.pixel(MixerPage._waveformMin),
        ),
        PaneEntry(id: 'decks_library', initialSize: PaneSize.fraction(1.0)),
      ],
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Invisible chrome; keep a grab hit-area (matches former library divider:none).
    return PaneTheme(
      data: const PaneThemeData(
        resizerColor: Color(0x00000000),
        resizerHoverColor: Color(0x00000000),
        resizerFocusedColor: Color(0x00000000),
        resizerThickness: 0,
        resizerHitTestThickness: 8,
      ),
      child: MultiPane(
        direction: Axis.vertical,
        controller: _controller,
        paneBuilder: (context, id, _) => switch (id) {
          'waveforms' => const WaveformSection(),
          'decks_library' => ColoredBox(
            color: context.theme.colors.card,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SizedBox(
                  height: MixerPage._deckRowHeight,
                  child: const ClipRect(child: DeckGrid()),
                ),
                FDivider(style: .delta(padding: .value(.all(0)))),
                const Expanded(child: LibraryPanel()),
              ],
            ),
          ),
          _ => const SizedBox.shrink(),
        },
      ),
    );
  }
}
