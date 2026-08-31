import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_grid.dart';
import 'package:gui_flutter/mixer/library_panel.dart';
import 'package:gui_flutter/mixer/waveform_section.dart';

/// Mixer page with Tauri-like resizable regions (Forui [FResizable]).
///
/// Vertical: waveforms | (fixed decks + library). Decks are not a resizable
/// region — only the waveform/library split moves.
class MixerPage extends StatelessWidget {
  const MixerPage({super.key});

  static const _waveformDefault = 160.0;
  static const _waveformMin = 110.0;
  static const _deckRowHeight = 410.0;

  @override
  Widget build(BuildContext context) {
    return FResizable(
      axis: .vertical,
      divider: .dividerWithThumb,
      children: [
        FResizableRegion.fixed(
          extent: _waveformDefault,
          minExtent: _waveformMin,
          builder: _fill,
          child: const WaveformSection(),
        ),
        FResizableRegion.flex(
          flex: 1,
          minFlex: 1,
          builder: _fill,
          child: ColoredBox(
            color: context.theme.colors.card,
            child: Column(
              crossAxisAlignment: .stretch,
              children: [
                SizedBox(
                  height: _deckRowHeight,
                  child: const ClipRect(child: DeckGrid()),
                ),
                FDivider(style: .delta(padding: .value(.all(0)))),
                Expanded(child: LibraryPanel()),
              ],
            ),
          ),
        ),
      ],
    );
  }

  static Widget _fill(BuildContext _, FResizableRegionData _, Widget? child) =>
      SizedBox.expand(child: child);
}
