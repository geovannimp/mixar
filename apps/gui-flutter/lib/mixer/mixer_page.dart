import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/deck_grid.dart';
import 'package:gui_flutter/mixer/library_panel.dart';
import 'package:gui_flutter/mixer/waveform_section.dart';

/// Mixer page with Tauri-like resizable regions (Forui [FResizable]).
///
/// Vertical: waveforms | (decks + library)
/// Nested vertical: decks (fixed) | library
class MixerPage extends StatelessWidget {
  const MixerPage({super.key});

  static const _waveformDefault = 112.0;
  static const _waveformMin = 70.0;
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
          child: FResizable(
            axis: .vertical,
            divider: .dividerWithThumb,
            children: [
              FResizableRegion.fixed(
                extent: _deckRowHeight,
                minExtent: 300,
                builder: _fill,
                child: const DeckGrid(),
              ),
              FResizableRegion.flex(
                flex: 1,
                minFlex: 1,
                builder: _fill,
                child: const LibraryPanel(),
              ),
            ],
          ),
        ),
      ],
    );
  }

  static Widget _fill(BuildContext _, FResizableRegionData _, Widget? child) =>
      child!;
}
