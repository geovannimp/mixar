import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:gui_flutter/mixer/engine_providers.dart';
import 'package:gui_flutter/mixer/waveform/overview_strip.dart';
import 'package:gui_flutter/mixer/waveform/scrolling_lane.dart';

/// Dual-lane waveform: overview + scrolling detail per deck.
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
              const OverviewStrip(deckId: 0, height: 22),
              const Expanded(child: ScrollingLane(deckId: 0, label: 'Deck A')),
              FDivider(),
              const OverviewStrip(deckId: 1, height: 22),
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
