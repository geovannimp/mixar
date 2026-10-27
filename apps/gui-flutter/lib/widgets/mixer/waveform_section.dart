import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:gui_flutter/providers/mixer/engine_providers.dart';
import 'package:gui_flutter/providers/settings/settings_providers.dart';
import 'package:gui_flutter/widgets/mixer/waveform/scrolling_lane.dart';
import 'package:gui_flutter/widgets/shell/m_divider.dart';
import 'package:gui_flutter/widgets/shell/mixar_theme.dart';

/// Scrolling waveform lanes for every active deck.
class WaveformSection extends ConsumerWidget {
  const new({super.key});

  static const _labels = ['A', 'B', 'C', 'D'];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = context.theme;
    final numDecks = ref.watch(appSettingsProvider).value?.numDecks ?? 2;
    final hasTrack = [
      for (var deckId = 0; deckId < numDecks; deckId++)
        ref.watch(deckTrackIdProvider(deckId)) != null,
    ];

    return ColoredBox(
      color: theme.colors.background,
      child: Column(
        children: [
          Expanded(
            child: Stack(
              children: [
                Column(
                  children: [
                    for (var deckId = 0; deckId < numDecks; deckId++) ...[
                      Expanded(
                        child: RepaintBoundary(
                          child: ScrollingLane(
                            deckId: deckId,
                            label: _labels[deckId],
                          ),
                        ),
                      ),
                      if (deckId < numDecks - 1) const MDivider(padding: .zero),
                    ],
                  ],
                ),
                if (!hasTrack.any((loaded) => loaded))
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
          ),
          // Hairline against the deck row, in the same border colour as the
          // other pane splits.
          MDivider(
            key: const ValueKey('waveform-foot'),
            padding: .zero,
            color: theme.colors.border,
          ),
        ],
      ),
    );
  }
}
