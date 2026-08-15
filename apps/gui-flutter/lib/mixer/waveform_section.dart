import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/scrolling_lane.dart';

/// Dual scrolling lanes. Overview strips live on each deck panel.
class WaveformSection extends ConsumerWidget {
  const WaveformSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final hasA = ref.watch(deckTrackIdProvider(0)) != null;
    final hasB = ref.watch(deckTrackIdProvider(1)) != null;

    return ColoredBox(
      color: theme.colors.background,
      child: Stack(
        children: [
          Column(
            children: [
              const Expanded(child: ScrollingLane(deckId: 0, label: 'Deck A')),
              const FDivider(style: .delta(padding: .value(.zero))),
              const Expanded(child: ScrollingLane(deckId: 1, label: 'Deck B')),
            ],
          ),
          if (!hasA && !hasB)
            Center(
              child: Text(
                'Load tracks to see waveforms.',
                style: theme.typography.body.sm.copyWith(
                  color: theme.colors.mutedForeground,
                ),
              ),
            ),
        ],
      ),
    );
  }
}
